# Inerium Shell

<p align="center">
  <a href="https://github.com/ineron01/inerium-shell/actions/workflows/ci.yml"><img src="https://github.com/ineron01/inerium-shell/actions/workflows/ci.yml/badge.svg" alt="CI Quality Gate"></a>
  <a href="https://github.com/ineron01/inerium-shell/releases/latest"><img src="https://img.shields.io/github/v/release/ineron01/inerium-shell?color=blue&label=Release" alt="Latest Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-GPL--3.0--or--later-green.svg" alt="License: GPL-3.0"></a>
  <a href="https://archlinux.org"><img src="https://img.shields.io/badge/Arch%20Linux%20%2F%20CachyOS-Supported-1793d1?logo=archlinux" alt="Arch Linux / CachyOS"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-2024%20Edition-dea584?logo=rust" alt="Rust 2024 Edition"></a>
</p>

[English](README.md) | [Русский](README.ru.md)

Universal CachyOS / Arch Linux desktop environment built around **Hyprland (Lua API)**, **Waybar**, **Rofi**, **Mako**, **SwayOSD**, **Dolphin KIO**, and a native **Rust / GTK4** utility suite themed in the **Inerium Dragon** palette.

![Inerium Desktop Preview](assets/preview/desktop.png)

---

## Repository Structure

```
inerium-shell/
├── assets/                  # Default wallpapers, preview showcases, and greeter background
├── config/                  # Symlinkable desktop configurations (~/.config/*)
├── crates/                  # Unified Rust workspace suite
│   ├── dolphin-converter/   # Fast document/media converter CLI with ONNX background removal
│   ├── inerium-camera/      # High-performance virtual webcam manager (Android to /dev/video0)
│   ├── inerium-greeter/     # Lightweight GTK4 login screen and session picker for greetd
│   ├── inerium-hw/          # Dynamic sysfs hardware discovery & battery/thermal telemetry
│   ├── inerium-i18n/        # Bilingual localization catalog (Russian / English)
│   ├── inerium-polkit/      # Native GTK4/Relm4 Polkit authentication agent
│   ├── inerium-setup/       # GTK4 onboarding wizard, display calibration & sign-ins
│   ├── inerium-shell/       # GTK4 / Layer Shell control center and Relm4 popups
│   └── inerium-tools/       # Waybar telemetry daemons, IPC bridges & cooling regulator
├── docs/                    # Illustrated guides, keybindings, and module runbooks
├── first-login.d/           # Onboarding sign-in steps (GitHub CLI, Google Drive rclone)
├── local/                   # User executables (~/.local/bin/) and Dolphin KIO servicemenus
├── scripts/install/         # Modular installation phases (00-helpers through 70-summary)
├── system/                  # System-wide udev rules, greetd configs, and systemd units
├── themes/                  # Base palettes (Kanagawa Dragon, Inerium Light) & themes/extra/ pool
├── bootstrap.sh             # One-command TTY bootstrap installer
└── install.sh               # Universal idempotent orchestrator (--bin, --from-source)
```

---

## Installation & Quick Start

### 1. One-Line Bootstrap (Recommended)

Works identically from a **fresh TTY** or **inside any existing desktop environment** (KDE Plasma, GNOME, etc.):

**Short URL (via GitHub Pages):**
```bash
curl -sL ineron01.github.io/inerium-shell | bash
```

**Direct GitHub Raw URL (Recommended):**
```bash
bash -c "$(curl -fsSL https://raw.githubusercontent.com/ineron01/inerium-shell/main/bootstrap.sh)"
```

- **Interactive TUI:** Auto-detects hardware (GPU, laptop battery/chassis, existing Display Manager) and offers quick choices.
- **Fast prebuilt binaries (Default):** Downloads compact release archives (~6 MB) directly from GitHub Releases, skipping heavy Cargo compilation.
- **Smart Fallback:** Automatically falls back to local source build if dynamic shared libraries mismatch.
- **Preserves existing DE:** When installed on top of KDE/GNOME, it sets Inerium (`Hyprland (UWSM-Waybar)`) as the default session in `greetd` while keeping KDE Plasma untouched and available to launch.

### 2. Silent / Automated Run (for scripts or SSH)

```bash
bash -c "$(curl -fsSL https://raw.githubusercontent.com/ineron01/inerium-shell/main/bootstrap.sh)" -- -y
```

### 3. Source Compilation & Developer Mode

Clones directly into `~/inerium-shell` (visible in your home directory) and compiles via Cargo:

```bash
bash -c "$(curl -fsSL https://raw.githubusercontent.com/ineron01/inerium-shell/main/bootstrap.sh)" -- --from-source
```

> **Target Paths & Overrides:**
> - Standard prebuilt bootstrap installs to `$XDG_DATA_HOME/inerium/shell` (default: `~/.local/share/inerium/shell`, hidden to keep `$HOME` clean).
> - Source compilation installs to `~/inerium-shell` (visible for easy access to source code).
> - Configuration files: Deployed as real files into `$XDG_CONFIG_HOME` (`~/.config/{hypr,waybar,kitty,rofi,mako,uwsm,gtk-3.0,gtk-4.0}`) and tracked via manifest (`~/.local/state/inerium/install-manifest`), eliminating fragile directory symlinks. User customizations are preserved on upgrade; pass `--update-configs` to apply updates with automatic `.bak.<timestamp>` backups.
> - Themes & Palettes: Shipped base themes reside in `~/.config/inerium/themes/` (`kanagawa-dragon.toml`, `inerium-light.toml`, `template.toml.example`); extra themes pool in `~/.local/share/inerium/themes/extra/`.
> - Binaries: Installed to `~/.local/bin/` (and `/usr/local/bin/inerium-greeter`).
> - You can override the clone path anytime with `INERIUM_SHELL_DIR=/custom/path`.

### 4. Local Run from Cloned Repo

```bash
git clone https://github.com/ineron01/inerium-shell.git ~/inerium-shell
cd ~/inerium-shell
./install.sh --bin --with-greeter --with-gdrive
```

---

## Documentation & Guides

- **[Hotkeys, Gestures & Waybar Tutorial](docs/keybindings.md)** ([Русская версия](docs/keybindings.ru.md)) — Complete guide for system navigation, window tiling, shortcuts, touchpad gestures, and Waybar interactive features.
- **[Optional Modules & Add-ons Guide](docs/addons.md)** ([Русская версия](docs/addons.ru.md)) — Detailed documentation for Inerium Camera, MSI Cooling (`msi-ec` DKMS + fan regulator), and noVPN split-tunneling.
- **[Android Phone Webcam Setup](docs/camera-setup.md)** ([Русская версия](docs/camera-setup.ru.md)) — One-time ADB setup instructions for using a smartphone as a Linux webcam.

---

## Optional System Add-ons & Installer Options

Inerium Shell provides optional system extensions that can be selected in the installer TUI or passed via CLI flags:

### Add-on Flags
- `--with-gdrive`: **Google Drive Integration** — Background `rclone` user service mount for cloud storage access in Dolphin.
- `--with-converter`: **Dolphin Converter** — Context menu audio, video, document, and PDF converter with optional AI background removal (`silueta.onnx`).
- `--with-camera`: **Inerium Camera** — Turns an Android phone into a high-definition webcam (`/dev/video0`) with GTK4 GUI, dynamic switching (back/front/laptop), and portrait/landscape support.
- `--with-msi-ec`: **MSI Cooling & Fan Regulator** — Upstream `msi-ec-dkms-git` kernel driver + dynamic CPU/GPU thermal curve regulator daemon (`msi-fan-regulator`).
- `--with-novpn`: **noVPN Split-Tunneling** — Run specific applications directly through physical network interfaces bypassing any active VPN.
- `--all-addons`: Installs all optional add-ons together.

### Configuration & Lifecycle Flags
- `--update`: Update an existing installation (preserves installed add-on modules and configuration without repeating initial setup questions).
- `--sysupgrade`, `-Syu`: Perform a full rolling OS upgrade (`pacman -Syu`) alongside the Inerium Shell update (default update mode is targeted and fast, updating only Inerium and missing dependencies).
- `--bin`: Install prebuilt binaries from GitHub Releases (default).
- `--from-source`: Compile Rust workspace locally via Cargo.
- `--with-greeter`: Install and configure `greetd` + `inerium-greeter` display manager.
- `--lang [en|ru]`: Set installer and system language preference.
- `--workspaces N`: Workspaces per monitor (1-9, default: 3).
- `--fetch-model`: Download `silueta.onnx` background removal AI model (~43 MB) if missing.
- `--update-configs`: Update configuration files while preserving modified user files as `.bak.<timestamp>`.
- `--check-abi`: Validate runtime dynamic library dependencies (`ldd`) for all installed binaries.
- `--prune`: Prune obsolete files against previous install manifest.
- `--dry-run`: Print planned actions without modifying the system.
- `--non-interactive`, `-y`: Run unattended without interactive TUI prompts.
- `--uninstall`: **Clean Uninstaller** — Disables registered user/system services, removes all deployed symlinks, binaries, and templates based on the installation manifest (`~/.local/state/inerium/install-manifest`), and removes `~/.local/share/inerium-shell` (leaving any custom developer paths untouched).

---

## First-Login Onboarding (`inerium-setup`)

On first login, `inerium-setup` launches an interactive GTK4 wizard allowing you to:
- Adjust display scaling, resolution, and refresh rate.
- Select your preferred wallpaper and layout preferences.
- Connect GitHub CLI and Google Drive via integrated terminal widgets.

Re-run anytime from the application launcher or by running `inerium-setup`.


---

## Theme Engine & Custom Themes

Inerium Desktop includes a declarative, Single Source of Truth theme compiler and switcher (`inerium-theme`).

### Out-of-the-Box Themes
- **Inerium Light (`inerium-light`)**: Clean, modern neutral light theme with crisp contrast, white card surfaces (`#ffffff`), deep slate text (`#1e293b`), and standard 6500K daylight display temperature.
- **Kanagawa Dragon (`kanagawa-dragon`)**: Deep sumi-ink dark theme with warm dragon tones and subdued accents.

### Adaptive Control Center
- **2 Themes (Default Out-of-the-Box)**: Clicking the **Theme** tile in the Control Center instantly toggles between dark and light modes.
- **>2 Themes (Custom User Themes)**: As soon as you add custom themes to `~/.config/inerium/themes/`, clicking the tile opens the interactive **Theme Gallery** with theme cards, author attribution, color swatches, and active badges. Right-clicking the tile always performs a quick dark ↔ light toggle.

### Adding Custom Themes
Save your custom theme as a TOML file in `~/.config/inerium/themes/<id>.toml`:

```toml
[theme]
id = "my-custom-theme"
name = "My Custom Theme"
author = "Your Name"
upstream = "https://github.com/your-username/my-theme"
variant = "dark" # or "light"
license = "MIT"
description = "A short description of your theme palette"

[palette]
bg_canvas         = "rgba(24, 22, 22, 0.97)"
bg_surface        = "#221f1f"
bg_surface_alt    = "#1c1a1a"
bg_surface_hover  = "#2d2a2a"
bg_surface_active = "#363434"

border_subtle     = "rgba(197, 201, 197, 0.10)"
border_active     = "rgba(197, 201, 197, 0.25)"

text_primary      = "#c5c9c5"
text_heading      = "#e6e2cc"
text_muted        = "#737c73"
text_subtle       = "#a6a69c"
text_inverted     = "#181616"

accent_green      = "#87a987"
accent_green_bg   = "rgba(135, 169, 135, 0.16)"
accent_green_border = "rgba(135, 169, 135, 0.35)"

accent_blue       = "#8ba4b0"
accent_blue_bg    = "rgba(139, 164, 176, 0.16)"
accent_blue_border = "rgba(139, 164, 176, 0.38)"

accent_amber      = "#c4b28a"
accent_amber_bg   = "rgba(196, 178, 138, 0.15)"
accent_amber_border = "rgba(196, 178, 138, 0.38)"

accent_red        = "#c4746e"
accent_red_bg     = "rgba(196, 116, 110, 0.15)"
accent_red_border = "rgba(196, 116, 110, 0.38)"

accent_aqua       = "#8ea4a2"
accent_aqua_bg    = "rgba(142, 164, 162, 0.16)"
accent_aqua_border = "rgba(142, 164, 162, 0.38)"

accent_purple     = "#8992a7"
accent_purple_bg  = "rgba(137, 146, 167, 0.16)"
accent_purple_border = "rgba(137, 146, 167, 0.38)"

shadow            = "rgba(0, 0, 0, 0.85)"
modal_backdrop    = "rgba(18, 16, 16, 0.48)"
slider_trough     = "rgba(197, 201, 197, 0.10)"
slider_thumb      = "#e6e2cc"

[display]
temperature = 6500 # Screen temperature in Kelvins (6500 for neutral daylight, 5500 for warm paper)
```

### Extra Themes Pool (`themes/extra/`)
In addition to the two primary base themes, Inerium includes a pool of popular community palettes ready for one-click activation in `inerium-setup` or installation via `inerium-theme install <id>` (which permanently promotes the theme into `~/.config/inerium/themes/`):
- **Kanagawa Lotus (`kanagawa-lotus`)**: Warm unbleached aged-paper aesthetic (by rebelot) with automatic 5500K soft display temperature.
- **Kanagawa Wave (`kanagawa-wave`)**: Classic oceanic dark palette (by rebelot).
- **Tokyo Night (`tokyo-night`)**: Clean neon cyberpunk aesthetic (by folke).
- **Catppuccin Mocha (`catppuccin-mocha`)**: Soothing dark pastel palette.
- **Catppuccin Latte (`catppuccin-latte`)**: Warm light pastel palette.
- **Nord (`nord`)**: Arctic, north-bluish clean palette.
- **Gruvbox Dark (`gruvbox-dark`)**: Retro groove warm dark palette.
- **Dracula (`dracula`)**: Iconic gothic dark contrast palette.

A template for crafting your own theme is available at `~/.config/inerium/themes/template.toml.example`.

### CLI Commands
- `inerium-theme list` — List active themes (`--all` to list all including extra, `--json` for machine output).
- `inerium-theme set <id>` — Apply a theme by ID.
- `inerium-theme install <id>` — Move an extra theme into permanent user themes (`~/.config/inerium/themes/`) and apply it.
- `inerium-theme toggle` — Toggle between the primary dark and light themes.
- `inerium-theme status` — Output active theme metadata in JSON.

### User Hooks (`~/.config/inerium/hooks.d/`)
Any executable placed in `~/.config/inerium/hooks.d/` runs automatically after a theme switch, receiving the active theme variant (`dark` or `light`) and ID as arguments.
