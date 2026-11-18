#!/usr/bin/env bash
# ============================================================================
# inerium-shell/install.sh — Universal Arch/CachyOS Hyprland + Rust Installer
# ============================================================================
set -euo pipefail

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
NON_INTERACTIVE="${INERIUM_NON_INTERACTIVE:-0}"
INSTALL_MODE="bin"
MODE_EXPLICIT=0
WORKSPACES_PER_MON=""
BACKUP_STAMP="$(date +%Y%m%d%H%M%S)"
LOG_FILE="/tmp/inerium-install.log"
WITH_CAMERA=0
WITH_MSI_EC=0
WITH_NOVPN=0
WITH_CONVERTER=0
WITH_GDRIVE=0
INSTALL_LANG=""
MSI_EC_REBUILT=0

RELEASE_BASE_URL="${INERIUM_RELEASE_URL:-https://github.com/ineron01/inerium-shell/releases/latest/download}"
BIN_ARCHIVE="inerium-binaries-linux-x86_64.tar.gz"
BIN_SHA="inerium-binaries-linux-x86_64.tar.gz.sha256"

SILUETA_URL="https://github.com/danielgatis/rembg/releases/download/v0.0.0/silueta.onnx"
SILUETA_PATH="${XDG_DATA_HOME:-$HOME/.local/share}/dolphin-converter/models/silueta.onnx"

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Options:
  --bin              Install prebuilt binaries from GitHub Releases (default)
  --from-source      Compile Rust workspace locally via Cargo
  --dry-run          Print planned actions without modifying the system
  --skip-pkgs        Skip paru package installation
  --skip-build       Skip binary download and Cargo compilation
  --fetch-model      Download silueta.onnx (~43 MB) if missing
  --with-greeter     Install and configure greetd + inerium-greeter (/etc/greetd/)
  --with-gdrive      Install Google Drive integration for Dolphin via rclone
  --with-converter   Install Dolphin Converter (LibreOffice, Pandoc, ONNX background removal)
  --with-camera      Install Inerium Camera (Android phone webcam via USB/Wi-Fi)
  --with-msi-ec      Install patched msi-ec DKMS & msi-fan-regulator (for MSI laptops)
  --with-novpn       Install noVPN split-tunneling service and helper scripts
  --all-addons       Install all optional add-ons (--with-gdrive, --with-converter, --with-camera, --with-msi-ec, --with-novpn)
  --lang [en|ru]     Set installer language (en or ru)
  --workspaces N     Workspaces per monitor (1-9, default: 3)
  --update-configs   Update configuration files (backs up existing user files to .bak)
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
        --from-source)
            if [[ "$MODE_EXPLICIT" -eq 1 && "$INSTALL_MODE" == "bin" ]]; then
                echo "Cannot combine --bin and --from-source." >&2; exit 1
            fi
            INSTALL_MODE="source"; MODE_EXPLICIT=1; shift ;;
        --dry-run)         DRY_RUN=1; NON_INTERACTIVE=1; shift ;;
        --uninstall)       DO_UNINSTALL=1; NON_INTERACTIVE=1; shift ;;
        --check-abi)       CHECK_ABI=1; NON_INTERACTIVE=1; shift ;;
        --update-configs)  UPDATE_CONFIGS=1; shift ;;
        --prune|--clean)   shift ;;
        --skip-pkgs)       SKIP_PKGS=1; shift ;;
        --skip-build)      SKIP_BUILD=1; shift ;;
        --fetch-model)     FETCH_MODEL=1; shift ;;
        --with-greeter)    WITH_GREETER=1; shift ;;
        --with-gdrive)     WITH_GDRIVE=1; shift ;;
        --with-converter)  WITH_CONVERTER=1; shift ;;
        --with-camera)     WITH_CAMERA=1; shift ;;
        --with-msi-ec)     WITH_MSI_EC=1; shift ;;
        --with-novpn)      WITH_NOVPN=1; shift ;;
        --all-addons)      WITH_CONVERTER=1; WITH_GDRIVE=1; WITH_CAMERA=1; WITH_MSI_EC=1; WITH_NOVPN=1; shift ;;
        --lang)            INSTALL_LANG="${2:-}"; shift 2 ;;
        --workspaces)      WORKSPACES_PER_MON="${2:-}"; shift 2 ;;
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
source "$SCRIPTS_DIR/20-tui.sh"
source "$SCRIPTS_DIR/30-packages.sh"
source "$SCRIPTS_DIR/40-binaries.sh"
source "$SCRIPTS_DIR/50-configs.sh"
source "$SCRIPTS_DIR/60-addons.sh"
source "$SCRIPTS_DIR/70-summary.sh"

export -f ensure_paru install_pkgs install_pkglist install_from_binaries install_root_if_changed warn log
export RELEASE_BASE_URL BIN_ARCHIVE BIN_SHA USER_BINS_STR DRY_RUN INSTALL_MODE SKIP_BUILD UPDATE_CONFIGS

# ============================================================================
# Main Execution Pipeline
# ============================================================================
check_not_root
init_log_file

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

ensure_gum_installed
if use_tui; then
    run_tui_prompts
fi

# 1. Base packages (pacman / paru)
install_base_packages

# 2. Rust binaries delivery
deliver_binaries

# 3. User scripts, KIO menus, desktop entries, indexer
deploy_user_scripts_and_menus

# 4. Dotfiles (~/.config), mutable templates, and local hardware overlays
deploy_desktop_configs
detect_and_generate_local_overlays
deploy_wallpapers

# 5. Wayland sessions and login screen (greetd)
configure_greeter

# 6. Systemd user services and browser defaults
enable_user_services_and_defaults

# 7. Optional add-ons (Google Drive, Dolphin Converter, MSI Cooling, Camera, noVPN)
deploy_selected_addons

# 8. Record installation manifest and prune obsolete files
prune_obsolete_and_save_manifest

# 9. Summary & completion
if [[ "$DRY_RUN" -eq 0 ]]; then
    print_summary
else
    log "Dry-run complete."
fi
