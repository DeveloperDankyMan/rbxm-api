--!nonstrict
--[[
	RBXM — client for the rbxm-api server (Rust + rbx-dom).

	Runs on the SERVER only (HttpService is unavailable to LocalScripts) and needs
	"Allow HTTP Requests" enabled in Game Settings > Security.

	local RBXM = require(path.to.RBXM)
	local client = RBXM.new({
		BaseUrl = "https://rbxm.example.com",
		ApiKey  = "same value as the server's RBXM_API_KEY",  -- keep this in a server-only module / Secret
	})

	local buf = client:Encode({ workspace.MyModel })      --> buffer containing a real .rbxm file
	local roots = client:Decode(buf, workspace)           --> { Instance } (parented to `workspace`)

	Encode options:
		IncludeDefaults : boolean   send every property, even ones equal to Instance.new defaults
		Properties      : { [Instance]: { [propertyName]: value } }
		                  extra/override values, e.g. Script.Source, which scripts cannot read at runtime:
		                  { [myScript] = { Source = "print('hi')" } }

	Notes / limits
		* Script.Source can't be read or written by non-plugin code, so decoded scripts come out empty
		  unless the server side of your game injects the source itself (see `Properties` above for encode).
		* Properties Roblox doesn't let scripts read/write are skipped silently (set client.Verbose = true to see them).
		* HttpService is limited to 500 requests/min per game server. Encode/Decode use 1 request each,
		  plus 1 per 50 previously-unseen classes (schemas are cached for the lifetime of the server).
]]

local HttpService = game:GetService("HttpService")

local RBXM = {}
RBXM.__index = RBXM

export type Config = {
	BaseUrl: string,
	ApiKey: string?,
	-- Send/receive base64 text instead of raw bytes. Use this if raw binary bodies get mangled on your setup.
	Base64: boolean?,
	Verbose: boolean?,
}

-- ---------------------------------------------------------------------------------------------
-- base64 (buffer based)
-- ---------------------------------------------------------------------------------------------

local B64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
local ENC = buffer.create(64)
local DEC: { [number]: number } = {}
for i = 1, 64 do
	local c = string.byte(B64, i)
	buffer.writeu8(ENC, i - 1, c)
	DEC[c] = i - 1
end

local function b64encode(s: string): string
	local n = #s
	if n == 0 then
		return ""
	end
	local src = buffer.fromstring(s)
	local out = buffer.create(((n + 2) // 3) * 4)
	local o = 0
	local full = n - n % 3
	for i = 0, full - 1, 3 do
		local v = buffer.readu8(src, i) * 65536 + buffer.readu8(src, i + 1) * 256 + buffer.readu8(src, i + 2)
		buffer.writeu8(out, o, buffer.readu8(ENC, v // 262144))
		buffer.writeu8(out, o + 1, buffer.readu8(ENC, (v // 4096) % 64))
		buffer.writeu8(out, o + 2, buffer.readu8(ENC, (v // 64) % 64))
		buffer.writeu8(out, o + 3, buffer.readu8(ENC, v % 64))
		o += 4
	end
	local rem = n - full
	if rem == 1 then
		local v = buffer.readu8(src, full) * 65536
		buffer.writeu8(out, o, buffer.readu8(ENC, v // 262144))
		buffer.writeu8(out, o + 1, buffer.readu8(ENC, (v // 4096) % 64))
		buffer.writeu8(out, o + 2, 61) -- '='
		buffer.writeu8(out, o + 3, 61)
	elseif rem == 2 then
		local v = buffer.readu8(src, full) * 65536 + buffer.readu8(src, full + 1) * 256
		buffer.writeu8(out, o, buffer.readu8(ENC, v // 262144))
		buffer.writeu8(out, o + 1, buffer.readu8(ENC, (v // 4096) % 64))
		buffer.writeu8(out, o + 2, buffer.readu8(ENC, (v // 64) % 64))
		buffer.writeu8(out, o + 3, 61)
	end
	return buffer.tostring(out)
end

local function b64decode(s: string): string
	local n = #s
	if n == 0 then
		return ""
	end
	local src = buffer.fromstring(s)
	local out = buffer.create((n // 4) * 3 + 3)
	local o, acc, bits = 0, 0, 0
	for i = 0, n - 1 do
		local d = DEC[buffer.readu8(src, i)]
		if d then -- ignores '=' and whitespace
			acc = bit32.bor(bit32.lshift(acc, 6), d)
			bits += 6
			if bits >= 8 then
				bits -= 8
				buffer.writeu8(out, o, bit32.band(bit32.rshift(acc, bits), 255))
				o += 1
				acc = bit32.band(acc, bit32.lshift(1, bits) - 1)
			end
		end
	end
	return buffer.readstring(out, 0, o)
end

-- ---------------------------------------------------------------------------------------------
-- Instance value  ->  wire value
-- ---------------------------------------------------------------------------------------------

local function fin(n: number): number
	if n ~= n or n == math.huge or n == -math.huge then
		return 0
	end
	return n
end

local function v3(v: Vector3)
	return { fin(v.X), fin(v.Y), fin(v.Z) }
end

local function cf(c: CFrame)
	local t = { c:GetComponents() }
	for i = 1, #t do
		t[i] = fin(t[i])
	end
	return t
end

local encoders: { [string]: (any, any) -> any } = {
	Bool = function(v)
		return v == true
	end,
	Int32 = function(v)
		return math.floor(fin(v))
	end,
	Int64 = function(v)
		return math.floor(fin(v))
	end,
	Float32 = function(v)
		return fin(v)
	end,
	Float64 = function(v)
		return fin(v)
	end,
	String = function(v)
		return tostring(v)
	end,
	BinaryString = function(v)
		return b64encode(v)
	end,
	SharedString = function(v)
		return b64encode(v)
	end,
	ContentId = function(v)
		return tostring(v)
	end,
	Content = function(v)
		if typeof(v) == "string" then
			return v
		end
		local ok, uri = pcall(function()
			return v.Uri
		end)
		return if ok and typeof(uri) == "string" then uri else ""
	end,
	Vector2 = function(v)
		return { fin(v.X), fin(v.Y) }
	end,
	Vector2int16 = function(v)
		return { v.X, v.Y }
	end,
	Vector3 = v3,
	Vector3int16 = function(v)
		return { v.X, v.Y, v.Z }
	end,
	CFrame = cf,
	OptionalCFrame = function(v)
		return if v == nil then nil else cf(v)
	end,
	Color3 = function(c)
		return { c.R, c.G, c.B }
	end,
	Color3uint8 = function(c)
		return { math.floor(c.R * 255 + 0.5), math.floor(c.G * 255 + 0.5), math.floor(c.B * 255 + 0.5) }
	end,
	BrickColor = function(b)
		return b.Number
	end,
	UDim = function(u)
		return { fin(u.Scale), u.Offset }
	end,
	UDim2 = function(u)
		return { fin(u.X.Scale), u.X.Offset, fin(u.Y.Scale), u.Y.Offset }
	end,
	Rect = function(r)
		return { fin(r.Min.X), fin(r.Min.Y), fin(r.Max.X), fin(r.Max.Y) }
	end,
	Ray = function(r)
		return { fin(r.Origin.X), fin(r.Origin.Y), fin(r.Origin.Z), fin(r.Direction.X), fin(r.Direction.Y), fin(r.Direction.Z) }
	end,
	NumberRange = function(r)
		return { fin(r.Min), fin(r.Max) }
	end,
	NumberSequence = function(s)
		local out = {}
		for i, k in ipairs(s.Keypoints) do
			out[i] = { k.Time, fin(k.Value), fin(k.Envelope) }
		end
		return out
	end,
	ColorSequence = function(s)
		local out = {}
		for i, k in ipairs(s.Keypoints) do
			out[i] = { k.Time, k.Value.R, k.Value.G, k.Value.B }
		end
		return out
	end,
	PhysicalProperties = function(p)
		if p == nil then
			return nil -- default physical properties
		end
		return { p.Density, p.Friction, p.Elasticity, p.FrictionWeight, p.ElasticityWeight }
	end,
	Enum = function(e)
		return e.Value
	end,
	Faces = function(f)
		return (f.Right and 1 or 0) + (f.Top and 2 or 0) + (f.Back and 4 or 0) + (f.Left and 8 or 0) + (f.Bottom and 16 or 0) + (f.Front and 32 or 0)
	end,
	Axes = function(a)
		return (a.X and 1 or 0) + (a.Y and 2 or 0) + (a.Z and 4 or 0)
	end,
	Font = function(f)
		return { family = f.Family, weight = f.Weight.Value, style = f.Style.Value }
	end,
}

-- Attribute types the server's rbx_types attribute writer understands.
local ATTRIBUTE_TYPES: { [string]: string } = {
	string = "String",
	boolean = "Bool",
	number = "Float64",
	Vector2 = "Vector2",
	Vector3 = "Vector3",
	Color3 = "Color3",
	BrickColor = "BrickColor",
	UDim = "UDim",
	UDim2 = "UDim2",
	Rect = "Rect",
	NumberRange = "NumberRange",
	NumberSequence = "NumberSequence",
	ColorSequence = "ColorSequence",
	CFrame = "CFrame",
}

local function encodeAttributes(attrs: { [string]: any })
	local out, any = {}, false
	for name, value in pairs(attrs) do
		local wireType = ATTRIBUTE_TYPES[typeof(value)]
		if wireType then
			out[name] = { t = wireType, v = encoders[wireType](value) }
			any = true
		end
	end
	return if any then out else nil
end

-- ---------------------------------------------------------------------------------------------
-- wire value  ->  Instance value
-- ---------------------------------------------------------------------------------------------

local enumCache: { [string]: { [number]: EnumItem } } = {}

local function enumItem(enumName: string, value: number): EnumItem?
	local items = enumCache[enumName]
	if not items then
		items = {}
		local ok, enum = pcall(function()
			return (Enum :: any)[enumName]
		end)
		if ok and enum then
			for _, item in ipairs(enum:GetEnumItems()) do
				items[item.Value] = item
			end
		end
		enumCache[enumName] = items
	end
	return items[value]
end

local decoders: { [string]: (any, string?) -> any }
decoders = {
	Bool = function(v)
		return v
	end,
	Int32 = function(v)
		return v
	end,
	Int64 = function(v)
		return v
	end,
	Float32 = function(v)
		return v
	end,
	Float64 = function(v)
		return v
	end,
	String = function(v)
		return v
	end,
	BinaryString = function(v)
		return b64decode(v)
	end,
	SharedString = function(v)
		return b64decode(v)
	end,
	ContentId = function(v)
		return v
	end,
	Content = function(v)
		return if v == "" then Content.none else Content.fromUri(v)
	end,
	Vector2 = function(v)
		return Vector2.new(v[1], v[2])
	end,
	Vector2int16 = function(v)
		return Vector2int16.new(v[1], v[2])
	end,
	Vector3 = function(v)
		return Vector3.new(v[1], v[2], v[3])
	end,
	Vector3int16 = function(v)
		return Vector3int16.new(v[1], v[2], v[3])
	end,
	CFrame = function(v)
		return CFrame.new(unpack(v))
	end,
	OptionalCFrame = function(v)
		return if v == nil then nil else CFrame.new(unpack(v))
	end,
	Color3 = function(v)
		return Color3.new(v[1], v[2], v[3])
	end,
	Color3uint8 = function(v)
		return Color3.fromRGB(v[1], v[2], v[3])
	end,
	BrickColor = function(v)
		return BrickColor.new(v)
	end,
	UDim = function(v)
		return UDim.new(v[1], v[2])
	end,
	UDim2 = function(v)
		return UDim2.new(v[1], v[2], v[3], v[4])
	end,
	Rect = function(v)
		return Rect.new(v[1], v[2], v[3], v[4])
	end,
	Ray = function(v)
		return Ray.new(Vector3.new(v[1], v[2], v[3]), Vector3.new(v[4], v[5], v[6]))
	end,
	NumberRange = function(v)
		return NumberRange.new(v[1], v[2])
	end,
	NumberSequence = function(v)
		local kps = {}
		for i, k in ipairs(v) do
			kps[i] = NumberSequenceKeypoint.new(k[1], k[2], k[3])
		end
		return NumberSequence.new(kps)
	end,
	ColorSequence = function(v)
		local kps = {}
		for i, k in ipairs(v) do
			kps[i] = ColorSequenceKeypoint.new(k[1], Color3.new(k[2], k[3], k[4]))
		end
		return ColorSequence.new(kps)
	end,
	PhysicalProperties = function(v)
		if v == nil then
			return nil
		end
		return PhysicalProperties.new(v[1], v[2], v[3], v[4], v[5])
	end,
	Faces = function(bits)
		local ids = {}
		if bit32.band(bits, 1) ~= 0 then table.insert(ids, Enum.NormalId.Right) end
		if bit32.band(bits, 2) ~= 0 then table.insert(ids, Enum.NormalId.Top) end
		if bit32.band(bits, 4) ~= 0 then table.insert(ids, Enum.NormalId.Back) end
		if bit32.band(bits, 8) ~= 0 then table.insert(ids, Enum.NormalId.Left) end
		if bit32.band(bits, 16) ~= 0 then table.insert(ids, Enum.NormalId.Bottom) end
		if bit32.band(bits, 32) ~= 0 then table.insert(ids, Enum.NormalId.Front) end
		return Faces.new(unpack(ids))
	end,
	Axes = function(bits)
		local ids = {}
		if bit32.band(bits, 1) ~= 0 then table.insert(ids, Enum.Axis.X) end
		if bit32.band(bits, 2) ~= 0 then table.insert(ids, Enum.Axis.Y) end
		if bit32.band(bits, 4) ~= 0 then table.insert(ids, Enum.Axis.Z) end
		return Axes.new(unpack(ids))
	end,
	Font = function(v)
		local weight = enumItem("FontWeight", v.weight) or Enum.FontWeight.Regular
		local style = enumItem("FontStyle", v.style) or Enum.FontStyle.Normal
		return Font.new(v.family, weight :: any, style :: any)
	end,
}

-- ---------------------------------------------------------------------------------------------
-- client
-- ---------------------------------------------------------------------------------------------

function RBXM.new(config: Config)
	assert(type(config) == "table" and type(config.BaseUrl) == "string", "RBXM.new: BaseUrl is required")
	local self = setmetatable({}, RBXM)
	self.BaseUrl = string.gsub(config.BaseUrl, "/+$", "")
	self.ApiKey = config.ApiKey
	self.Base64 = config.Base64 == true
	self.Verbose = config.Verbose == true
	self._schemas = {} :: { [string]: { { name: string, type: string, enum: string? } } }
	self._defaults = {} :: { [string]: Instance | false }
	return self
end

function RBXM:_request(method: string, path: string, body: string?, contentType: string?): string
	local headers = {}
	if self.ApiKey then
		headers["x-api-key"] = self.ApiKey
	end
	if contentType then
		headers["Content-Type"] = contentType
	end
	local ok, res = pcall(function()
		return HttpService:RequestAsync({
			Url = self.BaseUrl .. path,
			Method = method,
			Headers = headers,
			Body = body,
		})
	end)
	if not ok then
		error("RBXM: HTTP request failed: " .. tostring(res), 3)
	end
	if not res.Success then
		error(string.format("RBXM: server returned %d %s: %s", res.StatusCode, res.StatusMessage, string.sub(res.Body, 1, 400)), 3)
	end
	return res.Body
end

function RBXM:Ping(): boolean
	local ok = pcall(function()
		return self:_request("GET", "/health")
	end)
	return ok
end

-- Fetch the property schemas of every class we haven't seen yet (batched).
function RBXM:_loadSchemas(classes: { string })
	local missing = {}
	for _, class in ipairs(classes) do
		if self._schemas[class] == nil then
			table.insert(missing, class)
		end
	end
	local BATCH = 50
	for i = 1, #missing, BATCH do
		local batch = table.move(missing, i, math.min(i + BATCH - 1, #missing), 1, {})
		local body = self:_request("GET", "/schemas?classes=" .. HttpService:UrlEncode(table.concat(batch, ",")))
		local decoded = HttpService:JSONDecode(body)
		for _, s in ipairs(decoded.schemas) do
			self._schemas[s.class] = s.properties
		end
		for _, class in ipairs(batch) do
			if self._schemas[class] == nil then
				self._schemas[class] = {} -- unknown to the server's reflection database
				warn(("RBXM: server doesn't know class %q; only Name will be encoded"):format(class))
			end
		end
	end
end

function RBXM:_defaultFor(class: string): Instance?
	local cached = self._defaults[class]
	if cached == nil then
		local ok, inst = pcall(Instance.new, class :: any)
		cached = if ok then inst else false
		self._defaults[class] = cached
	end
	return if cached then cached :: Instance else nil
end

-- ---------------------------------------------------------------------------------------------
-- Encode
-- ---------------------------------------------------------------------------------------------

export type EncodeOptions = {
	IncludeDefaults: boolean?,
	Properties: { [Instance]: { [string]: any } }?,
}

function RBXM:Encode(instances: { Instance }, options: EncodeOptions?): buffer
	options = options or {}
	local extra = options.Properties or {}

	-- Collect the instances (roots + descendants, de-duplicated) and give each an id.
	local list: { Instance } = {}
	local ids: { [Instance]: string } = {}
	local function add(inst: Instance)
		if not ids[inst] then
			table.insert(list, inst)
			ids[inst] = tostring(#list)
		end
	end
	for _, root in ipairs(instances) do
		add(root)
		for _, d in ipairs(root:GetDescendants()) do
			add(d)
		end
	end

	local classSet, classes = {}, {}
	for _, inst in ipairs(list) do
		if not classSet[inst.ClassName] then
			classSet[inst.ClassName] = true
			table.insert(classes, inst.ClassName)
		end
	end
	self:_loadSchemas(classes)

	local out = table.create(#list)
	for i, inst in ipairs(list) do
		local class = inst.ClassName
		local schema = self._schemas[class]
		local defaults = if options.IncludeDefaults then nil else self:_defaultFor(class)
		local props = {}
		local overrides = extra[inst]

		for _, p in ipairs(schema) do
			local override = overrides and overrides[p.name]
			local value
			if override ~= nil then
				value = override
			else
				local ok, v
				if p.name == "Tags" then
					ok, v = true, inst:GetTags()
					if #v == 0 then
						v = nil
					end
				elseif p.name == "Attributes" then
					ok, v = true, encodeAttributes(inst:GetAttributes())
					if v == nil then
						continue
					end
					props[p.name] = { t = "Attributes", v = v }
					continue
				else
					ok, v = pcall(function()
						return inst[p.name]
					end)
				end
				if not ok or v == nil then
					continue
				end
				if defaults then
					local dok, dv = pcall(function()
						return defaults[p.name]
					end)
					if dok and dv == v then
						continue
					end
				end
				value = v
			end

			if p.type == "Ref" then
				value = ids[value]
				if value == nil then
					continue
				end
			elseif p.name == "Tags" then
				-- already a plain string array
			else
				local enc = encoders[p.type]
				if not enc then
					continue
				end
				local ok, encoded = pcall(enc, value)
				if not ok then
					if self.Verbose then
						warn(("RBXM: couldn't encode %s.%s: %s"):format(class, p.name, tostring(encoded)))
					end
					continue
				end
				value = encoded
			end
			props[p.name] = { t = p.type, v = value }
		end

		-- properties supplied through options.Properties that aren't in the schema
		if overrides then
			for name, value in pairs(overrides) do
				if props[name] == nil then
					local known = nil
					for _, p in ipairs(schema) do
						if p.name == name then
							known = p
						end
					end
					if not known then
						warn(("RBXM: %s has no serializable property %q; ignored"):format(class, name))
					end
				end
			end
		end

		local parent = inst.Parent
		out[i] = {
			id = ids[inst],
			class = class,
			name = inst.Name,
			parent = if parent then ids[parent] else nil,
			properties = props,
		}
	end

	local json = HttpService:JSONEncode({ instances = out })
	local body = self:_request("POST", if self.Base64 then "/encode?b64=1" else "/encode", json, "application/json")
	if self.Base64 then
		body = b64decode(body)
	end
	return buffer.fromstring(body)
end

-- ---------------------------------------------------------------------------------------------
-- Decode
-- ---------------------------------------------------------------------------------------------

local function decodeWire(w: any): any
	if w.t == "Enum" then
		return if w.e then enumItem(w.e, w.v) else nil
	end
	local dec = decoders[w.t]
	if not dec then
		return nil
	end
	return dec(w.v)
end

function RBXM:Decode(data: buffer | string, parent: Instance?): { Instance }
	local raw = if typeof(data) == "buffer" then buffer.tostring(data) else data
	local body = self:_request(
		"POST",
		if self.Base64 then "/decode?b64=1" else "/decode",
		if self.Base64 then b64encode(raw) else raw,
		if self.Base64 then "text/plain" else "application/octet-stream"
	)
	local tree = HttpService:JSONDecode(body)

	local byId: { [string]: Instance } = {}
	local function report(msg: string)
		if self.Verbose then
			warn("RBXM: " .. msg)
		end
	end

	-- 1) create every instance
	for _, d in ipairs(tree.instances) do
		local dead = d.parent ~= nil and byId[d.parent] == nil -- parent couldn't be created
		if not dead then
			local ok, inst = pcall(Instance.new, d.class)
			if ok then
				byId[d.id] = inst
			else
				report(("can't create %s (%s)"):format(d.class, d.name or "?"))
			end
		end
	end

	-- 2) plain properties, name, tags (no refs / attributes yet)
	for _, d in ipairs(tree.instances) do
		local inst = byId[d.id]
		if inst then
			if d.name then
				inst.Name = d.name
			end
			for prop, w in pairs(d.properties) do
				if w.t ~= "Ref" and w.t ~= "Attributes" and w.t ~= "Tags" then
					local ok, value = pcall(decodeWire, w)
					if ok and value ~= nil then
						local sok, err = pcall(function()
							inst[prop] = value
						end)
						if not sok then
							report(("%s.%s not settable (%s)"):format(d.class, prop, tostring(err)))
						end
					elseif not ok then
						report(("bad value for %s.%s: %s"):format(d.class, prop, tostring(value)))
					end
				elseif w.t == "Tags" then
					for _, tag in ipairs(w.v) do
						pcall(inst.AddTag, inst, tag)
					end
				end
			end
		end
	end

	-- 3) refs + attributes (all instances exist now)
	for _, d in ipairs(tree.instances) do
		local inst = byId[d.id]
		if inst then
			for prop, w in pairs(d.properties) do
				if w.t == "Ref" then
					local target = if w.v ~= nil then byId[w.v] else nil
					if target then
						pcall(function()
							inst[prop] = target
						end)
					end
				elseif w.t == "Attributes" then
					for name, aw in pairs(w.v) do
						local ok, value = pcall(decodeWire, aw)
						if ok and value ~= nil then
							pcall(inst.SetAttribute, inst, name, value)
						end
					end
				end
			end
		end
	end

	-- 4) hierarchy (parents first, so roots stay unparented until the very end)
	local roots: { Instance } = {}
	for _, d in ipairs(tree.instances) do
		local inst = byId[d.id]
		if inst then
			if d.parent == nil then
				table.insert(roots, inst)
			else
				inst.Parent = byId[d.parent]
			end
		end
	end
	if parent then
		for _, root in ipairs(roots) do
			root.Parent = parent
		end
	end
	return roots
end

return RBXM
