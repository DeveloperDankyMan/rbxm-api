--!strict
-- RBXM Lua client for the rbxm-api binary bridge.
--
-- POST /v1/rbxm/encode: RBXI packet bytes -> RBXM bytes
-- POST /v1/rbxm/decode: RBXM bytes -> RBXI packet bytes

local HttpService = game:GetService("HttpService")

local RBXM = {}
RBXM.VERSION = 1
RBXM.MAGIC = "RBXI"

local function appendU8(output, value)
    table.insert(output, string.char(value % 256))
end

local function appendU32(output, value)
    assert(value >= 0 and value <= 4294967295, "uint32 out of range")
    local b1 = value % 256
    local b2 = math.floor(value / 256) % 256
    local b3 = math.floor(value / 65536) % 256
    local b4 = math.floor(value / 16777216) % 256
    table.insert(output, string.char(b1, b2, b3, b4))
end

local function appendI32(output, value)
    assert(value % 1 == 0 and value >= -2147483648 and value <= 2147483647, "Int32 out of range")
    appendU32(output, value < 0 and value + 4294967296 or value)
end

local function appendI64(output, value)
    assert(value % 1 == 0, "Int64 must be an integer")
    local negative = value < 0
    if negative then
        value = value + 18446744073709551616
    end
    for _ = 1, 8 do
        appendU8(output, value % 256)
        value = math.floor(value / 256)
    end
end

local function appendF32(output, value)
    table.insert(output, string.pack("<f", value))
end

local function appendF64(output, value)
    table.insert(output, string.pack("<d", value))
end

local function appendString(output, value)
    assert(type(value) == "string", "RBXI string expected")
    assert(#value <= 1048576, "RBXI string is too long")
    appendU32(output, #value)
    table.insert(output, value)
end

local function readU8(data, position)
    assert(position <= #data, "unexpected end of RBXI packet")
    return string.byte(data, position), position + 1
end

local function readU32(data, position)
    assert(position + 3 <= #data, "unexpected end of RBXI packet")
    local a, b, c, d = string.byte(data, position, position + 3)
    return a + b * 256 + c * 65536 + d * 16777216, position + 4
end

local function readString(data, position)
    local length
    length, position = readU32(data, position)
    assert(length <= 1048576, "RBXI string is too long")
    assert(position + length - 1 <= #data, "unexpected end of RBXI packet")
    return string.sub(data, position, position + length - 1), position + length
end

local function encodeProperty(output, property)
    assert(type(property) == "table", "property record expected")
    local kind = property.valueType
    local value = property.value
    assert(type(kind) == "string", "property valueType is required")
    appendString(output, kind)

    if kind == "String" or kind == "Ref" then
        assert(type(value) == "string", kind .. " property requires a string")
        appendString(output, value)
    elseif kind == "Bool" then
        assert(type(value) == "boolean", "Bool property requires a boolean")
        appendU8(output, value and 1 or 0)
    elseif kind == "Int32" then
        assert(type(value) == "number", "Int32 property requires a number")
        appendI32(output, value)
    elseif kind == "Int64" then
        assert(type(value) == "number", "Int64 property requires a number")
        appendI64(output, value)
    elseif kind == "Float32" then
        assert(type(value) == "number", "Float32 property requires a number")
        appendF32(output, value)
    elseif kind == "Float64" then
        assert(type(value) == "number", "Float64 property requires a number")
        appendF64(output, value)
    elseif kind == "Vector2" or kind == "Vector3" or kind == "Color3" then
        assert(type(value) == "table", kind .. " property requires an array")
        local count = kind == "Vector2" and 2 or 3
        assert(#value == count, kind .. " requires " .. count .. " components")
        for index = 1, count do
            assert(type(value[index]) == "number", kind .. " components must be numbers")
            appendF32(output, value[index])
        end
    else
        error("unsupported RBXI property type: " .. kind)
    end
end

function RBXM.encodePacket(instances)
    assert(type(instances) == "table", "instances must be a table")
    assert(#instances <= 100000, "too many RBXI instances")

    local output = { RBXM.MAGIC }
    appendU8(output, RBXM.VERSION)
    appendU32(output, #instances)

    for _, instance in ipairs(instances) do
        assert(type(instance) == "table", "instance record expected")
        assert(type(instance.referent) == "string" and instance.referent ~= "", "instance referent is required")
        assert(type(instance.className) == "string" and instance.className ~= "", "instance className is required")
        assert(type(instance.name) == "string", "instance name is required")

        appendString(output, instance.referent)
        appendString(output, instance.parent or "")
        appendString(output, instance.className)
        appendString(output, instance.name)

        local properties = instance.properties or {}
        local propertyNames = {}
        for propertyName in pairs(properties) do
            assert(type(propertyName) == "string", "property names must be strings")
            table.insert(propertyNames, propertyName)
        end
        table.sort(propertyNames)
        assert(#propertyNames <= 100000, "too many properties")
        appendU32(output, #propertyNames)

        for _, propertyName in ipairs(propertyNames) do
            appendString(output, propertyName)
            encodeProperty(output, properties[propertyName])
        end
    end

    return table.concat(output)
end

local function decodeProperty(data, position, kind)
    if kind == "String" or kind == "Ref" then
        return readString(data, position)
    elseif kind == "Bool" then
        local byte
        byte, position = readU8(data, position)
        assert(byte <= 1, "invalid Bool value")
        return byte == 1, position
    elseif kind == "Int32" then
        local raw
        raw, position = readU32(data, position)
        return raw >= 2147483648 and raw - 4294967296 or raw, position
    elseif kind == "Int64" then
        local value
        value, position = string.unpack("<i8", data, position)
        return value, position
    elseif kind == "Float32" then
        local value
        value, position = string.unpack("<f", data, position)
        return value, position
    elseif kind == "Float64" then
        local value
        value, position = string.unpack("<d", data, position)
        return value, position
    elseif kind == "Vector2" or kind == "Vector3" or kind == "Color3" then
        local count = kind == "Vector2" and 2 or 3
        local value = {}
        for index = 1, count do
            value[index], position = string.unpack("<f", data, position)
        end
        return value, position
    else
        error("unsupported RBXI property type: " .. kind)
    end
end

function RBXM.decodePacket(data)
    assert(type(data) == "string", "RBXI packet must be a string")
    assert(string.sub(data, 1, 4) == RBXM.MAGIC, "invalid RBXI magic")
    assert(string.byte(data, 5) == RBXM.VERSION, "unsupported RBXI version")

    local position = 6
    local count
    count, position = readU32(data, position)
    assert(count <= 100000, "too many RBXI instances")
    local instances = {}

    for _ = 1, count do
        local referent, parent, className, name
        referent, position = readString(data, position)
        parent, position = readString(data, position)
        className, position = readString(data, position)
        name, position = readString(data, position)

        local propertyCount
        propertyCount, position = readU32(data, position)
        assert(propertyCount <= 100000, "too many RBXI properties")
        local properties = {}

        for _ = 1, propertyCount do
            local propertyName, kind, value
            propertyName, position = readString(data, position)
            kind, position = readString(data, position)
            value, position = decodeProperty(data, position, kind)
            properties[propertyName] = { valueType = kind, value = value }
        end

        table.insert(instances, {
            referent = referent,
            parent = parent == "" and nil or parent,
            className = className,
            name = name,
            properties = properties,
        })
    end

    assert(position == #data + 1, "trailing bytes after RBXI packet")
    return instances
end

function RBXM.encode(instances, options)
    assert(type(options) == "table" and type(options.baseUrl) == "string", "options.baseUrl is required")
    local response = HttpService:RequestAsync({
        Url = options.baseUrl .. "/v1/rbxm/encode",
        Method = "POST",
        Headers = { ["Content-Type"] = "application/octet-stream" },
        Body = RBXM.encodePacket(instances),
    })
    assert(response.Success, "RBXM encode failed: " .. tostring(response.StatusCode) .. " " .. tostring(response.StatusMessage))
    return response.Body
end

function RBXM.decode(data, options)
    assert(type(data) == "string", "RBXM bytes must be a string")
    assert(type(options) == "table" and type(options.baseUrl) == "string", "options.baseUrl is required")
    local response = HttpService:RequestAsync({
        Url = options.baseUrl .. "/v1/rbxm/decode",
        Method = "POST",
        Headers = { ["Content-Type"] = "application/octet-stream" },
        Body = data,
    })
    assert(response.Success, "RBXM decode failed: " .. tostring(response.StatusCode) .. " " .. tostring(response.StatusMessage))
    return RBXM.decodePacket(response.Body)
end

return RBXM
