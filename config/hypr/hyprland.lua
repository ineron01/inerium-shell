-- CachyOS Hyprland Configuration

local function safe_require(mod)
    local ok, err = pcall(require, mod)
    if not ok then
        io.stderr:write("[Hyprland Config Error] Failed loading " .. mod .. ": " .. tostring(err) .. "\n")
    end
    return ok
end

-- Core configuration modules (safe require: error in one module does not crash the session)
safe_require("config.animations")
package.loaded["config.colors"] = nil
safe_require("config.colors")
safe_require("config.decorations")
safe_require("config.variables")
safe_require("config.environment")
safe_require("config.input")
safe_require("config.misc")
safe_require("config.monitors")
safe_require("config.windowrules")
safe_require("config.workspaces")
safe_require("config.autostart")
safe_require("config.keybinds")
