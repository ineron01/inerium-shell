# Hyprland Lua Configuration Guidelines

## Architecture & Layout
- Entry point: `hyprland.lua`
- Core Modules: `config/variables.lua`, `config/autostart.lua`, `config/input.lua`, `config/keybinds.lua`, `config/monitors.lua`, `config/workspaces.lua`, `config/windowrules.lua`, `config/decorations.lua`, `config/animations.lua`, `config/misc.lua`
- Theme: `config/colors.lua` (Inerium Dragon palette)

## Clean Architecture & Design Principles
1. **Public Repo Anonymization (Universal Defaults):**
   - Files in `inerium-shell` MUST remain completely hardware-agnostic and anonymized.
   - `monitors.lua` uses universal wildcard output: `hl.monitor({ output = "", mode = "preferred", position = "auto", scale = 1.0 })`.
   - `workspaces.lua` defines generic persistent workspaces 1..10.
   - Host-specific monitor models, refresh rates (e.g. `eDP-2` 144Hz), offsets (`0x216`), and HDMI setups belong strictly in the local workstation config (`~/.config/hypr/config/`), never committed into the universal public repository.

2. **Formatting & Line Layout Standards:**
   - **Multi-line layout:** Used in `monitors.lua` and `input.lua` (vertical alignment of properties for rapid inspection and editing).
   - **Single-line compact layout:** Used in `windowrules.lua` and `workspaces.lua` (dense listing for fast visual scanning).
   - **Explicit numbers (No loops):** Workspaces must be defined as explicit integer rules (`workspace = 1`, `workspace = 2`, etc.). Never write `for ws = 1, N` loops or `tostring(ws)` boilerplate, which can cause rule overwriting.

3. **No Sysfs Scraping or Redundant Proxy Checks:**
   - Never query `/sys/class/drm/` or run blocking shell commands (`io.popen`, `os.execute`) inside Lua configs.
   - Do not wrap declarations in dead `pcall(require, "config.*_local")` checks when files are cleanly separated. Let Hyprland's native engine evaluate rules.

4. **Mandatory Engineering Rules:**
   - **Pre-flight Syntax Validation:** Always run `luac -p <file>` before reloading.
   - **Reload Command:** Reload using `hyprctl reload`.
   - **Application Launcher Prefix:** Prefix all graphical app bindings with `uwsm app --` for proper systemd session cgroups.
