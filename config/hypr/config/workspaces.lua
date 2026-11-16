-- Universal workspace configuration
-- Wiki: https://wiki.hypr.land/Configuring/Workspace-Rules/

local count_path = (os.getenv("HOME") or "") .. "/.config/hypr/config/workspaces_count"
local total = 10
local cf = io.open(count_path, "r")
if cf then
    local v = tonumber(cf:read("*l"))
    cf:close()
    if v and v >= 1 and v <= 10 then total = math.floor(v) end
end

for ws = 1, total do
    hl.workspace_rule({ workspace = ws, default = (ws == 1), persistent = true })
end
