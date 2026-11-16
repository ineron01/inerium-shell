-- Monitor configuration
-- Wiki: https://wiki.hypr.land/Configuring/Monitors/

local has_local = pcall(require, "config.monitors_local")
if not has_local then
    -- Laptop internal display: Hybrid (144Hz) or Integrated (60Hz)
    hl.monitor({ output = "eDP-2", mode = "1920x1080@144", position = "0x216", scale = 1.25, vrr = 0 })
    hl.monitor({ output = "eDP-1", mode = "1920x1080@60",  position = "0x216", scale = 1.25, vrr = 0 })

    -- External display (HDMI right of laptop)
    hl.monitor({ output = "HDMI-A-1", mode = "preferred", position = "1536x0", scale = 1.0, vrr = 0 })

    -- Universal fallback for newly connected displays
    hl.monitor({ output = "", mode = "preferred", position = "auto", scale = 1.0 })
end
