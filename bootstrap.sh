#!/usr/bin/env bash
# ============================================================================
# inerium-shell/bootstrap.sh — One-command TTY/DE bootstrap for Arch/CachyOS
# ============================================================================
set -euo pipefail

REPO_URL="${INERIUM_SHELL_REPO:-${INERIUM_DESKTOP_REPO:-https://github.com/ineron01/inerium-shell.git}}"
LOG_FILE="/tmp/inerium-install-${UID}.log"
DRY_RUN=0
SKIP_PKGS=0
FROM_SOURCE=0

for arg in "$@"; do
    case "$arg" in
        --dry-run)     DRY_RUN=1 ;;
        --skip-pkgs)   SKIP_PKGS=1 ;;
        --from-source) FROM_SOURCE=1 ;;
    esac
done

# Target directory resolution:
# 1. Environment override (INERIUM_SHELL_DIR / INERIUM_DESKTOP_DIR)
# 2. Local execution check (if run from an existing local clone)
# 3. Visible home directory for source builds (~/inerium-shell)
# 4. Standard XDG data directory for prebuilt binaries (~/.local/share/inerium/shell)
if [[ -n "${INERIUM_SHELL_DIR:-}" ]]; then
    TARGET_DIR="$INERIUM_SHELL_DIR"
elif [[ -n "${INERIUM_DESKTOP_DIR:-}" ]]; then
    TARGET_DIR="$INERIUM_DESKTOP_DIR"
elif [[ -f "${BASH_SOURCE[0]:-}" && -f "$(dirname "${BASH_SOURCE[0]}")/install.sh" ]]; then
    TARGET_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
elif [[ "$FROM_SOURCE" -eq 1 ]]; then
    TARGET_DIR="$HOME/inerium-shell"
elif [[ -d "${XDG_DATA_HOME:-$HOME/.local/share}/inerium-shell" && ! -d "${XDG_DATA_HOME:-$HOME/.local/share}/inerium/shell" ]]; then
    TARGET_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/inerium-shell"
else
    TARGET_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/inerium/shell"
fi

cd "$HOME" 2>/dev/null || true

has_gum() {
    command -v gum >/dev/null 2>&1 && [[ -t 1 ]]
}

print_banner() {
    if has_gum; then
        gum style \
            --border rounded \
            --border-foreground 212 \
            --padding "1 3" \
            --margin "1 0" \
            --bold \
            "INERIUM SHELL" \
            "Universal Arch / CachyOS Hyprland + Rust Installer"
    else
        printf '\n=== INERIUM SHELL — Bootstrap ===\n\n'
    fi
}

run_quiet_step() {
    local title="$1"
    shift
    if [[ "$DRY_RUN" -eq 1 ]]; then
        printf '[DRY-RUN] %s: %s\n' "$title" "$*"
        return 0
    fi
    if has_gum; then
        printf '\n--- %s (%s) ---\n' "$title" "$(date '+%H:%M:%S')" >> "$LOG_FILE"
        if ! gum spin --spinner dot --title "$title" -- bash -c '"$@" >> "$0" 2>&1' "$LOG_FILE" "$@"; then
            printf '\n[bootstrap] ERROR during: %s\n' "$title" >&2
            printf '[bootstrap] Last 25 lines of %s:\n' "$LOG_FILE" >&2
            tail -n 25 "$LOG_FILE" >&2 || true
            exit 1
        fi
    else
        printf '[bootstrap] %s\n' "$title"
        "$@"
    fi
}

if [[ "$DRY_RUN" -eq 0 ]]; then
    : > "$LOG_FILE"
    if ! sudo -n true 2>/dev/null; then
        sudo -v
    fi
fi

# Ensure bootstrap prerequisites (git, gum)
if [[ "$SKIP_PKGS" -eq 0 ]] && command -v pacman >/dev/null 2>&1; then
    declare -a missing_bootstrap=()
    mapfile -t missing_bootstrap < <(pacman -T git gum 2>/dev/null || true)
    if [[ ${#missing_bootstrap[@]} -gt 0 ]]; then
        declare -a sync_dbs=(/var/lib/pacman/sync/*.db)
        if [[ ! -e "${sync_dbs[0]}" ]]; then
            if [[ "$DRY_RUN" -eq 1 ]]; then
                echo "[DRY-RUN] sudo pacman -Sy --noconfirm"
            else
                echo "[bootstrap] Syncing pacman databases..."
                # shellcheck disable=SC2024
                sudo pacman -Sy --noconfirm >> "$LOG_FILE" 2>&1 || true
            fi
        fi

        if [[ "$DRY_RUN" -eq 1 ]]; then
            echo "[DRY-RUN] sudo pacman -S --needed --noconfirm ${missing_bootstrap[*]}"
        else
            echo "[bootstrap] Installing bootstrap prerequisites (${missing_bootstrap[*]})..."
            # shellcheck disable=SC2024
            sudo pacman -S --needed --noconfirm "${missing_bootstrap[@]}" >> "$LOG_FILE" 2>&1 || true
        fi
    fi
elif ! command -v git >/dev/null 2>&1; then
    if [[ "$DRY_RUN" -eq 1 ]]; then
        echo "[DRY-RUN] sudo pacman -S --needed --noconfirm git"
    else
        sudo pacman -S --needed --noconfirm git
    fi
fi

print_banner

if [[ "$DRY_RUN" -eq 0 ]]; then
    mkdir -p "$(dirname "$TARGET_DIR")"
fi
if [[ -d "$TARGET_DIR/.git" ]]; then
    if [[ "$FROM_SOURCE" -eq 0 ]]; then
        run_quiet_step "Updating repository at $TARGET_DIR..." bash -c '
            git -C "$1" pull --ff-only || {
                git -C "$1" fetch --depth 1 origin main && git -C "$1" reset --hard origin/main
            }
        ' _ "$TARGET_DIR"
    else
        run_quiet_step "Updating repository at $TARGET_DIR..." git -C "$TARGET_DIR" pull --ff-only
    fi
else
    declare -a clone_opts=()
    if [[ "$FROM_SOURCE" -eq 0 ]]; then
        clone_opts+=(--depth 1)
    fi
    run_quiet_step "Cloning inerium-shell repository..." git clone "${clone_opts[@]}" "$REPO_URL" "$TARGET_DIR"
fi

if [[ ! -t 0 && -t 1 && -r /dev/tty ]]; then
    exec "$TARGET_DIR/install.sh" "$@" < /dev/tty
else
    exec "$TARGET_DIR/install.sh" "$@"
fi
