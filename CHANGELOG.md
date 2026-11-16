# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Root `LICENSE` file under GNU General Public License v3.0 (`GPL-3.0-or-later`).
- `.editorconfig` and `.gitattributes` for uniform workspace style and line ending guarantees.
- `CONTRIBUTING.md` guide and GitHub Issue/PR templates.
- Unified `inerium-polkit` authentication agent into `crates/inerium-polkit` under the root Cargo workspace.
- GitHub Actions dependency caching (`Swatinem/rust-cache@v2`).

### Changed
- Standardized CLI tool naming in `local/bin/`: renamed `inerium-toggle-network.sh` and `inerium-toggle-bluetooth.sh` to remove `.sh` extensions.
- Streamlined fallback session: `autostart_backup.lua` and `binds_backup.lua` now reference primary config surfaces with clean fallbacks.
- Replaced hardcoded fallback user in `system/novpn/bin/novpn-exec` with dynamic user resolution (`$(id -un)`).

### Removed
- Deleted redundant duplicate backup configuration files (`config_backup.rasi`, `clipboard_backup.rasi`, `inerium_backup.rasi`, `config_backup.jsonc`, `style_backup.css`).
- Deleted legacy Rofi scripts (`rofi-audio-sink`, `rofi-bluetooth`, `rofi-power-menu`, `rofi-wifi-menu`, `inerium-popup-backup`, `rofi-power-menu-backup`), superseded by native `inerium-shell` popups.

### Fixed
- Fixed GitHub Actions CI syntax lint error when inspecting Python executables.
- Fixed `clippy::unnecessary-literal-unwrap` lint error in `msi-fan-regulator`.

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
