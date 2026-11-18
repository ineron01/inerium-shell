-- Universal workspace configuration
-- Wiki: https://wiki.hypr.land/Configuring/Workspace-Rules/

local count = 3
local home = os.getenv("HOME") or ""
local count_file = home .. "/.config/hypr/config/workspaces_count"
local f = io.open(count_file, "r")
if f then
    local content = f:read("*all")
    f:close()
    local n = tonumber(content and content:match("%d+"))
    if n and n >= 1 and n <= 10 then
        count = n
    end
end

for i = 1, count do
    if i == 1 then
        hl.workspace_rule({ workspace = 1, default = true, persistent = true })
    else
        hl.workspace_rule({ workspace = i, persistent = true })
    end
end
