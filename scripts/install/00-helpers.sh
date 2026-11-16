#!/usr/bin/env bash
# ============================================================================
# scripts/install/00-helpers.sh — Logging, execution & file helpers
# ============================================================================

log()  { printf '[inerium-shell] %s\n' "$*"; }
warn() { printf '[inerium-shell] WARNING: %s\n' "$*" >&2; }

# Manifest recording fallbacks (overridden when 05-manifest.sh is loaded)
manifest_record_binary() { :; }
manifest_record_symlink() { :; }
manifest_record_template() { :; }
manifest_record_system_file() { :; }
manifest_record_user_unit() { :; }
manifest_record_system_unit() { :; }
manifest_record_addon() { :; }

has_gum() {
    command -v gum >/dev/null 2>&1 && [[ -t 0 && -t 1 ]]
}

use_tui() {
    [[ "$DRY_RUN" -eq 0 && "$NON_INTERACTIVE" -eq 0 ]] && has_gum
}

check_not_root() {
    if [[ "$EUID" -eq 0 ]]; then
        echo "Run install.sh as your regular user (not root/sudo); it calls sudo itself." >&2
        exit 1
    fi
}

init_log_file() {
    if [[ "$DRY_RUN" -eq 0 ]]; then
        mkdir -p "$(dirname "$LOG_FILE")"
        : > "$LOG_FILE"
    fi
}

init_sudo_keepalive() {
    if [[ "$DRY_RUN" -eq 0 && -z "${INERIUM_SUDO_KEEPALIVE:-}" ]]; then
        if ! sudo -n true 2>/dev/null; then
            sudo -v
        fi
        ( while kill -0 "$$" 2>/dev/null; do sudo -n -v 2>/dev/null; sleep 50; done ) &
        export INERIUM_SUDO_KEEPALIVE=$!
        trap 'kill "$INERIUM_SUDO_KEEPALIVE" 2>/dev/null || true' EXIT
    fi
}

run() {
    if [[ "$DRY_RUN" -eq 1 ]]; then
        printf '[DRY-RUN] %s\n' "$*"
    else
        eval "$@"
    fi
}

run_step() {
    local title="$1"
    shift
    if [[ "$DRY_RUN" -eq 1 ]]; then
        printf '[DRY-RUN] %s: %s\n' "$title" "$*"
        return 0
    fi
    if use_tui; then
        printf '\n=== %s (%s) ===\n' "$title" "$(date '+%H:%M:%S')" >> "$LOG_FILE"
        if ! gum spin --spinner dot --title "$title" -- bash -c '"$@" >> "$0" 2>&1' "$LOG_FILE" "$@"; then
            printf '\n[inerium-shell] ERROR during: %s\n' "$title" >&2
            printf '[inerium-shell] Last 25 lines of %s:\n' "$LOG_FILE" >&2
            tail -n 25 "$LOG_FILE" >&2 || true
            return 1
        fi
    else
        log "$title"
        "$@"
    fi
}

link_item() {
    local src="$1"
    local dst="$2"
    manifest_record_symlink "$dst" "$src"
    if [[ "$DRY_RUN" -eq 1 ]]; then
        if [[ -e "$dst" && ! -L "$dst" ]]; then
            printf '[DRY-RUN] mv %s -> %s.bak.%s\n' "$dst" "$dst" "$BACKUP_STAMP"
        fi
        printf '[DRY-RUN] ln -sfn %s -> %s\n' "$src" "$dst"
        return 0
    fi
    mkdir -p "$(dirname "$dst")"
    if [[ -L "$dst" ]]; then
        if [[ "$(readlink -f "$dst" 2>/dev/null || true)" == "$(readlink -f "$src" 2>/dev/null || true)" ]]; then
            return 0
        fi
    elif [[ -e "$dst" ]]; then
        local bak="${dst}.bak.${BACKUP_STAMP}"
        log "Backing up existing target: $dst -> $bak"
        mv "$dst" "$bak"
    fi
    ln -sfn "$src" "$dst"
}

render_template() {
    local tpl="$1"
    local dst="$2"
    local only_if_missing="${3:-0}"
    manifest_record_template "$dst"
    if [[ "$only_if_missing" -eq 1 && -e "$dst" ]]; then
        log "Keeping existing mutable file: $dst"
        return 0
    fi
    if [[ "$DRY_RUN" -eq 1 ]]; then
        printf '[DRY-RUN] render %s -> %s\n' "$tpl" "$dst"
        return 0
    fi
    mkdir -p "$(dirname "$dst")"
    sed -e "s|__HOME__|$HOME|g" -e "s|__USER__|$USER|g" "$tpl" > "${dst}.tmp"
    mv -f "${dst}.tmp" "$dst"
}

install_root_if_changed() {
    local mode="$1"
    local src="$2"
    local dst="$3"
    manifest_record_system_file "$dst"
    if [[ "$DRY_RUN" -eq 1 ]]; then
        if ! cmp -s "$src" "$dst" 2>/dev/null; then
            printf '[DRY-RUN] sudo install -m %s %s -> %s\n' "$mode" "$src" "$dst"
        fi
        return 0
    fi
    if ! cmp -s "$src" "$dst" 2>/dev/null; then
        sudo install -d -m 755 "$(dirname "$dst")"
        sudo install -m "$mode" "$src" "$dst"
    fi
}

install_sudoers_template() {
    local tpl="$1"
    local dst="$2"
    manifest_record_system_file "$dst"
    if [[ "$DRY_RUN" -eq 1 ]]; then
        printf '[DRY-RUN] render sudoers %s -> %s (visudo -cf)\n' "$tpl" "$dst"
        return 0
    fi
    local tmp
    tmp="$(mktemp)"
    sed -e "s|__HOME__|$HOME|g" -e "s|__USER__|$USER|g" "$tpl" > "$tmp"
    if sudo cmp -s "$tmp" "$dst" 2>/dev/null; then
        rm -f "$tmp"
        return 0
    fi
    if sudo visudo -cf "$tmp" >/dev/null 2>&1; then
        sudo install -m 0440 "$tmp" "$dst"
    else
        warn "Invalid sudoers syntax in $tpl; skipped $dst"
    fi
    rm -f "$tmp"
}

user_unit_path() {
    local d
    for d in "$HOME/.config/systemd/user" /etc/systemd/user /usr/lib/systemd/user; do
        if [[ -e "$d/$1" ]]; then
            printf '%s\n' "$d/$1"
            return 0
        fi
    done
    return 1
}
