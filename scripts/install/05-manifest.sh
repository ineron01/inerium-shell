#!/usr/bin/env bash
# ============================================================================
# scripts/install/05-manifest.sh — Pure Bash Installation State & Uninstaller
# ============================================================================

MANIFEST_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/inerium"
MANIFEST_FILE="$MANIFEST_DIR/install-manifest"
NEW_MANIFEST_FILE=""

init_manifest() {
    NEW_MANIFEST_FILE="$(mktemp /tmp/inerium-manifest-XXXXXX)"
}

cleanup_manifest_tmp() {
    if [[ -n "${NEW_MANIFEST_FILE:-}" && -f "$NEW_MANIFEST_FILE" ]]; then
        rm -f "$NEW_MANIFEST_FILE"
    fi
}
trap cleanup_manifest_tmp EXIT

manifest_record() {
    local type="$1" target="$2" extra="${3:-}"
    [[ -n "${NEW_MANIFEST_FILE:-}" && -f "$NEW_MANIFEST_FILE" ]] || return 0
    printf '%s\t%s\t%s\n' "$type" "$target" "$extra" >> "$NEW_MANIFEST_FILE"
}

manifest_record_binary()      { manifest_record "BIN" "$1"; }
manifest_record_symlink()     { manifest_record "LINK" "$1" "$2"; }
manifest_record_template()    { manifest_record "RENDER" "$1"; }
manifest_record_system_file() { manifest_record "SYS_FILE" "$1"; }
manifest_record_user_unit()   { manifest_record "USER_UNIT" "$1"; }
manifest_record_system_unit() { manifest_record "SYS_UNIT" "$1"; }
manifest_record_addon()       { manifest_record "ADDON" "$1"; }

is_safe_removal_target() {
    local t="$1"
    [[ -n "$t" ]] || return 1
    case "$t" in
        /|/etc|/etc/|/usr|/usr/|/usr/local|/usr/local/|/var|/var/|/home|/home/|/root|/root/)
            return 1
            ;;
    esac
    local slashes
    slashes=$(tr -cd '/' <<< "$t" | wc -c)
    (( slashes >= 2 && ${#t} >= 6 )) || return 1
    return 0
}

manifest_clean_orphans() {
    log "Checking for broken symlinks and legacy artifacts..."
    local dir
    for dir in "$HOME/.local/bin" "$HOME/.local/share/applications" "$HOME/.local/share/color-schemes" "$HOME/.local/share/kio/servicemenus" "$HOME/.config" "$HOME/.config/hypr" "$HOME/.config/hypr/config" "$HOME/.config/systemd/user" "${XDG_CONFIG_HOME:-$HOME/.config}/inerium/first-login.d"; do
        [[ -d "$dir" ]] || continue
        while IFS= read -r -d '' link; do
            local raw
            raw="$(readlink "$link" 2>/dev/null || true)"
            if [[ "$raw" == *inerium* || "$raw" == *kanagawa* ]]; then
                log "Removing broken inerium symlink: $link"
                rm -f "$link"
            fi
        done < <(find "$dir" -maxdepth 1 -xtype l -print0 2>/dev/null)
    done

    # Known legacy obsolete paths from v0.1.0 and Kanagawa
    local legacy
    for legacy in \
        "$HOME/.config/hypr/AGENTS.md" \
        "$HOME/.config/hypr/hyprtoolkit.conf" \
        "$HOME/.config/hypr/profile" \
        "$HOME/.config/hypr/config/autostart_backup.lua" \
        "$HOME/.config/hypr/config/binds_backup.lua" \
        "$HOME/.config/hypr/config/inputs.lua" \
        "$HOME/.config/hypr/config/autostart_waybar.lua" \
        "$HOME/.config/hypr/config/binds_waybar.lua" \
        "$HOME/.config/hypr/config/binds_common.lua" \
        "$HOME/.config/hypr/config/theme_settings.json" \
        "$HOME/.local/bin/hypr-profile" \
        "$HOME/.local/bin/inerium-hotspot.sh" \
        "$HOME/.local/bin/inerium-popup-backup" \
        "$HOME/.local/bin/inerium-switch-bar" \
        "$HOME/.local/bin/inerium-switch-bar-backup" \
        "$HOME/.local/bin/rofi-audio-sink" \
        "$HOME/.local/bin/rofi-bluetooth" \
        "$HOME/.local/bin/rofi-power-menu" \
        "$HOME/.local/bin/rofi-power-menu-backup" \
        "$HOME/.local/bin/rofi-wifi-menu" \
        "$HOME/.local/bin/inerium-toggle-bluetooth.sh" \
        "$HOME/.local/bin/inerium-toggle-network.sh"
    do
        if [[ -e "$legacy" || -L "$legacy" ]]; then
            log "Removing legacy file/link: $legacy"
            rm -rf "$legacy"
        fi
    done
}

prune_obsolete_and_save_manifest() {
    if [[ "$DRY_RUN" -eq 1 ]]; then
        log "Dry-run: skipping manifest commit & prune."
        return 0
    fi

    log "Recording installation manifest & pruning obsolete items..."
    mkdir -p "$MANIFEST_DIR"

    # Prune obsolete items if old manifest exists
    if [[ -f "$MANIFEST_FILE" && -s "$NEW_MANIFEST_FILE" ]]; then
        declare -A new_items=()
        while IFS=$'\t' read -r type target extra; do
            [[ -z "$type" || "$type" =~ ^# ]] && continue
            new_items["$type:$target"]=1
        done < "$NEW_MANIFEST_FILE"

        while IFS=$'\t' read -r type target extra; do
            [[ -z "$type" || "$type" =~ ^# ]] && continue
            if [[ -z "${new_items["$type:$target"]:-}" ]]; then
                case "$type" in
                    LINK)
                        if [[ -L "$target" ]]; then
                            local dest
                            dest="$(readlink -f "$target" 2>/dev/null || true)"
                            if [[ "$dest" == *inerium* || "$dest" == *kanagawa* || ! -e "$target" ]]; then
                                log "Pruning obsolete symlink: $target"
                                rm -f "$target"
                            fi
                        fi
                        ;;
                    BIN)
                        if [[ -f "$target" ]]; then
                            log "Pruning obsolete binary: $target"
                            rm -f "$target"
                        fi
                        ;;
                    RENDER)
                        if [[ -f "$target" ]]; then
                            log "Pruning obsolete rendered config: $target"
                            rm -f "$target"
                        fi
                        ;;
                    USER_UNIT)
                        log "Disabling obsolete user unit: $target"
                        systemctl --user disable --now "$target" 2>/dev/null || true
                        rm -f "$HOME/.config/systemd/user/$target"
                        ;;
                    SYS_FILE)
                        if is_safe_removal_target "$target" && [[ -e "$target" ]]; then
                            log "Pruning obsolete system file: $target"
                            sudo rm -rf "$target" 2>/dev/null || true
                        fi
                        ;;
                    SYS_UNIT)
                        log "Disabling obsolete system unit: $target"
                        sudo systemctl disable --now "$target" 2>/dev/null || true
                        sudo rm -f "/etc/systemd/system/$target"
                        ;;
                esac
            fi
        done < "$MANIFEST_FILE"
        systemctl --user daemon-reload 2>/dev/null || true
        sudo systemctl daemon-reload 2>/dev/null || true
    fi

    # Clean broken symlinks & legacy paths
    manifest_clean_orphans

    # Save new manifest atomically
    local commit version now
    commit="$(git -C "$REPO_DIR" rev-parse --short HEAD 2>/dev/null || echo "unknown")"
    version="$(git -C "$REPO_DIR" describe --tags --always 2>/dev/null || echo "v0.2.0")"
    now="$(date -u +'%Y-%m-%dT%H:%M:%SZ')"

    {
        printf '# INERIUM_SHELL_MANIFEST %s\n' "$version"
        printf '# COMMIT %s\n' "$commit"
        printf '# DATE %s\n' "$now"
        printf '# MODE %s\n' "$INSTALL_MODE"
        printf '# REPO %s\n' "$REPO_DIR"
        sort -u "$NEW_MANIFEST_FILE"
    } > "${MANIFEST_FILE}.tmp"
    mv -f "${MANIFEST_FILE}.tmp" "$MANIFEST_FILE"
    rm -f "$NEW_MANIFEST_FILE"
    log "Updated installation manifest: $MANIFEST_FILE"
}

run_uninstall() {
    check_not_root
    init_sudo_keepalive
    log "Initiating Inerium Shell uninstallation..."

    if [[ ! -f "$MANIFEST_FILE" ]]; then
        warn "Manifest not found at $MANIFEST_FILE."
        warn "Performing fallback orphan cleanup..."
        manifest_clean_orphans
        return 0
    fi

    # 1. Stop and disable system services
    while IFS=$'\t' read -r type target extra; do
        if [[ "$type" == "SYS_UNIT" ]]; then
            log "Stopping and disabling system service: $target"
            if [[ "$DRY_RUN" -eq 1 ]]; then
                printf '[DRY-RUN] sudo systemctl disable --now %s\n' "$target"
            else
                sudo systemctl disable --now "$target" 2>/dev/null || true
                sudo rm -f "/etc/systemd/system/$target"
            fi
        fi
    done < "$MANIFEST_FILE"

    # 2. Stop and disable user services
    while IFS=$'\t' read -r type target extra; do
        if [[ "$type" == "USER_UNIT" ]]; then
            log "Stopping and disabling user service: $target"
            if [[ "$DRY_RUN" -eq 1 ]]; then
                printf '[DRY-RUN] systemctl --user disable --now %s\n' "$target"
            else
                systemctl --user disable --now "$target" 2>/dev/null || true
                rm -f "$HOME/.config/systemd/user/$target"
            fi
        fi
    done < "$MANIFEST_FILE"

    # 3. Remove tracked items (symlinks, binaries, templates, system files)
    while IFS=$'\t' read -r type target extra; do
        case "$type" in
            LINK)
                if [[ -L "$target" || ! -e "$target" ]]; then
                    log "Removing symlink: $target"
                    if [[ "$DRY_RUN" -eq 1 ]]; then
                        printf '[DRY-RUN] rm -f %s\n' "$target"
                    else
                        rm -f "$target"
                    fi
                fi
                ;;
            BIN)
                if [[ -f "$target" ]]; then
                    log "Removing user binary: $target"
                    if [[ "$DRY_RUN" -eq 1 ]]; then
                        printf '[DRY-RUN] rm -f %s\n' "$target"
                    else
                        rm -f "$target"
                    fi
                fi
                ;;
            RENDER)
                if [[ -f "$target" ]]; then
                    log "Removing rendered configuration: $target"
                    if [[ "$DRY_RUN" -eq 1 ]]; then
                        printf '[DRY-RUN] rm -f %s\n' "$target"
                    else
                        rm -f "$target"
                    fi
                fi
                ;;
            SYS_FILE)
                if is_safe_removal_target "$target" && [[ -e "$target" ]]; then
                    log "Removing system file: $target"
                    if [[ "$DRY_RUN" -eq 1 ]]; then
                        printf '[DRY-RUN] sudo rm -rf %s\n' "$target"
                    else
                        sudo rm -rf "$target" 2>/dev/null || true
                    fi
                fi
                ;;
        esac
    done < "$MANIFEST_FILE"

    # Remove set-wallpaper symlink
    if [[ "$DRY_RUN" -eq 1 ]]; then
        printf '[DRY-RUN] rm -f %s\n' "$HOME/.local/bin/set-wallpaper"
    else
        rm -f "$HOME/.local/bin/set-wallpaper"
    fi

    # 4. Clean orphans
    manifest_clean_orphans

    # 5. Clean state files
    if [[ "$DRY_RUN" -eq 0 ]]; then
        rm -f "$MANIFEST_FILE" "$MANIFEST_DIR/setup.done"
        rmdir "$MANIFEST_DIR" 2>/dev/null || true
    fi

    # 6. Clean cloned repository if installed in user data directory (~/.local/share/inerium-shell)
    local user_shell_dir="${XDG_DATA_HOME:-$HOME/.local/share}/inerium-shell"
    local installed_repo="${REPO_DIR:-}"
    if [[ -z "$installed_repo" && -f "$MANIFEST_FILE" ]]; then
        installed_repo="$(grep '^# REPO ' "$MANIFEST_FILE" | head -n 1 | cut -d' ' -f3- || true)"
    fi

    if [[ -n "$installed_repo" && -d "$user_shell_dir" ]]; then
        local can_installed can_user
        can_installed="$(realpath "$installed_repo" 2>/dev/null || true)"
        can_user="$(realpath "$user_shell_dir" 2>/dev/null || true)"
        if [[ -n "$can_installed" && "$can_installed" == "$can_user" ]]; then
            log "Removing cloned repository in user data directory: $user_shell_dir"
            if [[ "$DRY_RUN" -eq 1 ]]; then
                printf '[DRY-RUN] rm -rf %s\n' "$user_shell_dir"
            else
                rm -rf "$user_shell_dir"
            fi
        else
            log "Preserving custom repository path: $installed_repo"
        fi
    fi
    log "Inerium Shell uninstallation complete."
}
