#!/usr/bin/env bash
# ============================================================================
# scripts/install/05-manifest.sh — Pure Bash Installation State & Uninstaller
# ============================================================================
# shellcheck disable=SC2034 # Variables exported and consumed across installer modules

MANIFEST_DIR="${MANIFEST_DIR:-${XDG_STATE_HOME:-$HOME/.local/state}/inerium}"
MANIFEST_FILE="${MANIFEST_FILE:-$MANIFEST_DIR/install-manifest}"
NEW_MANIFEST_FILE="${NEW_MANIFEST_FILE:-}"

# Migrate legacy installed.manifest to install-manifest if needed
if [[ "${DRY_RUN:-0}" -eq 0 && ! -f "$MANIFEST_FILE" && -f "$MANIFEST_DIR/installed.manifest" ]]; then
    mkdir -p "$MANIFEST_DIR"
    mv -f "$MANIFEST_DIR/installed.manifest" "$MANIFEST_FILE" 2>/dev/null || true
fi

init_manifest() {
    if [[ "${DRY_RUN:-0}" -eq 0 ]]; then
        if [[ ! -f "$MANIFEST_FILE" && -f "$MANIFEST_DIR/installed.manifest" ]]; then
            mkdir -p "$MANIFEST_DIR"
            mv -f "$MANIFEST_DIR/installed.manifest" "$MANIFEST_FILE" 2>/dev/null || true
        fi
        NEW_MANIFEST_FILE="$(mktemp /tmp/inerium-manifest-XXXXXX)"
    else
        NEW_MANIFEST_FILE=""
    fi
}

cleanup_manifest_tmp() {
    if [[ -n "${NEW_MANIFEST_FILE:-}" && -f "$NEW_MANIFEST_FILE" ]]; then
        rm -f "$NEW_MANIFEST_FILE"
    fi
}

manifest_record() {
    local type="$1" target="$2" extra="${3:-}"
    [[ -n "${NEW_MANIFEST_FILE:-}" && -f "$NEW_MANIFEST_FILE" ]] || return 0
    printf '%s\t%s\t%s\n' "$type" "$target" "$extra" >> "$NEW_MANIFEST_FILE"
}

manifest_record_binary()      { manifest_record "BIN" "$1"; }
manifest_record_symlink()     { manifest_record "LINK" "$1" "$2"; }
manifest_record_template()    { manifest_record "RENDER" "$1"; }
manifest_record_config()      { manifest_record "CONFIG" "$1"; }
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

migrate_legacy_dir_symlinks() {
    local config_dir
    for config_dir in environment.d gtk-3.0 gtk-4.0 kitty mako rofi uwsm waybar hypr; do
        local target="$HOME/.config/$config_dir"
        if [[ -L "$target" ]]; then
            local raw_dest
            raw_dest="$(readlink -f "$target" 2>/dev/null || true)"
            if [[ "$raw_dest" == *inerium* || "$raw_dest" == *kanagawa* ]]; then
                # Guard developer workspace on live host
                if [[ -n "${REPO_DIR:-}" && "$raw_dest" == "$REPO_DIR"* && -d "$REPO_DIR/.git" && ( -n "${INERIUM_DEV:-}" || -n "$(git -C "$REPO_DIR" status --porcelain 2>/dev/null)" ) ]]; then
                    continue
                fi
                log "Migrating legacy directory symlink to real directory: $target"
                run "rm -f '$target'"
                run "mkdir -p '$target'"
            fi
        else
            run "mkdir -p '$HOME/.config/$config_dir'"
        fi
    done
}

manifest_clean_orphans() {
    log "Checking for broken symlinks and legacy artifacts..."
    local dir
    for dir in "$HOME/.local/bin" "$HOME/.local/share/applications" "$HOME/.local/share/color-schemes" "$HOME/.local/share/kio/servicemenus" "$HOME/.config" "$HOME/.config/hypr" "$HOME/.config/hypr/config" "$HOME/.config/systemd/user" "${XDG_CONFIG_HOME:-$HOME/.config}/inerium/first-login.d"; do
        [[ -d "$dir" ]] || continue
        while IFS= read -r -d '' link; do
            case "$(basename "$link")" in
                *_local.*|*_backup.*|*.bak*)
                    continue
                    ;;
            esac
            local raw
            raw="$(readlink "$link" 2>/dev/null || true)"
            if [[ "$raw" == *inerium* || "$raw" == *kanagawa* ]]; then
                log "Removing broken inerium symlink: $link"
                if [[ "$DRY_RUN" -eq 0 ]]; then
                    rm -f "$link"
                else
                    printf '[DRY-RUN] rm -f %s\n' "$link"
                fi
            fi
        done < <(find "$dir" -maxdepth 1 -xtype l -print0 2>/dev/null)
    done

    # Known legacy obsolete paths from v0.1.0 and Kanagawa
    local legacy
    for legacy in \
        "$HOME/.config/hypr/AGENTS.md" \
        "$HOME/.config/hypr/hyprtoolkit.conf" \
        "$HOME/.config/hypr/profile" \
        "$HOME/.config/hypr/config/inputs.lua" \
        "$HOME/.config/hypr/config/autostart_waybar.lua" \
        "$HOME/.config/hypr/config/binds_waybar.lua" \
        "$HOME/.config/hypr/config/binds_common.lua" \
        "$HOME/.config/hypr/config/theme_settings.json" \
        "$HOME/.local/bin/hypr-profile" \
        "$HOME/.local/bin/inerium-hotspot.sh" \
        "$HOME/.local/bin/inerium-switch-bar" \
        "$HOME/.local/bin/rofi-audio-sink" \
        "$HOME/.local/bin/rofi-bluetooth" \
        "$HOME/.local/bin/rofi-power-menu" \
        "$HOME/.local/bin/rofi-wifi-menu" \
        "$HOME/.local/bin/inerium-toggle-bluetooth.sh" \
        "$HOME/.local/bin/inerium-toggle-network.sh" \
        "$HOME/.local/bin/inerium-first-login" \
        "$HOME/.local/share/applications/inerium-first-login.desktop" \
        "$HOME/.local/share/applications/google-chrome-direct.desktop" \
        "$HOME/.config/fish/functions/code.fish" \
        "$HOME/.config/fish/functions/hybrid.fish" \
        "$HOME/.config/fish/functions/integrated.fish"
    do
        case "$(basename "$legacy")" in
            *_local.*|*_backup.*|*.bak*)
                continue
                ;;
        esac
        if [[ "$legacy" == "$HOME/.config/fish/functions/"* || "$legacy" == "$HOME/.local/share/applications/google-chrome-direct.desktop" ]]; then
            if [[ -L "$legacy" ]]; then
                local raw_link
                raw_link="$(readlink "$legacy" 2>/dev/null || true)"
                if [[ "$raw_link" == *inerium* || "$raw_link" == *kanagawa* ]]; then
                    log "Removing legacy symlink: $legacy"
                    if [[ "$DRY_RUN" -eq 0 ]]; then
                        rm -f "$legacy"
                    else
                        printf '[DRY-RUN] rm -f %s\n' "$legacy"
                    fi
                fi
            fi
            continue
        fi

        if [[ -e "$legacy" || -L "$legacy" ]]; then
            log "Removing legacy file/link: $legacy"
            if [[ "$DRY_RUN" -eq 0 ]]; then
                rm -rf "$legacy"
            else
                printf '[DRY-RUN] rm -rf %s\n' "$legacy"
            fi
        fi
    done
}

disable_greetd_safely() {
    log "Disabling display manager autostart (greetd, keeping active session)..."
    if [[ "$DRY_RUN" -eq 1 ]]; then
        printf '[DRY-RUN] sudo systemctl disable greetd.service\n'
    else
        sudo systemctl disable greetd.service 2>/dev/null || true
    fi

    # Restore /etc/greetd/config.toml from newest .bak.* if available
    local newest_cfg_bak
    newest_cfg_bak="$(ls -1t /etc/greetd/config.toml.bak.* 2>/dev/null | head -n 1 || true)"
    if [[ -n "$newest_cfg_bak" && -f "$newest_cfg_bak" ]]; then
        log "Restoring original greetd config from $newest_cfg_bak"
        if [[ "$DRY_RUN" -eq 1 ]]; then
            printf '[DRY-RUN] sudo cp -a %s /etc/greetd/config.toml\n' "$newest_cfg_bak"
        else
            sudo cp -a "$newest_cfg_bak" /etc/greetd/config.toml 2>/dev/null || true
        fi
    fi

    # Restore /etc/pam.d/greetd from newest .bak.* if available
    local newest_pam_bak
    newest_pam_bak="$(ls -1t /etc/pam.d/greetd.bak.* 2>/dev/null | head -n 1 || true)"
    if [[ -n "$newest_pam_bak" && -f "$newest_pam_bak" ]]; then
        log "Restoring original greetd PAM config from $newest_pam_bak"
        if [[ "$DRY_RUN" -eq 1 ]]; then
            printf '[DRY-RUN] sudo cp -a %s /etc/pam.d/greetd\n' "$newest_pam_bak"
        else
            sudo cp -a "$newest_pam_bak" /etc/pam.d/greetd 2>/dev/null || true
        fi
    fi

    # Remove inerium-greeter configs and binaries
    if [[ "$DRY_RUN" -eq 1 ]]; then
        printf '[DRY-RUN] sudo rm -f /etc/greetd/inerium-greeter.json /usr/local/bin/inerium-greeter\n'
    else
        sudo rm -f /etc/greetd/inerium-greeter.json /usr/local/bin/inerium-greeter 2>/dev/null || true
    fi
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
        declare -A new_targets=()
        while IFS=$'\t' read -r type target extra; do
            [[ -z "$type" || "$type" =~ ^# ]] && continue
            new_items["$type:$target"]=1
            new_targets["$target"]=1
        done < "$NEW_MANIFEST_FILE"

        # Pass 1: Stop and disable obsolete units (before removing files/scripts)
        while IFS=$'\t' read -r type target extra; do
            [[ -z "$type" || "$type" =~ ^# ]] && continue
            case "$(basename "$target")" in
                *_local.*|*_backup.*|*.bak*)
                    continue
                    ;;
            esac

            if [[ -n "${new_targets["$target"]:-}" ]]; then
                continue
            fi

            if [[ -z "${new_items["$type:$target"]:-}" ]]; then
                case "$type" in
                    SYS_UNIT)
                        if [[ "$target" == "greetd.service" ]]; then
                            disable_greetd_safely
                        else
                            log "Disabling obsolete system unit: $target"
                            sudo systemctl disable --now "$target" 2>/dev/null || true
                            sudo rm -f "/etc/systemd/system/$target"
                        fi
                        ;;
                    USER_UNIT)
                        log "Disabling obsolete user unit: $target"
                        systemctl --user disable --now "$target" 2>/dev/null || true
                        rm -f "$HOME/.config/systemd/user/$target"
                        ;;
                esac
            fi
        done < "$MANIFEST_FILE"

        # Pass 2: Remove obsolete tracked files (LINK, BIN, RENDER, CONFIG, SYS_FILE)
        while IFS=$'\t' read -r type target extra; do
            [[ -z "$type" || "$type" =~ ^# ]] && continue
            case "$(basename "$target")" in
                *_local.*|*_backup.*|*.bak*)
                    continue
                    ;;
            esac

            if [[ "$target" == "$HOME/.config/locale.conf" || "$target" == "${XDG_CONFIG_HOME:-$HOME/.config}/locale.conf" ]]; then
                continue
            fi

            # Migration guard: if target path exists in new manifest under ANY type (e.g. BIN -> LINK),
            # never prune or delete it!
            if [[ -n "${new_targets["$target"]:-}" ]]; then
                continue
            fi

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
                        if [[ -f "$target" && ! -L "$target" ]]; then
                            log "Pruning obsolete binary: $target"
                            rm -f "$target"
                        fi
                        ;;
                    RENDER)
                        if [[ -f "$target" && ! -L "$target" ]]; then
                            log "Pruning obsolete rendered config: $target"
                            rm -f "$target"
                        fi
                        ;;
                    CONFIG)
                        if [[ -f "$target" && ! -L "$target" ]]; then
                            local bak="${target}.bak.${BACKUP_STAMP}"
                            log "Backing up obsolete installed config: $target -> $bak"
                            mv -f "$target" "$bak" 2>/dev/null || rm -f "$target"
                        fi
                        ;;
                    SYS_FILE)
                        case "$target" in
                            /etc/greetd/config.toml|/etc/greetd/environments|/etc/pam.d/greetd)
                                continue
                                ;;
                        esac
                        if is_safe_removal_target "$target" && [[ -e "$target" ]]; then
                            log "Pruning obsolete system file: $target"
                            sudo rm -rf "$target" 2>/dev/null || true
                        fi
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
    version="$(git -C "$REPO_DIR" describe --tags --always 2>/dev/null || echo "v0.2.2")"
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

    if [[ ! -f "$MANIFEST_FILE" && -f "$MANIFEST_DIR/installed.manifest" ]]; then
        if [[ "${DRY_RUN:-0}" -eq 1 ]]; then
            MANIFEST_FILE="$MANIFEST_DIR/installed.manifest"
        else
            mv -f "$MANIFEST_DIR/installed.manifest" "$MANIFEST_FILE" 2>/dev/null || true
        fi
    fi

    if [[ ! -f "$MANIFEST_FILE" ]]; then
        warn "Manifest not found at $MANIFEST_FILE."
        warn "Performing fallback orphan cleanup..."
        manifest_clean_orphans
        return 0
    fi

    # Resolve installed repo directory BEFORE deleting manifest (manifest takes precedence over current REPO_DIR)
    local installed_repo
    installed_repo="$(grep '^# REPO ' "$MANIFEST_FILE" 2>/dev/null | head -n 1 | cut -d' ' -f3- || true)"
    if [[ -z "$installed_repo" ]]; then
        installed_repo="${REPO_DIR:-}"
    fi

    # 1. Stop and disable system services
    while IFS=$'\t' read -r type target extra; do
        if [[ "$type" == "SYS_UNIT" ]]; then
            if [[ "$target" == "greetd.service" ]]; then
                disable_greetd_safely
            else
                log "Stopping and disabling system service: $target"
                if [[ "$DRY_RUN" -eq 1 ]]; then
                    printf '[DRY-RUN] sudo systemctl disable --now %s\n' "$target"
                    printf '[DRY-RUN] sudo rm -f /etc/systemd/system/%s\n' "$target"
                else
                    sudo systemctl disable --now "$target" 2>/dev/null || true
                    sudo rm -f "/etc/systemd/system/$target" 2>/dev/null || true
                fi
            fi
        fi
    done < "$MANIFEST_FILE"

    # 2. Stop and disable Inerium user services
    while IFS=$'\t' read -r type target extra; do
        if [[ "$type" == "USER_UNIT" ]]; then
            # Only disable/remove Inerium units, leaving standard system units (waybar, hypridle, mako, etc.) intact
            case "$target" in
                inerium-*|hypr-ipc-bridge*|gdrive*)
                    log "Stopping and disabling user service: $target"
                    if [[ "$DRY_RUN" -eq 1 ]]; then
                        printf '[DRY-RUN] systemctl --user disable --now %s\n' "$target"
                        printf '[DRY-RUN] rm -f %s/.config/systemd/user/%s\n' "$HOME" "$target"
                    else
                        systemctl --user disable --now "$target" 2>/dev/null || true
                        rm -f "$HOME/.config/systemd/user/$target"
                    fi
                    ;;
                *)
                    local u_path="$HOME/.config/systemd/user/$target"
                    if [[ -L "$u_path" ]]; then
                        local u_dest
                        u_dest="$(readlink -f "$u_path" 2>/dev/null || true)"
                        if [[ "$u_dest" == *inerium* || "$u_dest" == *kanagawa* || ( -n "$installed_repo" && "$u_dest" == "$installed_repo"* ) ]]; then
                            log "Removing Inerium unit symlink: $u_path"
                            if [[ "$DRY_RUN" -eq 1 ]]; then
                                printf '[DRY-RUN] rm -f %s\n' "$u_path"
                            else
                                rm -f "$u_path"
                            fi
                        fi
                    fi
                    ;;
            esac
        fi
    done < "$MANIFEST_FILE"

    # Remove waybar bridge drop-in
    local waybar_bridge_dropin="$HOME/.config/systemd/user/waybar.service.d/bridge.conf"
    if [[ -e "$waybar_bridge_dropin" || -L "$waybar_bridge_dropin" ]]; then
        log "Removing Waybar bridge drop-in: $waybar_bridge_dropin"
        if [[ "$DRY_RUN" -eq 1 ]]; then
            printf '[DRY-RUN] rm -f %s\n' "$waybar_bridge_dropin"
        else
            rm -f "$waybar_bridge_dropin"
            rmdir "$HOME/.config/systemd/user/waybar.service.d" 2>/dev/null || true
        fi
    fi

    # Clean any dangling user unit symlinks pointing to $installed_repo
    for u_link in "$HOME/.config/systemd/user"/* "$HOME/.config/systemd/user"/*/*; do
        [[ -L "$u_link" ]] || continue
        local u_dest raw_dest
        u_dest="$(readlink -f "$u_link" 2>/dev/null || true)"
        raw_dest="$(readlink "$u_link" 2>/dev/null || true)"
        if [[ -n "$installed_repo" && ( "$u_dest" == "$installed_repo"* || "$raw_dest" == "$installed_repo"* ) ]] || [[ "$u_dest" == *inerium* || "$u_dest" == *kanagawa* || "$raw_dest" == *inerium* || "$raw_dest" == *kanagawa* ]]; then
            if [[ "$DRY_RUN" -eq 1 ]]; then
                printf '[DRY-RUN] rm -f %s\n' "$u_link"
            else
                rm -f "$u_link"
            fi
        fi
    done

    # Unmask masked user units (hyprpolkitagent, at-spi2-registryd, at-spi-dbus-bus)
    for u in hyprpolkitagent.service at-spi2-registryd.service at-spi-dbus-bus.service; do
        local masked_unit="$HOME/.config/systemd/user/$u"
        if [[ -L "$masked_unit" ]]; then
            local raw_target
            raw_target="$(readlink "$masked_unit" 2>/dev/null || true)"
            if [[ "$raw_target" == "/dev/null" ]]; then
                log "Unmasking $u (removing /dev/null symlink)..."
                if [[ "$DRY_RUN" -eq 1 ]]; then
                    printf '[DRY-RUN] rm -f %s\n' "$masked_unit"
                else
                    rm -f "$masked_unit"
                fi
            fi
        fi
        if [[ "$DRY_RUN" -eq 1 ]]; then
            printf '[DRY-RUN] systemctl --user unmask %s\n' "$u"
        else
            systemctl --user unmask "$u" 2>/dev/null || true
        fi
    done
    if [[ "$DRY_RUN" -eq 0 ]]; then
        systemctl --user daemon-reload 2>/dev/null || true
    fi

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
            RENDER|CONFIG)
                if [[ "$target" == "$HOME/.config/"* || "$target" == "${XDG_CONFIG_HOME:-$HOME/.config}/"* ]]; then
                    # Preserve user desktop configurations so Hyprland session remains usable
                    continue
                fi
                if [[ -f "$target" ]]; then
                    log "Removing configuration: $target"
                    if [[ "$DRY_RUN" -eq 1 ]]; then
                        printf '[DRY-RUN] rm -f %s\n' "$target"
                    else
                        rm -f "$target"
                    fi
                fi
                ;;
            SYS_FILE)
                case "$target" in
                    /etc/greetd/config.toml|/etc/greetd/environments|/etc/pam.d/greetd)
                        continue
                        ;;
                esac
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

    # 6. Clean cloned repository if installed in user data directory (~/.local/share/inerium/shell or legacy ~/.local/share/inerium-shell)
    local user_shell_dir="${XDG_DATA_HOME:-$HOME/.local/share}/inerium/shell"
    local legacy_user_shell_dir="${XDG_DATA_HOME:-$HOME/.local/share}/inerium-shell"

    for repo_candidate in "$user_shell_dir" "$legacy_user_shell_dir"; do
        if [[ -n "$installed_repo" && -d "$repo_candidate" ]]; then
            local can_installed can_cand
            can_installed="$(realpath "$installed_repo" 2>/dev/null || true)"
            can_cand="$(realpath "$repo_candidate" 2>/dev/null || true)"
            if [[ -n "$can_installed" && "$can_installed" == "$can_cand" ]]; then
                log "Removing cloned repository in user data directory: $repo_candidate"
                if [[ "$DRY_RUN" -eq 1 ]]; then
                    printf '[DRY-RUN] rm -rf %s\n' "$repo_candidate"
                else
                    rm -rf "$repo_candidate"
                fi
            fi
        fi
    done
    log "Inerium Shell uninstallation complete."
    printf '\n'
    echo "============================================================================"
    echo "  Inerium Shell Uninstallation Summary / Сводка удаления Inerium Shell"
    echo "============================================================================"
    echo "  [EN] What was removed:"
    echo "       - Inerium binaries (~/.local/bin/inerium*), multicall symlinks, and desktop entries"
    echo "       - Inerium systemd background services and bridge drop-ins"
    echo "       - Greeter daemon (/usr/local/bin/inerium-greeter) and greetd autostart disabled"
    echo "  [EN] Preserved:"
    echo "       - All desktop configurations in ~/.config/ (hypr, waybar, kitty, rofi, mako)"
    echo "         Your Hyprland desktop session remains completely usable."
    echo "  [EN] Important:"
    echo "       - Remember to enable your preferred display manager if greetd was disabled:"
    echo "         sudo systemctl enable sddm.service (or gdm / lightdm / tty autologin)"
    echo "----------------------------------------------------------------------------"
    echo "  [RU] Что удалено:"
    echo "       - Бинарники Inerium (~/.local/bin/inerium*), симлинки утилит и .desktop файлы"
    echo "       - Фоновые службы Inerium systemd и дроп-ины мостов"
    echo "       - Демон экрана входа (/usr/local/bin/inerium-greeter), автозапуск greetd отключён"
    echo "  [RU] Сохранено:"
    echo "       - Все конфигурационные файлы в ~/.config/ (hypr, waybar, kitty, rofi, mako)"
    echo "         Ваша сессия Hyprland остаётся полностью функциональной."
    echo "  [RU] Важно:"
    echo "       - Не забудьте включить нужный дисплейный менеджер, если greetd был отключён:"
    echo "         sudo systemctl enable sddm.service (или gdm / lightdm / автологин в TTY)"
    echo "============================================================================"
}

detect_installed_addons_and_features() {
    if [[ -f "$MANIFEST_FILE" ]]; then
        while IFS=$'\t' read -r type target extra; do
            case "$type" in
                ADDON)
                    case "$target" in
                        gdrive)        WITH_GDRIVE=1 ;;
                        converter)     WITH_CONVERTER=1 ;;
                        camera)        WITH_CAMERA=1 ;;
                        msi-ec)        WITH_MSI_EC=1 ;;
                        fan-regulator) WITH_FAN_REGULATOR=1; WITH_MSI_EC=1 ;;
                        novpn)         WITH_NOVPN=1 ;;
                        github)        WITH_GITHUB=1 ;;
                        greeter)       WITH_GREETER=1 ;;
                    esac
                    ;;
                SYS_FILE)
                    if [[ "$target" == *inerium-greeter* ]]; then
                        WITH_GREETER=1
                    fi
                    ;;
            esac
        done < "$MANIFEST_FILE"
    fi

    # Live system fallback checks (considering broken symlinks via -e or -L)
    [[ -e "$HOME/.config/systemd/user/gdrive.service" || -L "$HOME/.config/systemd/user/gdrive.service" ]] && WITH_GDRIVE=1
    [[ -e "$HOME/.local/share/kio/servicemenus/converter-document.desktop" || -L "$HOME/.local/share/kio/servicemenus/converter-document.desktop" ]] && WITH_CONVERTER=1
    [[ -e "$HOME/.config/systemd/user/inerium-camera-bridge.service" || -L "$HOME/.config/systemd/user/inerium-camera-bridge.service" ]] && WITH_CAMERA=1
    [[ -e "${XDG_CONFIG_HOME:-$HOME/.config}/inerium/first-login.d/10-github" || -L "${XDG_CONFIG_HOME:-$HOME/.config}/inerium/first-login.d/10-github" ]] && WITH_GITHUB=1

    # False-positive fixes:
    # MSI EC: verify loaded module conf or udev rule, NOT sysfs platform node
    if [[ -f "/etc/modules-load.d/msi-ec.conf" || -f "/etc/udev/rules.d/99-msi-ec.rules" ]]; then
        WITH_MSI_EC=1
    fi
    if [[ -f "/etc/systemd/system/msi-fan-regulator.service" ]]; then
        WITH_FAN_REGULATOR=1
        WITH_MSI_EC=1
    fi
    if [[ -f "/etc/systemd/system/novpn.service" ]] || command -v novpn >/dev/null 2>&1; then
        WITH_NOVPN=1
    fi

    # Greeter: verify inerium-greeter configuration specifically, NOT any other DM
    if [[ -f "/etc/greetd/inerium-greeter.json" ]] || ( [[ -f "/etc/greetd/config.toml" ]] && grep -q "inerium-greeter" /etc/greetd/config.toml 2>/dev/null ); then
        WITH_GREETER=1
    fi
}
