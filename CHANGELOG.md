# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.2] - 2026-11-22

### Added
- **Event-Driven Idle Architecture**: Switched `inerium-bluetooth` to `dbus-monitor` signal streaming, `inerium-layout` to blocking `poll(-1)` on Hyprland `socket2`, cached power supply / hwmon sensor paths in `inerium-battery` and `inerium-hwmon`, and moved `inerium-camera` bridge idle waits to `Condvar` + `v4l2loopback` `sustain_framerate=1` with a blocking GStreamer bus.
- **Modular Crate Architecture**: Split monolithic views and models into submodules across `inerium-shell` (`control_center/`, `network/`, `appearance/`), `inerium-setup` (`model/`, `ui/interface/`, `ui/integrations/`), `inerium-hw` (`backlight.rs`, `gpu.rs`), and `inerium-tools` (`desktop/`).
- **Opt-in GitHub CLI & Distro Selector**: Added `--with-github` documentation (`docs/addons*.md`), `inerium distro` Waybar icon switcher, bilingual `Title[ru]:` headers in `first-login.d/*`, live EN/RU switching with preserved status warnings in `inerium-setup`, and dynamic `@cc_*` theming in `inerium-camera`.
- **Theme Alignment & MSI CI Tests**: Added read-only default theme consistency test (`test_align_default_theme_kanagawa_dragon`) and CI test coverage for `inerium-tools --features msi-ec`.

### Changed
- **Responsive GTK4 / Relm4 UI**: Consolidated `wpctl` calls, coalesced `ddcutil` worker queue, and offloaded blocking `init()` probes (Wi-Fi/BT scans, wallpapers, `powerprofilesctl`, `envycontrol`) to background threads with loading placeholders in `inerium-shell`.
- **Default Theme Synchronization**: Aligned bundled `config/*` and Qt5ct/Qt6ct `Inerium-Current.conf` with `themes/kanagawa-dragon.toml` and removed legacy/personal theme and WirePlumber configs.
- **Installer Hardening**: Added `run_live_step` nesting guard, strict `pacman`/`paru` error propagation, and per-UID log isolation (`/tmp/inerium-install-${UID}.log`).

### Fixed
- **Memory & Borrow Safety in `inerium-shell`**: Eliminated Relm4 controller/runtime leaks and potential `RefCell already borrowed` panics via deferred `cleanup_displaced` and `WeakRef` cleanup.
- **`SelfPipe` & Event-Loop Reliability**: Added full drain loop, `EINTR` handling, atomic `compare_exchange` in `Drop`, and `TEST_LOCK` serialization in `SelfPipe` tests; eliminated EOF busy-loop risk in `inerium-bluetooth` and idle subprocess forks in `inerium-layout` and `inerium-bluetooth`.
- **Safe `inerium update` & `--uninstall`**: Protected `~/.config/locale.conf`, user-preserved themes (`RESTORE_ASSETS=0`), and user-generated `google-chrome-direct.desktop` shortcuts from manifest pruning; implemented safe `greetd` disabling/restoration without killing active sessions.
- **Setup & Appearance Fixes**: Fixed `inerium-setup` reset leaving `idle_dirty` set (recreating deleted `hypridle_local.conf` on `Save & Apply`), strict 6-digit HEX parsing in Background Studio, `saved_connections` sync in Network view, and unprivileged user `.desktop` retention in `inerium-desktop-indexer`.

### Security
- **Hyprland IPC Bridge**: Added Lua string escaping and strict command boundary matching in `hypr-ipc-bridge`.
- **Desktop Indexer Privilege Drop**: Executed per-user directory indexing via `run_as_owner` with `mkstemp` atomic writes and symlink protections in `inerium-desktop-indexer`.
- **Greeter Theme & Installer Isolation**: Added symlink checks and `create_new(true)` atomic writes in `inerium-theme`, plus per-UID installer log files.

## [0.2.0] - 2026-11-17

### Added
- **Native Inerium Theme Engine (`inerium-theme`)**: Declarative Single Source of Truth TOML theme schemas, live color compiler, palette harmonization, and dynamic post-switch hook execution in `~/.config/inerium/hooks.d/`.
- **Interactive Appearance View in Control Center**: Unified wallpaper picker with multi-monitor targeting (`all` or per-display), interactive Theme Gallery with author attribution, color swatches, active badges, and live folder openers.
- **Inerium Light (`inerium-light`)**: High-contrast, clean neutral light theme with white card surfaces, deep slate text, and 6500K daylight display harmonization.
- **Procedural Adaptive Greeter & Hyprlock Theming**: Replaced raster greeter wallpapers with dynamic procedural radial gradients derived directly from the theme palette (`/var/tmp/inerium/greeter-theme.css`); added synchronized `hyprlock` palette variables (`~/.config/hypr/hyprlock-colors.conf`).
- **Stateful Installation Manifest & Orphan Pruning (`scripts/install/05-manifest.sh`)**: State tracking in pure Bash TSV, clean `--uninstall` with root/path safety guardrails, and automated `--prune` for obsolete artifacts.
- **Upstream `msi-ec` AUR DKMS Driver Migration**: Upstream packaging support with PR #773; removed deprecated custom in-tree C driver code.
- **Workspace-Integrated Polkit Agent (`crates/inerium-polkit`)**: Native Rust/GTK4 polkit daemon integrated into the root Cargo workspace.
- **Declarative Hyprland Lua Clean Architecture**: Integer workspaces (1..10), single-line window rules, hardware-agnostic repository templates, and strict anonymization.
- **Open Source Standards**: Root `LICENSE` (`GPL-3.0-or-later`), `.editorconfig`, `.gitattributes`, `CONTRIBUTING.md`, and GitHub issue/PR templates.

### Changed
- Standardized CLI tool naming in `local/bin/` (dropping `.sh` extensions).
- Offloaded theme switching in GTK4/Relm4 UI to background worker threads to keep interface interactions responsive.
- Migrated desktop data directory to standard XDG data directory (`$XDG_DATA_HOME/inerium-shell`).
- Sorted display modes in `inerium-setup` numerically by pixel area descending (`w * h`).

### Removed
- Removed legacy fallback session overlays and redundant duplicate backup configurations.
- Removed deprecated Rofi scripts and in-tree `msi-ec` C driver sources.
- Eradicated hardcoded personal paths (`/home/ineron`) across all crates and public configuration surfaces.

### Fixed
- Fixed release packaging workflow `.github/workflows/release.yml` to package `inerium-theme`.
- Fixed potential `RefCell already borrowed` panic in `inerium-greeter` and added early CLI help/version intercept.
- Fixed `MonitorTarget.label` dead code compiler warning and connected monitor icon badges.
- Fixed subfolder and symlink theme discovery in `inerium-theme`.

## [0.1.2] - 2026-11-16

### Added
- Complete migration of all Layer-Shell popups to Relm4 simple components in `inerium-shell`.
- Crate-level documentation and README files across all workspace members.
- Automated CI quality gate `.github/workflows/ci.yml`.

### Changed
- Decoupled `dolphin-converter` and `gdrive` into optional add-on modules in `install.sh`.
- Standardized workspace dependencies in `Cargo.toml`.

## [0.1.1] - 2026-11-15

### Added
- Added prebuilt release packaging workflow `.github/workflows/release.yml` with SHA256 checksums.
- Smart installer fallback from prebuilt binary downloads to local source compilation.
- Inerium Setup onboarding wizard with GTK4 / VTE-4 terminal sign-ins.

## [0.1.0] - 2026-11-12

### Added
- Initial public release of unified Inerium Shell desktop distribution for Arch / CachyOS.
- Hyprland Lua modular configuration, Inerium Dragon Waybar panel, and native telemetry utilities (`inerium-tools`).
