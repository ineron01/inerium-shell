-- Keybindings configurationwiki https://wiki.hypr.land/Configuring/Binds/

local mainMod = "SUPER"
local home = os.getenv("HOME") or ""
local launchPrefix = "uwsm app -- "

local function digitCode(d)
    return "code:" .. (d == 0 and 19 or (9 + d))
end

-----------------------------------
---- 1. SYSTEM MENUS & POPUPS ----
-----------------------------------

-- Application launcher (Rofi drun)
hl.bind(mainMod .. " + Space", hl.dsp.exec_cmd(launchPrefix .. "rofi -show drun"))

-- Window switcher
hl.bind(mainMod .. " + Tab", hl.dsp.exec_cmd(launchPrefix .. "rofi -show window"))

-- Control Center
hl.bind(mainMod .. " + A", hl.dsp.exec_cmd(home .. "/.local/bin/inerium-popup control"))

-- Clipboard manager
hl.bind(mainMod .. " + V", hl.dsp.exec_cmd(launchPrefix .. home .. "/.local/bin/inerium-clipboard"))

-- Session lock & power menu
hl.bind(mainMod .. " + L", hl.dsp.exec_cmd("hyprlock || loginctl lock-session"))
hl.bind(mainMod .. " + ALT + C", hl.dsp.exec_cmd(home .. "/.local/bin/inerium-popup power"))
hl.bind("XF86PowerOff", hl.dsp.exec_cmd(home .. "/.local/bin/inerium-popup power"))

-------------------------------
---- 2. WINDOW MANAGEMENT ----
-------------------------------

hl.bind(mainMod .. " + Escape",      hl.dsp.exec_cmd("hyprctl kill"))
hl.bind(mainMod .. " + Q",           hl.dsp.window.close())
hl.bind(mainMod .. " + SHIFT + Q",   hl.dsp.window.kill())
hl.bind(mainMod .. " + ALT + Space", hl.dsp.window.float({ action = "toggle" }))
hl.bind(mainMod .. " + D",           hl.dsp.window.fullscreen({ mode = 1 }))
hl.bind(mainMod .. " + F",           hl.dsp.window.fullscreen())
hl.bind(mainMod .. " + J",           hl.dsp.layout("togglesplit"))

-- Focus navigation
hl.bind(mainMod .. " + Left",  hl.dsp.focus({ direction = "left" }))
hl.bind(mainMod .. " + Right", hl.dsp.focus({ direction = "right" }))
hl.bind(mainMod .. " + Up",    hl.dsp.focus({ direction = "up" }))
hl.bind(mainMod .. " + Down",  hl.dsp.focus({ direction = "down" }))
hl.bind("ALT + Tab",           hl.dsp.window.cycle_next())

-- Window movement
hl.bind(mainMod .. " + SHIFT + Up",                   hl.dsp.window.move({ direction = "u" }))
hl.bind(mainMod .. " + SHIFT + Right",                hl.dsp.window.move({ direction = "r" }))
hl.bind(mainMod .. " + SHIFT + Left",                 hl.dsp.window.move({ direction = "l" }))
hl.bind(mainMod .. " + SHIFT + Down",                 hl.dsp.window.move({ direction = "d" }))
hl.bind(mainMod .. " + SHIFT + mouse_up",             hl.dsp.window.move({ monitor   = "-1" }))
hl.bind(mainMod .. " + SHIFT + mouse_down",           hl.dsp.window.move({ monitor   = "+1" }))
hl.bind(mainMod .. " + CONTROL + SHIFT + Right",      hl.dsp.window.move({ workspace = "m+1" }))
hl.bind(mainMod .. " + CONTROL + SHIFT + Left",       hl.dsp.window.move({ workspace = "m-1" }))
hl.bind(mainMod .. " + CONTROL + SHIFT + mouse_up",   hl.dsp.window.move({ workspace = "m-1" }))
hl.bind(mainMod .. " + CONTROL + SHIFT + mouse_down", hl.dsp.window.move({ workspace = "m+1" }))

-- Mouse window drag & resize
hl.bind(mainMod .. " + mouse:272", hl.dsp.window.drag())
hl.bind(mainMod .. " + mouse:273", hl.dsp.window.resize())

------------------------------
---- 3. DEFAULT APPS --------
------------------------------

hl.bind(mainMod .. " + Return",     hl.dsp.exec_cmd(launchPrefix .. TERMINAL))
hl.bind(mainMod .. " + E",          hl.dsp.exec_cmd(launchPrefix .. FILE_MANAGER))
hl.bind(mainMod .. " + T",          hl.dsp.exec_cmd(launchPrefix .. EDITOR))
hl.bind(mainMod .. " + C",          hl.dsp.exec_cmd(launchPrefix .. CALCULATOR))
hl.bind("XF86Calculator",           hl.dsp.exec_cmd(launchPrefix .. CALCULATOR))
hl.bind(mainMod .. " + W",          hl.dsp.exec_cmd(launchPrefix .. BROWSER))
hl.bind("CONTROL + SHIFT + Escape", hl.dsp.exec_cmd(launchPrefix .. TERMINAL .. " -e btop"))

---------------------------------------
---- 4. WORKSPACES & MONITORS --------
---------------------------------------

-- Direct switch to workspaces 1-6
for i = 1, 6 do
    local digit = digitCode(i)
    hl.bind(mainMod .. " + " .. digit, hl.dsp.focus({ workspace = i }))
    hl.bind(mainMod .. " + SHIFT + " .. digit, hl.dsp.window.move({ workspace = i }))
    hl.bind(mainMod .. " + ALT + " .. digit, hl.dsp.window.move({ workspace = i, follow = false }))
end

-- Focus monitors by index with SUPER + CONTROL + 1..3
for i = 1, 3 do
    hl.bind(mainMod .. " + CONTROL + " .. digitCode(i), hl.dsp.focus({ monitor = tostring(i - 1) }))
end

-- Relative workspace navigation
hl.bind(mainMod .. " + CONTROL + Right",       hl.dsp.focus({ workspace = "m+1" }))
hl.bind(mainMod .. " + CONTROL + Left",        hl.dsp.focus({ workspace = "m-1" }))
hl.bind(mainMod .. " + CONTROL + Down",        hl.dsp.focus({ workspace = "emptym" }))
hl.bind(mainMod .. " + mouse_down",           hl.dsp.focus({ workspace = "m-1" }))
hl.bind(mainMod .. " + mouse_up",             hl.dsp.focus({ workspace = "m+1" }))
hl.bind(mainMod .. " + CONTROL + mouse_up",   hl.dsp.focus({ workspace = "m-1" }))
hl.bind(mainMod .. " + CONTROL + mouse_down", hl.dsp.focus({ workspace = "m+1" }))

-- Special workspace (scratchpad)
hl.bind(mainMod .. " + SHIFT + S", hl.dsp.window.move({ workspace = "special" }))
hl.bind(mainMod .. " + S",         hl.dsp.workspace.toggle_special())

---------------------------------------------
---- 5. HARDWARE CONTROLS (AUDIO/DISPLAY) ---
---------------------------------------------

-- Audio Volume & Mute (SwayOSD)
hl.bind("XF86AudioRaiseVolume", hl.dsp.exec_cmd("swayosd-client --output-volume raise"), { locked = true, repeating = true })
hl.bind("XF86AudioLowerVolume", hl.dsp.exec_cmd("swayosd-client --output-volume lower"), { locked = true, repeating = true })
hl.bind("XF86AudioMute",        hl.dsp.exec_cmd("swayosd-client --output-volume mute-toggle"), { locked = true })
hl.bind("XF86AudioMicMute",     hl.dsp.exec_cmd("swayosd-client --input-volume mute-toggle"),  { locked = true })

-- Media Player Controls
hl.bind("XF86AudioPlay",  hl.dsp.exec_cmd("playerctl play-pause"), { locked = true })
hl.bind("XF86AudioPause", hl.dsp.exec_cmd("playerctl play-pause"), { locked = true })
hl.bind("XF86AudioNext",  hl.dsp.exec_cmd("playerctl next"),       { locked = true })
hl.bind("XF86AudioPrev",  hl.dsp.exec_cmd("playerctl previous"),   { locked = true })

-- Display Brightness (SwayOSD)
hl.bind("XF86MonBrightnessUp",   hl.dsp.exec_cmd("swayosd-client --brightness raise"), { locked = true, repeating = true })
hl.bind("XF86MonBrightnessDown", hl.dsp.exec_cmd("swayosd-client --brightness lower"), { locked = true, repeating = true })

-----------------------------------
---- 6. UTILITIES & SCREENSHOTS ---
-----------------------------------

-- Color picker
hl.bind(mainMod .. " + P", hl.dsp.exec_cmd("hyprpicker -a -n"))

-- Screenshots (hypr-screenshot)
hl.bind("Print",         hl.dsp.exec_cmd(launchPrefix .. home .. "/.local/bin/hypr-screenshot area"))
hl.bind("SHIFT + Print", hl.dsp.exec_cmd(launchPrefix .. home .. "/.local/bin/hypr-screenshot fullscreen"))

-- Zoom
local function zoomfunction(value)
    local zoomvalue = hl.get_config("cursor:zoom_factor") or 1.0
    local new_val = math.max(1.0, math.min(3.0, zoomvalue + value))
    hl.config({ cursor = { zoom_factor = new_val } })
end
hl.bind(mainMod .. " + Minus",   function() zoomfunction(-0.3) end, { repeating = true })
hl.bind(mainMod .. " + Plus",    function() zoomfunction(0.3) end,  { repeating = true })
hl.bind(mainMod .. " + code:82", function() zoomfunction(-0.3) end, { repeating = true })
hl.bind(mainMod .. " + code:86", function() zoomfunction(0.3) end,  { repeating = true })
