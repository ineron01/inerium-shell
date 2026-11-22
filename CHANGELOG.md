# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.2] - 2026-11-22

### Added
- **Opt-in GitHub CLI Addon**: Added `--with-github` flag, manifest tracking (`ADDON github`), and optional interactive setup step (`first-login.d/10-github`).
- **Enhanced System Diagnostics (`inerium doctor`)**: Added validation checks for orphaned packages, systemd user/system unit health, and configuration symlink integrity.
- **Qt5ct / Qt6ct & KDE Theming in `inerium-theme`**: Added native generator for Qt5ct and Qt6ct color schemes (`emit_qtct_colors`) and idempotent KDE globals synchronization (`update_kdeglobals`).
- **Atomic Self-Pipe Event Wakeup (`SelfPipe`)**: Added robust non-blocking self-pipe IPC mechanism in `inerium-tools` for waking event loops without thread spin-locks.
- **CI Test Coverage for MSI Fan Regulator**: Added automated `cargo test -p inerium-tools --features msi-ec` in GitHub Actions CI workflow.

### Changed
- **Memory & Runtime Lifecycle Management in `inerium-shell`**: Refactored all 7 Layer-Shell popups to eliminate Relm4 runtime detach leaks by retaining the active controller in the application root component.
- **Optimized Hyprland Layout Manager**: Eliminated repetitive `hyprctl` process spawning in stream mode; event loop now waits directly on IPC file descriptors with zero idle CPU utilization.
- **Asynchronous Bluetooth Device Discovery**: Refactored Bluetooth status tracking to use asynchronous DBus object queries with robust EOF/EINTR/EAGAIN pipe draining.
- **Stabilized Input Overrides**: Reset idle dirty flags on layout reset to prevent runaway Hyprland configuration reload loops.

### Fixed
- **Safe `greetd` Handling in Manifest & Uninstaller**: Implemented safe helper to stop `greetd` only when running outside the target session, correctly preserving and restoring original PAM configs without killing active user sessions.
- **Theme Preservation During Updates**: Ensured user-selected themes, wallpapers, and generated color assets are preserved during `inerium update` (`IS_UPDATE=1`).
- **Race Condition in `SelfPipe` Tests**: Fixed multi-threaded test contention with atomic drop ordering and process-level test locking.
- **Desktop Indexer File Deletion**: Fixed `inerium-desktop-indexer` inadvertently purging user-created `.desktop` files when executed unprivileged.
- **Legacy Orphan Cleanup Safeguards**: Protected `google-chrome-direct.desktop` and user-defined application shortcuts from being purged during manifest orphan cleanup.
- **Persistent `hyprlock-i18n.conf` Tracking**: Fixed configuration rotation to prevent `hyprlock-i18n.conf` from being moved to `.bak` on consecutive updates.

### Security
- **Hardened Systemd Service Units**: Applied strict filesystem and privilege isolation (`ProtectSystem=strict`, `ProtectHome=read-only`, `PrivateTmp=true`) across helper units.
- **PAM Configuration Sanitization**: Ensured atomic file swapping and verified PAM module arguments during greetd configuration generation.

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
