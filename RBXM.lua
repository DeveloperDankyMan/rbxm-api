--!strict
-- RBXI client for rbxm-api.
--
-- Production API contract:
--   POST /v1/rbxm/encode: RBXI bytes -> RBXM bytes
--   POST /v1/rbxm/decode: RBXM bytes -> RBXI bytes
--
-- HttpService transports the binary payload as a Lua string. Do not base64-encode
-- production requests; the Rust API expects Content-Type application/octet-stream.

local HttpService = game:GetService("HttpService")

local RBXM = {}

RBXM.VERSION = 1
RBXM.MAGIC = "RBXI"

export type Property = {
\tvalueType: string,
\tvalue: any,
}

export type Instance = {
\treferent: string,
\tclassName: string,
\tname: string,
\tparent: string?,
\tproperties: {[string]: Property},
}

export type RequestOptions = {
\tbaseUrl: string,
\ttimeout: number?,
}

local function appendU8(output: {string}, value: number)
\ttable.insert(output, string.char(value % 256))
end

local function appendU32(output: {string}, value: number)
\t-- RBXI integers are unsigned little-endian values.
\tlocal b1 = value % 256
\tlocal b2 = math.floor(value / 256) % 256
\tlocal b3 = math.floor(value / 65536) % 256
\tlocal b4 = math.floor(value / 16777216) % 256
\ttable.insert(output, string.char(b1, b2, b3, b4))
end

local function appendI32(output: {string}, value: number)
\tif value < 0 then
\t\tvalue = value + 4294967296
\tend
\tappendU32(output, value)
end

local function appendU64(output: {string}, value: number)
\t-- Roblox numbers cannot exactly represent every u64. Int64 properties should
\t-- therefore stay within the safe integer range on the Lua side.
\tif value < 0 then
\t\tvalue = value + 18446744073709551616
\tend
\tfor _ = 1, 8 do
\t\tlocal byte = value % 256
\t\tappendU8(output, byte)
\t\tvalue = math.floor(value / 256)
\tend
end

local function appendF32(output: {string}, value: number)
\tlocal packed = string.pack("<f", value)
\ttable.insert(output, packed)
end

local function appendF64(output: {string}, value: number)
\ttable.insert(output, string.pack("<d", value))
end

local function appendString(output: {string}, value: string)
\tassert(#value <= 4294967295, "RBXI string is too long")
\tappendU32(output, #value)
\ttable.insert(output, value)
end

local function readU8(data: string, position: number): (number, number)
\tassert(position <= #data, "unexpected end of RBXI packet")
\treturn string.byte(data, position), position + 1
end

local function readU32(data: string, position: number): (number, number)
\tassert(position + 3 <= #data, "unexpected end of RBXI packet")
\tlocal a, b, c, d = string.byte(data, position, position + 3)
\treturn a + b * 256 + c * 65536 + d * 16777216, position + 4
end

local function readString(data: string, position: number): (string, number)
\tlocal length
\tlength, position = readU32(data, position)
\tassert(length <= 1048576, "RBXI string is too long")
\tassert(position + length - 1 <= #data, "unexpected end of RBXI packet")
\treturn string.sub(data, position, position + length - 1), position + length
end

local function appendProperty(output: {string}, property: Property)
\tlocal kind = property.valueType
\tlocal value = property.value
\tappendString(output, kind)

\tif kind == "String" or kind == "Ref" then
\t\tassert(type(value) == "string", kind .. " property requires a string")
\t\tappendString(output, value)
\telseif kind == "Bool" then
\t\tassert(type(value) == "boolean", "Bool property requires a boolean")
\t\tappendU8(output, if value then 1 else 0)
\telseif kind == "Int32" then
\t\tassert(type(value) == "number", "Int32 property requires a number")
\t\tappendI32(output, value)
\telseif kind == "Int64" then
\t\tassert(type(value) == "number", "Int64 property requires a number")
\t\tappendU64(output, value)
\telseif kind == "Float32" then
\t\tassert(type(value) == "number", "Float32 property requires a number")
\t\tappendF32(output, value)
\telseif kind == "Float64" then
\t\tassert(type(value) == "number", "Float64 property requires a number")
\t\tappendF64(output, value)
\telseif kind == "Vector2" or kind == "Vector3" or kind == "Color3" then
\t\tassert(type(value) == "table", kind .. " property requires an array")
\t\tlocal count = if kind == "Vector2" then 2 else 3
\t\tassert(#value == count, kind .. " property has the wrong component count")
\t\tfor index = 1, count do
\t\t\tassert(type(value[index]) == "number", kind .. " components must be numbers")
\t\t\tappendF32(output, value[index])
\t\tend
\telse
\t\terror("unsupported RBXI property type: " .. kind)
\tend
end

function RBXM.encodePacket(instances: {Instance}): string
\tassert(#instances <= 100000, "too many RBXI instances")
\tlocal output = {RBXM.MAGIC}
\tappendU8(output, RBXM.VERSION)
\tappendU32(output, #instances)

\tfor _, instance in ipairs(instances) do
\t\tassert(type(instance.referent) == "string", "instance referent is required")
\t\tassert(type(instance.className) == "string", "instance className is required")
\t\tassert(type(instance.name) == "string", "instance name is required")
\t\tappendString(output, instance.referent)
\t\tappendString(output, instance.parent or "")
\t\tappendString(output, instance.className)
\t\tappendString(output, instance.name)

\t\tlocal properties = instance.properties or {}
\t\tlocal propertyNames = {}
\t\tfor propertyName in pairs(properties) do
\t\t\ttable.insert(propertyNames, propertyName)
\t\tend
\t\ttable.sort(propertyNames)
\t\tappendU32(output, #propertyNames)
\t\tfor _, propertyName in ipairs(propertyNames) do
\t\t\tappendString(output, propertyName)
\t\t\tappendProperty(output, properties[propertyName])
\t\tend
\tend

\treturn table.concat(output)
end

function RBXM.encode(instances: {Instance}, options: RequestOptions): string
\tlocal packet = RBXM.encodePacket(instances)
\tlocal response = HttpService:RequestAsync({
\t\tUrl = options.baseUrl .. "/v1/rbxm/encode",
\t\tMethod = "POST",
\t\tHeaders = { ["Content-Type"] = "application/octet-stream" },
\t\tBody = packet,
\t})
\tassert(response.Success, "RBXM encode failed: " .. tostring(response.StatusCode) .. " " .. response.StatusMessage)
\treturn response.Body
end

function RBXM.decodePacket(data: string): {Instance}
\tassert(string.sub(data, 1, 4) == RBXM.MAGIC, "invalid RBXI magic")
\tassert(string.byte(data, 5) == RBXM.VERSION, "unsupported RBXI version")
\tlocal position = 6
\tlocal count
\tcount, position = readU32(data, position)
\tassert(count <= 100000, "too many RBXI instances")
\tlocal instances = {}

\tfor _ = 1, count do
\t\tlocal referent, parent, className, name
\t\treferent, position = readString(data, position)
\t\tparent, position = readString(data, position)
\t\tclassName, position = readString(data, position)
\t\tname, position = readString(data, position)
\t\tlocal propertyCount
\t\tpropertyCount, position = readU32(data, position)
\t\tassert(propertyCount <= 100000, "too many RBXI properties")
\t\tlocal properties = {}

\t\tfor _ = 1, propertyCount do
\t\t\tlocal propertyName, kind
\t\t\tpropertyName, position = readString(data, position)
\t\t\tkind, position = readString(data, position)
\t\t\tlocal value
\t\t\tif kind == "String" or kind == "Ref" then
\t\t\t\tvalue, position = readString(data, position)
\t\t\telseif kind == "Bool" then
\t\t\t\tlocal byte
\t\t\t\tbyte, position = readU8(data, position)
\t\t\t\tassert(byte <= 1, "invalid Bool value")
\t\t\t\tvalue = byte == 1
\t\t\telseif kind == "Int32" then
\t\t\t\tlocal raw
\t\t\t\traw, position = readU32(data, position)
\t\t\t\tvalue = if raw >= 2147483648 then raw - 4294967296 else raw
\t\t\telseif kind == "Int64" then
\t\t\t\t-- Luau's string.unpack handles little-endian signed 64-bit values.
\t\t\t\tvalue, position = string.unpack("<i8", data, position)
\t\t\telseif kind == "Float32" then
\t\t\t\tvalue, position = string.unpack("<f", data, position)
\t\t\telseif kind == "Float64" then
\t\t\t\tvalue, position = string.unpack("<d", data, position)
\t\t\telseif kind == "Vector2" or kind == "Vector3" or kind == "Color3" then
\t\t\t\tlocal componentCount = if kind == "Vector2" then 2 else 3
\t\t\t\tvalue = {}
\t\t\t\tfor index = 1, componentCount do
\t\t\t\t\tvalue[index], position = string.unpack("<f", data, position)
\t\t\t\tend
\t\t\telse
\t\t\t\terror("unsupported RBXI property type: " .. kind)
\t\t\tend
\t\t\tproperties[propertyName] = { valueType = kind, value = value }
\t\tend
\n\t\ttable.insert(instances, {
\t\t\treferent = referent,
\t\t\tparent = if parent == "" then nil else parent,
\t\t\tclassName = className,
\t\t\tname = name,
\t\t\tproperties = properties,
\t\t})
\tend

\tassert(position == #data + 1, "trailing bytes after RBXI packet")
\treturn instances
end

function RBXM.decode(data: string, options: RequestOptions): {Instance}
\tlocal response = HttpService:RequestAsync({
\t\tUrl = options.baseUrl .. "/v1/rbxm/decode",
\t\tMethod = "POST",
\t\tHeaders = { ["Content-Type"] = "application/octet-stream" },
\t\tBody = data,
\t})
\tassert(response.Success, "RBXM decode failed: " .. tostring(response.StatusCode) .. " " .. response.StatusMessage)
\treturn RBXM.decodePacket(response.Body)
end

return RBXM
