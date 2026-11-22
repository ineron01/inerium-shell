# Inerium Shell Development Guidelines

Guidelines and architectural principles for working on the Inerium Shell desktop environment and Rust workspace.

---

## 1. Hyprland Lua Configuration Guidelines

### Architecture & Layout
- Entry point: `config/hypr/hyprland.lua`
- Core Modules: `config/variables.lua`, `config/autostart.lua`, `config/input.lua`, `config/keybinds.lua`, `config/monitors.lua`, `config/workspaces.lua`, `config/windowrules.lua`, `config/decorations.lua`, `config/animations.lua`, `config/misc.lua`
- Theme: `config/colors.lua` (Inerium Dragon palette)

### Architecture & Design Principles
1. **Public Repo Anonymization (Universal Defaults):**
   - Files in `inerium-shell` MUST remain completely hardware-agnostic and anonymized.
   - `monitors.lua` uses universal wildcard output: `hl.monitor({ output = "", mode = "preferred", position = "auto", scale = 1.0 })`.
   - `workspaces.lua` defines generic persistent workspaces 1..10.
   - Host-specific monitor models, refresh rates (e.g. `eDP-2` 144Hz), offsets, and multi-monitor layouts belong strictly in the local workstation config (`~/.config/hypr/config/`), never committed into the universal public repository.

2. **Formatting & Line Layout Standards:**
   - **Multi-line layout:** Used in `monitors.lua` and `input.lua` (vertical alignment of properties for rapid inspection and editing).
   - **Single-line compact layout:** Used in `windowrules.lua` (dense listing for fast visual scanning).
   - **Dynamic Workspaces:** `workspaces.lua` reads the configured workspace count from `~/.config/hypr/config/workspaces_count` and generates persistent rules in a concise loop (`for i = 1, count do ... end`).

3. **Clean Module Separation & Safe User Overrides:**
   - Never query `/sys/class/drm/` or run blocking shell commands (`io.popen`, `os.execute`) inside Lua configs.
   - User custom overrides belong strictly in `~/.config/hypr/config/user.lua` (loaded safely at the end of `hyprland.lua` via `pcall(require, "config.user")`), which is preserved across installer updates. Legacy fragmented `*_local.lua` overlay files are eliminated in favor of direct declarative configs and `user.lua`.

4. **Mandatory Engineering Rules:**
   - **Pre-flight Syntax Validation:** Always run `luac -p <file>` before reloading.
   - **Reload Command:** Reload using `hyprctl reload`.
   - **Application Launcher Prefix:** Prefix all graphical app bindings with `uwsm app --` for proper systemd session cgroups.

---

## 2. Inerium Shell Daemon (`crates/inerium-shell`)

### Architecture
- GTK4 Layer-Shell daemon managing popups (Control Center, Audio, Network, BT, Media, Power).
- IPC Client: `inerium-popup` sends commands via Unix domain socket.
- Service: Managed via `inerium-popups.service` (`systemctl --user`).

### Engineering Guidelines
1. **SafeZone & Dismissal:** Retain pointer motion SafeZone tracking to ensure smooth dismissal without flickering.
2. **Dimension Constraints:** Popups must enforce maximum heights with `GtkScrolledWindow` to prevent stretching outside display bounds on high device counts.
3. **Quality & Validation:**
   ```bash
   cargo check --workspace
   cargo clippy --workspace -- -D warnings
   ```
4. **Reload Procedure:** Deploy binary to `~/.local/bin/inerium-shell` and run `systemctl --user restart inerium-popups`.

---

## 3. Inerium Tools (`crates/inerium-tools`)

### Architecture & Codebase
- Crate Type: Cargo workspace targeting lightweight native telemetry binaries and multicall dispatcher.
- Core Library: `libinerium_tools` (shared telemetry, IPC, Waybar formatters, hardware abstractions).
- Multicall Dispatcher: `inerium-ctl` — Single optimized binary handling commands via `argv[0]` symlink or explicit CLI subcommands.
- Multicall Symlinks (15 tools): `inerium-battery`, `inerium-bluetooth`, `inerium-clock`, `inerium-dnd`, `inerium-hotspot`, `inerium-hwmon`, `inerium-layout`, `inerium-network`, `inerium-updates`, `inerium-clipboard`, `inerium-popup`, `hypr-ipc-bridge`, `inerium-osd-watcher`, `inerium-theme`, `inerium-wallpaper`.
- Hardware Regulator: `msi-fan-regulator` — Hardware-specific thermal curve regulator daemon built conditionally under `--features msi-ec`.

### Engineering & Output Protocol
1. **Waybar JSON Schema:** Every `--waybar` output MUST be valid compact JSON adhering to:
   `{"text": "...", "tooltip": "...", "class": "...", "percentage": 0}`
2. **Non-blocking Execution:** Never perform unbounded I/O, heavy polling, or sleeping on the main execution thread. Use async tasks or event sockets.
3. **Quality & Lint Gate:** Always ensure clean compilation before deploy:
   ```bash
   cargo check --workspace
   cargo clippy --workspace -- -D warnings
   ```
4. **Deployment:** Atomically copy `inerium-ctl` (and `msi-fan-regulator` if `--features msi-ec` is enabled) into `~/.local/bin/` and establish symlinks for all 15 applets pointing to `inerium-ctl`.
