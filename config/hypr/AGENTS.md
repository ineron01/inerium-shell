# Hyprland Lua Configuration Guidelines

## Architecture & Layout
- Entry point: `hyprland.lua`
- Modules: `config/variables.lua`, `config/windowrules.lua`, `config/keybinds.lua`, `config/autostart.lua`, `config/input.lua`
- Theme: `config/colors.lua` (Inerium Dragon palette)

## Mandatory Engineering Rules
1. **Pre-flight Syntax Validation:** Always run `luac -p <file>` before reloading.
2. **Reload Command:** Reload using `hyprctl reload`.
3. **Application Launcher Prefix:** Prefix all graphical app bindings with `uwsm app --` for correct systemd session cgroups.
4. **Monitor Detection:** Never hardcode primary laptop monitor; rely on `detect_drm_monitors()`.
