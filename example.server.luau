-- Example: round-trip a model through the rbxm-api server.
-- Put RBXM.lua (ModuleScript) next to this Script inside ServerScriptService.
local RBXM = require(script.Parent.RBXM)

local client = RBXM.new({
	BaseUrl = "https://rbxm.example.com", -- your deployed server (HTTPS recommended)
	ApiKey = "change-me",                 -- must match RBXM_API_KEY on the server
	Verbose = true,
})

local model = workspace:FindFirstChildWhichIsA("Model")
if not model then
	warn("Put a Model in workspace to try this")
	return
end

-- Encode -> real .rbxm bytes in a buffer
local rbxm = client:Encode({ model })
print("encoded", buffer.len(rbxm), "bytes")

-- Decode -> fresh copy of the model
local copies = client:Decode(rbxm, workspace)
for _, copy in ipairs(copies) do
	copy:PivotTo(copy:GetPivot() + Vector3.new(10, 0, 0))
end
