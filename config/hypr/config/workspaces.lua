-- Workspace rules
-- Wiki: https://wiki.hypr.land/Configuring/Workspace-Rules/

-- Workspaces 1-3: Laptop display (eDP-2 hybrid / eDP-1 integrated)
for ws = 1, 3 do
    hl.workspace_rule({ workspace = tostring(ws), monitor = "eDP-2", default = (ws == 1), persistent = true })
    hl.workspace_rule({ workspace = tostring(ws), monitor = "eDP-1" })
end

-- Workspaces 4-6: External display (HDMI-A-1)
for ws = 4, 6 do
    hl.workspace_rule({ workspace = tostring(ws), monitor = "HDMI-A-1", default = (ws == 4), persistent = true })
end

-- Gaming workspace pinned to external display
hl.workspace_rule({ workspace = "name:gaming", monitor = "HDMI-A-1" })
