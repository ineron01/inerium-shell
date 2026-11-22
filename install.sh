#!/usr/bin/env bash
# shellcheck disable=SC2034 # State and configuration variables are consumed across sourced scripts/install/*.sh
# ============================================================================
# inerium-shell/install.sh — Universal Arch/CachyOS Hyprland + Rust Installer
# ============================================================================
set -euo pipefail

export PATH="$HOME/.local/bin:$PATH"

REPO_DIR="${INERIUM_SHELL_DIR:-${INERIUM_DESKTOP_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)}}"
SCRIPTS_DIR="$REPO_DIR/scripts/install"

# Installation state & flags
DRY_RUN=0
SKIP_PKGS=0
SKIP_BUILD=0
FETCH_MODEL=0
WITH_GREETER=0
DO_UNINSTALL=0
CHECK_ABI=0
UPDATE_CONFIGS=0
UPDATE_CONFIGS_MODE="none"
IS_UPDATE=0
USE_ROLLING=0
NON_INTERACTIVE="${INERIUM_NON_INTERACTIVE:-0}"
INSTALL_MODE="bin"
MODE_EXPLICIT=0
WORKSPACES_PER_MON=""
BACKUP_STAMP="$(date +%Y%m%d%H%M%S)"
LOG_FILE="/tmp/inerium-install-${UID}.log"
WITH_CAMERA=0
WITH_MSI_EC=0
WITH_FAN_REGULATOR=0
WITH_NOVPN=0
WITH_CONVERTER=0
WITH_GDRIVE=0
WITH_SYSUPGRADE=0
WITH_GITHUB=0
RESTORE_ASSETS=0
INSTALL_LANG=""
EXPLICIT_LANG=0
MSI_EC_REBUILT=0
ALL_ADDONS_REQUESTED=0
RELEASE_DIST_DIR=""

RELEASE_BASE_URL="${INERIUM_RELEASE_URL:-https://github.com/ineron01/inerium-shell/releases/latest/download}"
ROLLING_BASE_URL="${INERIUM_ROLLING_URL:-https://github.com/ineron01/inerium-shell/releases/download/rolling}"
BIN_ARCHIVE="inerium-binaries-linux-x86_64.tar.gz"
BIN_SHA="inerium-binaries-linux-x86_64.tar.gz.sha256"

SILUETA_URL="https://github.com/danielgatis/rembg/releases/download/v0.0.0/silueta.onnx"
SILUETA_PATH="${XDG_DATA_HOME:-$HOME/.local/share}/inerium/models/silueta.onnx"

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Options:
  --bin              Install prebuilt binaries from GitHub Releases (default)
  --rolling          Install prebuilt binaries from rolling release channel
  --from-source      Compile Rust workspace locally via Cargo
  --dry-run          Print planned actions without modifying the system
  --skip-pkgs        Skip paru package installation
  --skip-build       Skip binary download and Cargo compilation
  --fetch-model      Download silueta.onnx (~43 MB) if missing
  --with-greeter     Install and configure greetd + inerium-greeter (/etc/greetd/)
  --with-gdrive      Install Google Drive integration for Dolphin via rclone
  --with-converter   Install Dolphin Converter (LibreOffice, Pandoc, ONNX background removal)
  --with-camera      Install Inerium Camera (Android phone webcam via USB/Wi-Fi)
  --with-msi-ec      Install patched msi-ec DKMS kernel module (for MSI laptops)
  --with-fan-regulator Install msi-fan-regulator daemon and fan curves (requires MSI hardware)
  --with-novpn       Install noVPN split-tunneling service and helper scripts
  --with-github      Install GitHub CLI and enable first-login authentication step
  --all-addons       Install all optional add-ons (--with-gdrive, --with-converter, --with-camera, --with-novpn, --with-github, and --with-msi-ec if MSI hardware)
  --lang [en|ru]     Set installer language (en or ru)
  --workspaces N     Workspaces per monitor (1-9, default: 3)
  --update           Update existing installation (keeps installed modules, pulls latest binaries)
  --restore-assets   Restore missing standard wallpapers and themes (on update)
  --sysupgrade, -Syu Perform full system upgrade (pacman -Syu) alongside Inerium update
  --update-ui        Update UI & shell configurations only (Waybar, Rofi, themes, animations; keeps keybinds & rules)
  --update-configs[=all|ui]
                     Update configuration files (all: full update with .bak; ui: UI/shell only)
  --check-abi        Check dynamic library ABI health for installed binaries
  --uninstall        Uninstall Inerium Shell and remove tracked files/services
  --prune            Prune obsolete files against previous install manifest
  --non-interactive  Run without interactive TUI prompts (also -y)
  -h, --help         Show this help
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --bin)
            if [[ "$MODE_EXPLICIT" -eq 1 && "$INSTALL_MODE" == "source" ]]; then
                echo "Cannot combine --bin and --from-source." >&2; exit 1
            fi
            INSTALL_MODE="bin"; MODE_EXPLICIT=1; shift ;;
        --rolling)
            USE_ROLLING=1; shift ;;
        --from-source)
            if [[ "$MODE_EXPLICIT" -eq 1 && "$INSTALL_MODE" == "bin" ]]; then
                echo "Cannot combine --bin and --from-source." >&2; exit 1
            fi
            INSTALL_MODE="source"; MODE_EXPLICIT=1; shift ;;
        --dry-run)         DRY_RUN=1; NON_INTERACTIVE=1; shift ;;
        --update)          IS_UPDATE=1; shift ;;
        --restore-assets|--restore-defaults) RESTORE_ASSETS=1; shift ;;
        --sysupgrade|-Syu) WITH_SYSUPGRADE=1; shift ;;
        --uninstall)       DO_UNINSTALL=1; NON_INTERACTIVE=1; shift ;;
        --check-abi)       CHECK_ABI=1; NON_INTERACTIVE=1; shift ;;
        --update-ui|--update-configs=ui) UPDATE_CONFIGS=1; UPDATE_CONFIGS_MODE="ui"; shift ;;
        --update-configs=all|--update-configs) UPDATE_CONFIGS=1; UPDATE_CONFIGS_MODE="all"; shift ;;
        --prune|--clean)   shift ;;
        --skip-pkgs)       SKIP_PKGS=1; shift ;;
        --skip-build)      SKIP_BUILD=1; shift ;;
        --fetch-model)     FETCH_MODEL=1; shift ;;
        --with-greeter)    WITH_GREETER=1; shift ;;
        --with-gdrive)     WITH_GDRIVE=1; shift ;;
        --with-converter)  WITH_CONVERTER=1; FETCH_MODEL=1; shift ;;
        --with-camera)     WITH_CAMERA=1; shift ;;
        --with-msi-ec)     WITH_MSI_EC=1; shift ;;
        --with-fan-regulator) WITH_FAN_REGULATOR=1; WITH_MSI_EC=1; shift ;;
        --with-novpn)      WITH_NOVPN=1; shift ;;
        --with-github)     WITH_GITHUB=1; shift ;;
        --all-addons)      WITH_CONVERTER=1; FETCH_MODEL=1; WITH_GDRIVE=1; WITH_CAMERA=1; WITH_NOVPN=1; WITH_GITHUB=1; ALL_ADDONS_REQUESTED=1; shift ;;
        --lang)
            if [[ $# -lt 2 || -z "$2" || ( "$2" != "en" && "$2" != "ru" ) ]]; then
                echo "Invalid --lang value (expected 'en' or 'ru')." >&2; exit 1
            fi
            INSTALL_LANG="$2"; EXPLICIT_LANG=1; shift 2 ;;
        --workspaces)
            if [[ $# -lt 2 || ! "$2" =~ ^[1-9]$ ]]; then
                echo "Invalid --workspaces value (expected 1-9)." >&2; exit 1
            fi
            WORKSPACES_PER_MON="$2"; shift 2 ;;
        --non-interactive|-y) NON_INTERACTIVE=1; shift ;;
        -h|--help)         usage; exit 0 ;;
        *)                 echo "Unknown option: $1" >&2; usage; exit 1 ;;
    esac
done

if [[ -n "$WORKSPACES_PER_MON" && ! "$WORKSPACES_PER_MON" =~ ^[1-9]$ ]]; then
    echo "Invalid --workspaces value '$WORKSPACES_PER_MON' (expected 1-9)." >&2
    exit 1
fi

# ============================================================================
# Source modular stages
# ============================================================================
source "$SCRIPTS_DIR/00-helpers.sh"
source "$SCRIPTS_DIR/05-manifest.sh"
source "$SCRIPTS_DIR/10-detect.sh"

if [[ "$ALL_ADDONS_REQUESTED" -eq 1 ]]; then
    if detect_is_msi_hardware; then
        WITH_MSI_EC=1
    else
        WITH_MSI_EC=0
    fi
fi

source "$SCRIPTS_DIR/20-tui.sh"
source "$SCRIPTS_DIR/30-packages.sh"
source "$SCRIPTS_DIR/40-binaries.sh"
source "$SCRIPTS_DIR/50-configs.sh"
source "$SCRIPTS_DIR/60-addons.sh"
source "$SCRIPTS_DIR/70-summary.sh"

export -f ensure_paru install_pkgs install_pkglist install_from_binaries install_root_if_changed download_file warn log run_live_step
export RELEASE_BASE_URL ROLLING_BASE_URL USE_ROLLING BIN_ARCHIVE BIN_SHA USER_BINS_STR DRY_RUN INSTALL_MODE SKIP_BUILD UPDATE_CONFIGS UPDATE_CONFIGS_MODE RESTORE_ASSETS IS_UPDATE WITH_GITHUB RELEASE_DIST_DIR EXPLICIT_LANG

# ============================================================================
# Main Execution Pipeline
# ============================================================================
check_not_root
init_log_file

cleanup_all_on_exit() {
    restore_terminal
    cleanup_manifest_tmp
    if [[ -n "${RELEASE_DIST_DIR:-}" && -d "$RELEASE_DIST_DIR" ]]; then
        rm -rf "$RELEASE_DIST_DIR"
    fi
    if [[ -n "${INERIUM_SUDO_KEEPALIVE:-}" ]]; then
        kill "$INERIUM_SUDO_KEEPALIVE" 2>/dev/null || true
    fi
}
trap cleanup_all_on_exit EXIT

if [[ "$DO_UNINSTALL" -eq 1 ]]; then
    run_uninstall
    exit 0
fi

if [[ "$CHECK_ABI" -eq 1 ]]; then
    check_installed_binaries_abi
    exit $?
fi

init_sudo_keepalive
init_manifest

if [[ "$IS_UPDATE" -eq 1 ]]; then
    detect_installed_addons_and_features
    if [[ -z "$INSTALL_LANG" ]]; then
        if [[ -f "$HOME/.config/locale.conf" ]] && grep -q "ru_RU" "$HOME/.config/locale.conf"; then
            INSTALL_LANG="ru"
        elif [[ "${LANG:-}" =~ ^ru || "${LC_ALL:-}" =~ ^ru ]]; then
            INSTALL_LANG="ru"
        else
            INSTALL_LANG="en"
        fi
    fi
fi

ensure_gum_installed
if use_tui; then
    ensure_console_cyrillic_font
    run_tui_prompts
fi

# 1. Base packages (pacman / paru)
log "[1/8] Installing base packages (pacman / paru)..."
install_base_packages

# 2. Rust binaries delivery
log "[2/8] Delivering Inerium binaries..."
deliver_binaries

# 3. User scripts, KIO menus, desktop entries, indexer
log "[3/8] Deploying user scripts and menus..."
deploy_user_scripts_and_menus

# 4. Dotfiles (~/.config), mutable templates, and local hardware overlays
log "[4/8] Installing desktop configs and wallpapers (~/.config)..."
deploy_desktop_configs
detect_and_generate_local_overlays
deploy_wallpapers

# 5. Wayland sessions and login screen (greetd)
log "[5/8] Configuring display manager (greetd)..."
configure_greeter

# 6. Systemd user services and browser defaults
log "[6/8] Enabling systemd user services..."
enable_user_services_and_defaults

# 7. Optional add-ons (Google Drive, Dolphin Converter, MSI Cooling, Camera, noVPN)
log "[7/8] Configuring selected add-on modules..."
deploy_selected_addons

# 8. Record installation manifest and prune obsolete files
log "[8/8] Finalizing install manifest..."
prune_obsolete_and_save_manifest
refresh_desktop_caches

if [[ "$INSTALL_MODE" == "source" && "$SKIP_BUILD" -eq 0 && "$DRY_RUN" -eq 0 ]]; then
    log "Cleaning temporary build cache to free disk space..."
    cargo clean --manifest-path "$REPO_DIR/Cargo.toml" 2>/dev/null || true
fi

# 9. Summary & completion
if [[ "$DRY_RUN" -eq 0 ]]; then
    print_summary
else
    log "Dry-run complete."
fi
