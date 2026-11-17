# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
