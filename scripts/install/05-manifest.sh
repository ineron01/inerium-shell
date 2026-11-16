#!/usr/bin/env bash
# ============================================================================
# scripts/install/05-manifest.sh — Installation State Tracking & Manifest
# ============================================================================

MANIFEST_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/inerium"
MANIFEST_FILE="$MANIFEST_DIR/install-manifest.json"
MANIFEST_TMP_DIR=""

init_manifest() {
    MANIFEST_TMP_DIR="$(mktemp -d /tmp/inerium-manifest-XXXXXX)"
    : > "$MANIFEST_TMP_DIR/binaries"
    : > "$MANIFEST_TMP_DIR/symlinks"
    : > "$MANIFEST_TMP_DIR/templates"
    : > "$MANIFEST_TMP_DIR/system_files"
    : > "$MANIFEST_TMP_DIR/user_units"
    : > "$MANIFEST_TMP_DIR/system_units"
    : > "$MANIFEST_TMP_DIR/addons"
}

cleanup_manifest_tmp() {
    if [[ -n "${MANIFEST_TMP_DIR:-}" && -d "$MANIFEST_TMP_DIR" ]]; then
        rm -rf "$MANIFEST_TMP_DIR"
    fi
}
trap cleanup_manifest_tmp EXIT

manifest_record_binary() {
    [[ -n "${MANIFEST_TMP_DIR:-}" && -d "$MANIFEST_TMP_DIR" ]] && printf '%s\n' "$1" >> "$MANIFEST_TMP_DIR/binaries"
}

manifest_record_symlink() {
    [[ -n "${MANIFEST_TMP_DIR:-}" && -d "$MANIFEST_TMP_DIR" ]] && printf '%s\t%s\n' "$1" "$2" >> "$MANIFEST_TMP_DIR/symlinks"
}

manifest_record_template() {
    [[ -n "${MANIFEST_TMP_DIR:-}" && -d "$MANIFEST_TMP_DIR" ]] && printf '%s\n' "$1" >> "$MANIFEST_TMP_DIR/templates"
}

manifest_record_system_file() {
    [[ -n "${MANIFEST_TMP_DIR:-}" && -d "$MANIFEST_TMP_DIR" ]] && printf '%s\n' "$1" >> "$MANIFEST_TMP_DIR/system_files"
}

manifest_record_user_unit() {
    [[ -n "${MANIFEST_TMP_DIR:-}" && -d "$MANIFEST_TMP_DIR" ]] && printf '%s\n' "$1" >> "$MANIFEST_TMP_DIR/user_units"
}

manifest_record_system_unit() {
    [[ -n "${MANIFEST_TMP_DIR:-}" && -d "$MANIFEST_TMP_DIR" ]] && printf '%s\n' "$1" >> "$MANIFEST_TMP_DIR/system_units"
}

manifest_record_addon() {
    [[ -n "${MANIFEST_TMP_DIR:-}" && -d "$MANIFEST_TMP_DIR" ]] && printf '%s\n' "$1" >> "$MANIFEST_TMP_DIR/addons"
}

prune_obsolete_and_save_manifest() {
    if [[ "$DRY_RUN" -eq 1 ]]; then
        log "Dry-run: skipping manifest commit & prune."
        return 0
    fi

    log "Recording installation manifest & pruning obsolete items..."
    mkdir -p "$MANIFEST_DIR"
    local commit version
    commit="$(git -C "$REPO_DIR" rev-parse HEAD 2>/dev/null || echo "unknown")"
    version="$(git -C "$REPO_DIR" describe --tags --always 2>/dev/null || echo "v0.1.1")"

    python3 "$SCRIPTS_DIR/manifest.py" save \
        --temp-dir "$MANIFEST_TMP_DIR" \
        --manifest "$MANIFEST_FILE" \
        --repo-dir "$REPO_DIR" \
        --version "$version" \
        --commit "$commit" \
        --install-mode "$INSTALL_MODE"

    # Also clean broken symlinks and known legacy orphans
    python3 "$SCRIPTS_DIR/manifest.py" clean-orphans \
        --repo-dir "$REPO_DIR"
}

run_uninstall() {
    check_not_root
    init_sudo_keepalive
    log "Initiating Inerium Shell uninstallation..."
    python3 "$SCRIPTS_DIR/manifest.py" uninstall \
        --manifest "$MANIFEST_FILE" \
        --repo-dir "$REPO_DIR" \
        --dry-run "$DRY_RUN"
    log "Uninstallation completed successfully."
}
