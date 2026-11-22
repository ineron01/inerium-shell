#!/usr/bin/env bash
# ============================================================================
# scripts/install/50-configs.sh — Configs, templates, overlays, and services
# ============================================================================

is_russian_env() {
    if [[ "$INSTALL_LANG" == "en" ]]; then
        return 1
    fi
    if [[ "$INSTALL_LANG" == "ru" ]]; then
        return 0
    fi
    if [[ "${LANG:-}" == *ru_* || "${LC_ALL:-}" == *ru_* ]]; then
        return 0
    fi
    if command -v localectl >/dev/null 2>&1; then
        local x11_layout
        x11_layout="$(localectl status 2>/dev/null | awk -F: '/X11 Layout/ {gsub(/^[ \t]+|[ \t]+$/, "", $2); print $2; exit}' || true)"
        if [[ "$x11_layout" == *"ru"* ]]; then
            return 0
        fi
    fi
    return 1
}

deploy_user_scripts_and_menus() {
    log "Deploying user scripts & desktop entries..."
    run "mkdir -p '$HOME/.local/bin' '$HOME/.local/share/applications'"
    for script in "$REPO_DIR"/local/bin/*; do
        [[ -e "$script" ]] || continue
        name="$(basename "$script")"
        link_item "$script" "$HOME/.local/bin/$name"
    done
    for desktop in "$REPO_DIR"/local/share/applications/*.desktop; do
        [[ -e "$desktop" ]] || continue
        link_item "$desktop" "$HOME/.local/share/applications/$(basename "$desktop")"
    done

    run "mkdir -p '${XDG_CONFIG_HOME:-$HOME/.config}/inerium/first-login.d'"
    if [[ -d "$REPO_DIR/first-login.d" ]]; then
        for step in "$REPO_DIR"/first-login.d/*; do
            [[ -e "$step" ]] || continue
            local step_name
            step_name="$(basename "$step")"
            [[ "$step_name" == "10-github" ]] && continue  # Managed by --with-github
            [[ "$step_name" == "20-gdrive" ]] && continue  # Managed by --with-gdrive
            [[ "$step_name" == "30-novpn" ]] && continue   # Managed by --with-novpn
            link_item "$step" "${XDG_CONFIG_HOME:-$HOME/.config}/inerium/first-login.d/$step_name"
        done
    fi

    if is_russian_env; then
        install_root_if_changed 755 "$REPO_DIR/system/usr/local/bin/inerium-desktop-indexer" /usr/local/bin/inerium-desktop-indexer
    fi
    if [[ -f "$REPO_DIR/system/etc/udev/rules.d/99-inerium-hw.rules" ]]; then
        install_root_if_changed 644 "$REPO_DIR/system/etc/udev/rules.d/99-inerium-hw.rules" /etc/udev/rules.d/99-inerium-hw.rules
    fi
}

INPUT_EXISTED=0
MONITORS_EXISTED=0

deploy_desktop_configs() {
    log "Installing desktop configurations into ~/.config..."

    INPUT_EXISTED=0
    MONITORS_EXISTED=0
    [[ -e "$HOME/.config/hypr/config/input.lua" ]] && INPUT_EXISTED=1
    [[ -e "$HOME/.config/hypr/config/monitors.lua" ]] && MONITORS_EXISTED=1

    # Migrate legacy directory symlinks from earlier releases to real directories
    migrate_legacy_dir_symlinks

    # 1. Environment.d
    run "mkdir -p '$HOME/.config/environment.d'"
    [[ -f "$REPO_DIR/config/environment.d/10-theme.conf" ]] && \
        deploy_config_file "$REPO_DIR/config/environment.d/10-theme.conf" "$HOME/.config/environment.d/10-theme.conf"

    # 2. GTK 3 & GTK 4
    run "mkdir -p '$HOME/.config/gtk-3.0' '$HOME/.config/gtk-4.0'"
    for f in gtk.css settings.ini; do
        [[ -f "$REPO_DIR/config/gtk-3.0/$f" ]] && deploy_config_file "$REPO_DIR/config/gtk-3.0/$f" "$HOME/.config/gtk-3.0/$f"
        [[ -f "$REPO_DIR/config/gtk-4.0/$f" ]] && deploy_config_file "$REPO_DIR/config/gtk-4.0/$f" "$HOME/.config/gtk-4.0/$f"
    done

    # 3. Kitty Terminal
    run "mkdir -p '$HOME/.config/kitty/themes'"
    deploy_config_file "$REPO_DIR/config/kitty/kitty.conf" "$HOME/.config/kitty/kitty.conf"
    for f in "$REPO_DIR"/config/kitty/themes/*.conf; do
        [[ -f "$f" ]] || continue
        deploy_config_file "$f" "$HOME/.config/kitty/themes/$(basename "$f")"
    done

    # 4. Mako Notification Daemon
    run "mkdir -p '$HOME/.config/mako'"
    deploy_config_file "$REPO_DIR/config/mako/config" "$HOME/.config/mako/config"

    # 5. Rofi Application Launcher
    run "mkdir -p '$HOME/.config/rofi'"
    for f in "$REPO_DIR"/config/rofi/*.rasi; do
        [[ -f "$f" ]] || continue
        deploy_config_file "$f" "$HOME/.config/rofi/$(basename "$f")"
    done

    # 6. UWSM Session Manager
    run "mkdir -p '$HOME/.config/uwsm'"
    for f in env env-hyprland; do
        [[ -f "$REPO_DIR/config/uwsm/$f" ]] && deploy_config_file "$REPO_DIR/config/uwsm/$f" "$HOME/.config/uwsm/$f"
    done

    # 7. Waybar Panel
    run "mkdir -p '$HOME/.config/waybar'"
    local waybar_existed=0
    [[ -e "$HOME/.config/waybar/config.jsonc" ]] && waybar_existed=1

    for f in config.jsonc style.css colors.css; do
        [[ -f "$REPO_DIR/config/waybar/$f" ]] && deploy_config_file "$REPO_DIR/config/waybar/$f" "$HOME/.config/waybar/$f"
    done

    # Fresh install (or explicit UPDATE_CONFIGS=1): detect distro logo once
    if [[ "$waybar_existed" -eq 0 || "${UPDATE_CONFIGS:-0}" -eq 1 ]]; then
        local distro_id="" distro_icon="󰣇"
        if [[ -f /etc/os-release ]]; then
            distro_id="$(grep -E '^ID=' /etc/os-release | cut -d= -f2 | tr -d '"' | tr '[:upper:]' '[:lower:]')"
        fi
        case "$distro_id" in
            cachyos)     distro_icon="" ;;
            endeavouros) distro_icon="" ;;
            manjaro)     distro_icon="" ;;
            garuda)      distro_icon="" ;;
            artix)       distro_icon="" ;;
            archcraft)   distro_icon="" ;;
            arcolinux)   distro_icon="" ;;
            blackarch)   distro_icon="" ;;
            *)           distro_icon="󰣇" ;;
        esac
        if [[ "$DRY_RUN" -eq 0 && -f "$HOME/.config/waybar/config.jsonc" && ! -L "$HOME/.config/waybar/config.jsonc" ]]; then
            sed -i -E '/"custom\/launcher": \{/,/\}/ s/("format": ")[^"]*(")/\1'"$distro_icon"'\2/' "$HOME/.config/waybar/config.jsonc"
            log "Configured initial Waybar distro icon: $distro_icon ($distro_id)"
        fi
    fi

    # 8. Btop System Monitor
    run "mkdir -p '$HOME/.config/btop'"
    deploy_config_file "$REPO_DIR/config/btop/btop.conf" "$HOME/.config/btop/btop.conf"

    # 9. Fish shell (preserve runtime fish_variables)
    run "mkdir -p '$HOME/.config/fish/functions'"
    deploy_config_file "$REPO_DIR/config/fish/config.fish" "$HOME/.config/fish/config.fish" 644 "user"
    if [[ -d "$REPO_DIR/config/fish/functions" ]]; then
        for func in "$REPO_DIR"/config/fish/functions/*.fish; do
            [[ -f "$func" ]] || continue
            deploy_config_file "$func" "$HOME/.config/fish/functions/$(basename "$func")"
        done
    fi

    # 10. KDE / Dolphin integration
    deploy_config_file "$REPO_DIR/config/kdeglobals" "$HOME/.config/kdeglobals"
    deploy_config_file "$REPO_DIR/config/dolphinrc" "$HOME/.config/dolphinrc"

    # 11. Qt5 / Qt6 colors & SwayOSD
    run "mkdir -p '$HOME/.config/qt5ct/colors' '$HOME/.config/qt6ct/colors' '$HOME/.config/swayosd'"
    for c in "$REPO_DIR"/config/qt5ct/colors/*.conf; do
        [[ -f "$c" ]] && deploy_config_file "$c" "$HOME/.config/qt5ct/colors/$(basename "$c")"
    done
    for c in "$REPO_DIR"/config/qt6ct/colors/*.conf; do
        [[ -f "$c" ]] && deploy_config_file "$c" "$HOME/.config/qt6ct/colors/$(basename "$c")"
    done
    deploy_config_file "$REPO_DIR/config/swayosd/style.css" "$HOME/.config/swayosd/style.css"
    local tpl_only_missing=0
    [[ "$UPDATE_CONFIGS_MODE" == "none" ]] && tpl_only_missing=1
    render_template "$REPO_DIR/config/qt5ct/qt5ct.conf.template" "$HOME/.config/qt5ct/qt5ct.conf" "$tpl_only_missing"
    render_template "$REPO_DIR/config/qt6ct/qt6ct.conf.template" "$HOME/.config/qt6ct/qt6ct.conf" "$tpl_only_missing"
    render_template "$REPO_DIR/config/swayosd/config.toml.template" "$HOME/.config/swayosd/config.toml" "$tpl_only_missing"

    # 12. Themes (Base in ~/.config/inerium/themes, Extra in ~/.local/share/inerium/themes/extra)
    run "mkdir -p '$HOME/.local/share/color-schemes'"
    run "mkdir -p '$HOME/.local/share/inerium/themes/extra'"
    run "mkdir -p '$HOME/.config/inerium/themes'"
    local themes_sentinel="$HOME/.local/share/inerium/.themes_initialized"
    if [[ ! -f "$themes_sentinel" || "${RESTORE_ASSETS:-0}" -eq 1 || "${IS_UPDATE:-0}" -ne 1 ]]; then
        # Base themes (Dark & Light)
        for t in "$REPO_DIR"/themes/*.toml; do
            [[ -f "$t" ]] || continue
            deploy_config_file "$t" "$HOME/.config/inerium/themes/$(basename "$t")"
        done
        # Extra themes pool for inerium-setup and inerium-theme install
        for t in "$REPO_DIR"/themes/extra/*.toml; do
            [[ -f "$t" ]] || continue
            deploy_config_file "$t" "$HOME/.local/share/inerium/themes/extra/$(basename "$t")"
        done
        if [[ -f "$REPO_DIR/themes/template.toml.example" ]]; then
            deploy_config_file "$REPO_DIR/themes/template.toml.example" "$HOME/.config/inerium/themes/template.toml.example"
        fi
        run "touch '$themes_sentinel'"
    else
        log "Preserving user themes (pass --restore-assets to restore standard themes)"
    fi
    # Clean up legacy 'examples' symlink
    [[ -L "$HOME/.config/inerium/themes/examples" ]] && run "rm -f '$HOME/.config/inerium/themes/examples'"
    deploy_config_file "$REPO_DIR/local/share/color-schemes/Inerium-Dragon.colors" "$HOME/.local/share/color-schemes/Inerium-Dragon.colors"
    deploy_config_file "$REPO_DIR/local/share/color-schemes/Inerium-Lotus.colors" "$HOME/.local/share/color-schemes/Inerium-Lotus.colors"

    # 13. Hyprland configuration
    run "mkdir -p '$HOME/.config/hypr/config'"
    for f in hyprland.lua hyprlock.conf hypridle.conf xdph.conf; do
        [[ -f "$REPO_DIR/config/hypr/$f" ]] || continue
        local cat="ui"
        [[ "$f" == "hypridle.conf" ]] && cat="user"
        deploy_config_file "$REPO_DIR/config/hypr/$f" "$HOME/.config/hypr/$f" 644 "$cat"
    done
    for f in "$REPO_DIR"/config/hypr/config/*.lua; do
        [[ -f "$f" ]] || continue
        base="$(basename "$f")"
        local cat="user"
        case "$base" in
            animations.lua|decorations.lua|colors.lua)
                cat="ui"
                ;;
        esac
        deploy_config_file "$f" "$HOME/.config/hypr/config/$base" 644 "$cat"
    done
    render_template "$REPO_DIR/config/hypr/hyprpaper.conf.template" "$HOME/.config/hypr/hyprpaper.conf" 1

    local hyprlock_i18n="$HOME/.config/hypr/hyprlock-i18n.conf"
    manifest_record_config "$hyprlock_i18n"
    if [[ ! -f "$hyprlock_i18n" ]]; then
        if [[ "$DRY_RUN" -eq 1 ]]; then
            printf '[DRY-RUN] create %s\n' "$hyprlock_i18n"
        else
            if is_russian_env; then
                cat > "$hyprlock_i18n" <<'EOF'
# Auto-generated by Inerium Setup
$LOCK_PLACEHOLDER = <span foreground="$LOCK_PLACEHOLDER_COLOR">Пароль</span>
$LOCK_FAIL = <span foreground="$LOCK_FAIL_TEXT_COLOR">Неверный пароль · попытка $ATTEMPTS</span>
$LOCK_CHECK = <span foreground="$LOCK_CHECK_TEXT_COLOR">Проверка…</span>
$LOCK_DATE_CMD = LC_TIME=ru_RU.UTF-8 date +"%A, %-d %B"
EOF
            else
                cat > "$hyprlock_i18n" <<'EOF'
# Generated by Inerium Installer
# Empty override: defaults from hyprlock.conf will be used
EOF
            fi
        fi
    fi

    local locale_conf="$HOME/.config/locale.conf"
    if [[ -n "${INSTALL_LANG:-}" && ( "$INSTALL_LANG" == "ru" || "$INSTALL_LANG" == "en" ) ]]; then
        manifest_record_config "$locale_conf"
        if [[ ! -f "$locale_conf" || "${IS_UPDATE:-0}" -eq 0 || "${EXPLICIT_LANG:-0}" -eq 1 ]]; then
            local loc_content
            if [[ "$INSTALL_LANG" == "ru" ]]; then
                loc_content="LANG=ru_RU.UTF-8"$'\n'"LC_TIME=ru_RU.UTF-8"$'\n'
            else
                loc_content="LANG=en_US.UTF-8"$'\n'"LC_TIME=en_US.UTF-8"$'\n'
            fi
            if [[ "$DRY_RUN" -eq 1 ]]; then
                printf '[DRY-RUN] write %s -> %s\n' "$INSTALL_LANG" "$locale_conf"
            else
                mkdir -p "$(dirname "$locale_conf")"
                printf '%s' "$loc_content" > "$locale_conf"
            fi
        fi
    fi

    local ws_count_file="$HOME/.config/hypr/config/workspaces_count"
    if [[ -n "$WORKSPACES_PER_MON" && "$WORKSPACES_PER_MON" =~ ^[1-9]$ ]]; then
        if [[ "$DRY_RUN" -eq 1 ]]; then
            printf '[DRY-RUN] write %s -> %s\n' "$WORKSPACES_PER_MON" "$ws_count_file"
        else
            printf '%s\n' "$WORKSPACES_PER_MON" > "$ws_count_file"
        fi
    fi

    local current_theme_file="${XDG_CONFIG_HOME:-$HOME/.config}/inerium/current.json"
    if [[ "$DRY_RUN" -eq 0 ]]; then
        if [[ ! -f "$current_theme_file" ]]; then
            if command -v inerium-theme >/dev/null 2>&1; then
                inerium-theme set kanagawa-dragon >/dev/null 2>&1 || true
            elif command -v plasma-apply-colorscheme >/dev/null 2>&1; then
                plasma-apply-colorscheme Inerium-Dragon >/dev/null 2>&1 || true
            fi
        elif [[ "$UPDATE_CONFIGS_MODE" == "ui" || "$UPDATE_CONFIGS_MODE" == "all" ]]; then
            if command -v inerium-theme >/dev/null 2>&1; then
                log "Reloading active theme configuration..."
                inerium-theme reload >/dev/null 2>&1 || true
            fi
        fi
    fi
}

detect_and_generate_local_overlays() {
    local env_local="$HOME/.config/hypr/config/environment_local.lua"
    local idle_local="$HOME/.config/hypr/hypridle_local.conf"
    local idle_dropin="$HOME/.config/systemd/user/hypridle.service.d/local.conf"
    local user_input="$HOME/.config/hypr/config/input.lua"
    local user_monitors="$HOME/.config/hypr/config/monitors.lua"

    # 1. GPU detection
    if [[ ! -e "$env_local" ]]; then
        local has_nv=0 has_amd=0 has_intel=0 v vf
        for vf in /sys/class/drm/card*/device/vendor; do
            [[ -r "$vf" ]] || continue
            v="$(cat "$vf" 2>/dev/null || true)"
            case "$v" in
                0x10de) has_nv=1 ;;
                0x1002) has_amd=1 ;;
                0x8086) has_intel=1 ;;
            esac
        done
        if [[ "$DRY_RUN" -eq 1 ]]; then
            printf '[DRY-RUN] generate GPU overlay %s (nv=%d amd=%d intel=%d)\n' "$env_local" "$has_nv" "$has_amd" "$has_intel"
        else
            mkdir -p "$(dirname "$env_local")"
            {
                echo "-- Auto-generated by inerium-shell hardware detection (gitignored)"
                if [[ "$has_nv" -eq 1 && "$has_amd" -eq 0 && "$has_intel" -eq 0 ]]; then
                    echo 'hl.env("LIBVA_DRIVER_NAME", "nvidia")'
                    echo 'hl.env("__GLX_VENDOR_LIBRARY_NAME", "nvidia")'
                    echo 'hl.env("NVD_BACKEND", "direct")'
                    echo '-- Note: GBM_BACKEND is omitted for Aquamarine / modern Hyprland compatibility (explicit sync).'
                elif [[ "$has_amd" -eq 1 ]]; then
                    echo 'hl.env("LIBVA_DRIVER_NAME", "radeonsi")'
                    echo 'hl.env("VDPAU_DRIVER", "radeonsi")'
                    if [[ "$has_nv" -eq 1 ]]; then
                        echo '-- Hybrid AMD+NVIDIA detected; iGPU handles Wayland display, use prime-run for dGPU offload.'
                    fi
                elif [[ "$has_intel" -eq 1 ]]; then
                    echo 'hl.env("LIBVA_DRIVER_NAME", "iHD")'
                    if [[ "$has_nv" -eq 1 ]]; then
                        echo '-- Hybrid Intel+NVIDIA detected; iGPU handles Wayland display, use prime-run for dGPU offload.'
                    fi
                fi
            } > "${env_local}.tmp" && mv -f "${env_local}.tmp" "$env_local"
        fi
    fi

    # 2. Chassis detection (laptop battery vs desktop) -> hypridle_local.conf
    if [[ ! -e "$idle_local" ]]; then
        local is_laptop=0
        if compgen -G "/sys/class/power_supply/BAT*" >/dev/null 2>&1; then
            is_laptop=1
        fi
        if [[ "$DRY_RUN" -eq 1 ]]; then
            printf '[DRY-RUN] generate idle overlay %s (laptop=%d)\n' "$idle_local" "$is_laptop"
        else
            mkdir -p "$(dirname "$idle_local")"
            if [[ "$is_laptop" -eq 1 ]]; then
                cat > "${idle_local}.tmp" <<'EOF'
# Auto-generated by inerium-shell (laptop chassis, gitignored)
general {
    lock_cmd = pidof hyprlock || hyprlock
    before_sleep_cmd = loginctl lock-session
    after_sleep_cmd = hyprctl dispatch dpms on
}

listener {
    timeout = 150
    on-timeout = brightnessctl -s set 20%
    on-resume = brightnessctl -r
}

listener {
    timeout = 300
    on-timeout = loginctl lock-session
}

listener {
    timeout = 330
    on-timeout = hyprctl dispatch dpms off
    on-resume = hyprctl dispatch dpms on
}

listener {
    timeout = 600
    on-timeout = systemctl suspend
}
EOF
            else
                cat > "${idle_local}.tmp" <<'EOF'
# Auto-generated by inerium-shell (desktop chassis, gitignored)
general {
    lock_cmd = pidof hyprlock || hyprlock
    before_sleep_cmd = loginctl lock-session
    after_sleep_cmd = hyprctl dispatch dpms on
}

listener {
    timeout = 600
    on-timeout = loginctl lock-session
}

listener {
    timeout = 900
    on-timeout = hyprctl dispatch dpms off
    on-resume = hyprctl dispatch dpms on
}
EOF
            fi
            mv -f "${idle_local}.tmp" "$idle_local"
            mkdir -p "$(dirname "$idle_dropin")"
            printf '[Service]\nExecStart=\nExecStart=/usr/bin/hypridle -c %s\n' "$idle_local" > "${idle_dropin}.tmp"
            mv -f "${idle_dropin}.tmp" "$idle_dropin"
        fi
    fi

    # 3. Keyboard layout detection
    local input_existed="${INPUT_EXISTED:-0}"
    local monitors_existed="${MONITORS_EXISTED:-0}"

    local kb_layout="us,ru"
    local kb_options="grp:alt_shift_toggle"
    if command -v localectl >/dev/null 2>&1; then
        local x11_layout x11_options
        x11_layout="$(localectl status 2>/dev/null | awk -F: '/X11 Layout/ {gsub(/^[ \t]+|[ \t]+$/, "", $2); print $2; exit}' || true)"
        if [[ -n "$x11_layout" && "$x11_layout" != "n/a" && "$x11_layout" != "us" ]]; then
            if [[ "$x11_layout" == *","* ]]; then
                kb_layout="$x11_layout"
            else
                kb_layout="us,$x11_layout"
            fi
        fi
        x11_options="$(localectl status 2>/dev/null | awk -F: '/X11 Options/ {gsub(/^[ \t]+|[ \t]+$/, "", $2); print $2; exit}' || true)"
        if [[ -n "$x11_options" && "$x11_options" != "n/a" && "$x11_options" == *"grp:"* ]]; then
            kb_options="$x11_options"
        fi
    fi
    if [[ "$input_existed" -eq 1 && "${UPDATE_CONFIGS_MODE:-none}" != "all" ]]; then
        log "Preserving existing input config: $user_input"
    elif [[ "$DRY_RUN" -eq 1 ]]; then
        printf '[DRY-RUN] update input config %s (kb_layout=%s, kb_options=%s)\n' "$user_input" "$kb_layout" "$kb_options"
    else
        if [[ -f "$user_input" && ! -L "$user_input" ]]; then
            sed -i -E "s/kb_layout[ \t]*=[ \t]*\"[^\"]*\"/kb_layout     = \"${kb_layout}\"/" "$user_input"
            sed -i -E "s/kb_options[ \t]*=[ \t]*\"[^\"]*\"/kb_options    = \"${kb_options}\"/" "$user_input"
        fi
    fi

    # 4. HiDPI monitor scale detection
    local should_detect_monitors=0
    if [[ "$monitors_existed" -eq 0 || "${UPDATE_CONFIGS_MODE:-none}" == "all" || $(grep -c "INERIUM_TEMPLATE_MONITORS" "$user_monitors" 2>/dev/null || true) -gt 0 ]]; then
        should_detect_monitors=1
    fi
    if [[ "$should_detect_monitors" -eq 0 ]]; then
        log "Preserving existing monitors config: $user_monitors"
    elif [[ ! -L "$user_monitors" ]] && command -v hyprctl >/dev/null 2>&1 && command -v python3 >/dev/null 2>&1; then
        local mon_json
        mon_json="$(hyprctl monitors -j 2>/dev/null || true)"
        if [[ -n "$mon_json" && "$mon_json" == "["* ]]; then
            if [[ "$DRY_RUN" -eq 1 ]]; then
                printf '[DRY-RUN] detect and write displays to %s from hyprctl monitors -j\n' "$user_monitors"
            else
                python3 - "$mon_json" "$user_monitors" <<'PYEOF' || true
import json, math, os, sys
mons = json.loads(sys.argv[1])
lines = ["-- Monitor configuration\n-- Wiki: https://wiki.hypr.land/Configuring/Monitors/\n\n"]
for m in mons:
    name = m.get("name", "")
    if not name:
        continue
    w = int(m.get("width", 1920) or 1920)
    h = int(m.get("height", 1080) or 1080)
    pw = int(m.get("physicalWidth", 0) or 0)
    ph = int(m.get("physicalHeight", 0) or 0)
    scale = 1.0
    diag_in = math.hypot(pw, ph) / 25.4 if pw > 0 and ph > 0 else 0.0
    if w >= 3840:
        scale = 2.0 if (diag_in > 0 and diag_in <= 16.5) else 1.75
    elif w >= 2560 and diag_in > 0 and diag_in <= 16.5:
        scale = 1.5 if w >= 2880 else 1.25
    lines.append(
        f'hl.monitor({{\n    output   = "{name}",\n    mode     = "preferred",\n    position = "auto",\n    scale    = {scale},\n}})\n\n'
    )
lines.append(
    '-- Universal fallback for newly connected displays\nhl.monitor({\n    output   = "",\n    mode     = "preferred",\n    position = "auto",\n    scale    = 1.0,\n})\n'
)
tmp = sys.argv[2] + ".tmp"
with open(tmp, "w", encoding="utf-8") as f:
    f.writelines(lines)
os.replace(tmp, sys.argv[2])
PYEOF
            fi
        fi
    fi
}

deploy_wallpapers() {
    local target_dir="$HOME/.local/share/inerium/wallpapers"
    run "mkdir -p '$target_dir'"
    local sentinel="$target_dir/.wallpapers_initialized"

    if [[ -f "$sentinel" && "${RESTORE_ASSETS:-0}" -ne 1 && "${IS_UPDATE:-0}" -eq 1 ]]; then
        log "Preserving user wallpapers (pass --restore-assets to restore standard wallpapers)"
        return 0
    fi

    for wall in "$REPO_DIR"/assets/wallpapers/*; do
        [[ -f "$wall" ]] || continue
        wname="$(basename "$wall")"
        if [[ ! -e "$target_dir/$wname" ]]; then
            run "install -m 644 '$wall' '$target_dir/$wname'"
        fi
    done
    run "touch '$sentinel'"
}

fetch_silueta_model() {
    if [[ -s "$SILUETA_PATH" ]]; then
        log "Found silueta.onnx at $SILUETA_PATH"
    elif [[ "$FETCH_MODEL" -eq 1 ]]; then
        if [[ "$DRY_RUN" -eq 1 ]]; then
            printf '[DRY-RUN] download_file %s -> %s\n' "$SILUETA_URL" "$SILUETA_PATH"
        else
            mkdir -p "$(dirname "$SILUETA_PATH")"
            run_step "Downloading silueta.onnx (~43 MB)..." download_file "$SILUETA_URL" "$SILUETA_PATH" 300
        fi
    elif [[ "$WITH_CONVERTER" -eq 1 && "${IS_UPDATE:-0}" -ne 1 ]]; then
        # Fresh install with converter: download model
        if [[ "$DRY_RUN" -eq 1 ]]; then
            printf '[DRY-RUN] download_file %s -> %s\n' "$SILUETA_URL" "$SILUETA_PATH"
        else
            mkdir -p "$(dirname "$SILUETA_PATH")"
            run_step "Downloading silueta.onnx (~43 MB)..." download_file "$SILUETA_URL" "$SILUETA_PATH" 300
        fi
    else
        log "NOTE: silueta.onnx not found at $SILUETA_PATH."
        log "      Run with --fetch-model to download Dolphin AI background removal model."
    fi
}

configure_greeter() {
    for sess in "$REPO_DIR"/system/usr/share/wayland-sessions/*.desktop; do
        [[ -f "$sess" ]] || continue
        install_root_if_changed 644 "$sess" "/usr/share/wayland-sessions/$(basename "$sess")"
    done

    if [[ "$WITH_GREETER" -eq 1 ]]; then
        log "Configuring greetd + inerium-greeter..."
        if [[ "$INSTALL_MODE" == "bin" ]]; then
            if [[ -f "$RELEASE_DIST_DIR/inerium-greeter" ]]; then
                install_root_if_changed 755 "$RELEASE_DIST_DIR/inerium-greeter" /usr/local/bin/inerium-greeter
                manifest_record_system_file "/usr/local/bin/inerium-greeter"
            fi
        else
            if [[ "$SKIP_BUILD" -eq 0 ]]; then
                if [[ "$DRY_RUN" -eq 1 ]]; then
                    printf '[DRY-RUN] cargo build --release -p inerium-greeter\n'
                    printf '[DRY-RUN] install -m 755 target/release/inerium-greeter -> /usr/local/bin/inerium-greeter\n'
                else
                    run_live_step "Building inerium-greeter..." cargo build --release -p inerium-greeter --manifest-path "$REPO_DIR/Cargo.toml"
                    install_root_if_changed 755 "$REPO_DIR/target/release/inerium-greeter" /usr/local/bin/inerium-greeter
                    manifest_record_system_file "/usr/local/bin/inerium-greeter"
                fi
            elif [[ -f "$REPO_DIR/target/release/inerium-greeter" ]]; then
                install_root_if_changed 755 "$REPO_DIR/target/release/inerium-greeter" /usr/local/bin/inerium-greeter
                manifest_record_system_file "/usr/local/bin/inerium-greeter"
            fi
        fi
        if [[ "$SKIP_PKGS" -eq 0 ]]; then
            if [[ "$DRY_RUN" -eq 1 ]]; then
                printf '[DRY-RUN] install_pkgs greetd cage\n'
            else
                run_step "Installing greetd & cage..." install_pkgs greetd cage
            fi
        fi
        if [[ "$DRY_RUN" -eq 1 ]]; then
            printf '[DRY-RUN] install -d -m 1777 /var/cache/inerium\n'
        else
            run "sudo install -d -m 1777 /var/cache/inerium"
        fi
        if [[ -f /etc/greetd/config.toml ]] && ! cmp -s "$REPO_DIR/system/etc/greetd/config.toml" /etc/greetd/config.toml 2>/dev/null; then
            log "Backing up existing /etc/greetd/config.toml -> /etc/greetd/config.toml.bak.${BACKUP_STAMP}"
            run "sudo cp -a /etc/greetd/config.toml '/etc/greetd/config.toml.bak.${BACKUP_STAMP}'"
        fi
        if [[ -f /etc/pam.d/greetd ]] && ! cmp -s "$REPO_DIR/system/etc/pam.d/greetd" /etc/pam.d/greetd 2>/dev/null; then
            log "Backing up existing /etc/pam.d/greetd -> /etc/pam.d/greetd.bak.${BACKUP_STAMP}"
            run "sudo cp -a /etc/pam.d/greetd '/etc/pam.d/greetd.bak.${BACKUP_STAMP}'"
        fi
        install_root_if_changed 644 "$REPO_DIR/system/etc/greetd/config.toml" /etc/greetd/config.toml
        install_root_if_changed 644 "$REPO_DIR/system/etc/greetd/environments" /etc/greetd/environments
        if [[ -f "$REPO_DIR/system/etc/pam.d/greetd" ]]; then
            install_root_if_changed 644 "$REPO_DIR/system/etc/pam.d/greetd" /etc/pam.d/greetd
        fi
        if [[ "$DRY_RUN" -eq 1 ]]; then
            printf '[DRY-RUN] render %s -> /etc/greetd/inerium-greeter.json\n' "$REPO_DIR/system/etc/greetd/inerium-greeter.json.template"
        else
            local greeter_tmp
            greeter_tmp="$(mktemp)"
            sed -e "s|__HOME__|$HOME|g" -e "s|__USER__|$USER|g" "$REPO_DIR/system/etc/greetd/inerium-greeter.json.template" > "$greeter_tmp"
            install_root_if_changed 644 "$greeter_tmp" /etc/greetd/inerium-greeter.json
            rm -f "$greeter_tmp"
        fi
        for dm in sddm.service gdm.service lightdm.service; do
            if systemctl is-enabled "$dm" >/dev/null 2>&1; then
                log "Disabling display manager autostart (keeping current session running): $dm"
                run "sudo systemctl disable '$dm'"
            fi
        done
        run "sudo mkdir -p /var/lib/inerium-greeter"
        run "sudo chown -R greeter:greeter /var/lib/inerium-greeter 2>/dev/null || true"
        run "sudo chmod 0755 /var/lib/inerium-greeter"
        manifest_record_addon "greeter"
        if [[ -x /usr/local/bin/inerium-greeter || "$DRY_RUN" -eq 1 ]]; then
            if ! systemctl is-enabled --quiet greetd.service 2>/dev/null; then
                log "Enabling greetd.service..."
                run "sudo systemctl enable --force greetd.service"
            fi
            manifest_record_system_unit "greetd.service"
        else
            warn "inerium-greeter executable not found in /usr/local/bin; skipping greetd autostart enable."
        fi
    fi
}

enable_user_services_and_defaults() {
    # Systemd user unit symlinks
    run "mkdir -p '$HOME/.config/systemd/user/hypridle.service.d' '$HOME/.config/systemd/user/mako.service.d' '$HOME/.config/systemd/user/waybar.service.d'"
    for unit in "$REPO_DIR"/config/systemd/user/*.service "$REPO_DIR"/config/systemd/user/*.timer "$REPO_DIR"/config/systemd/user/*.path; do
        [[ -f "$unit" ]] || continue
        local uname
        uname="$(basename "$unit")"
        [[ "$uname" == "gdrive.service" ]] && continue
        [[ "$uname" == "inerium-camera-bridge.service" ]] && continue
        if ! is_russian_env && [[ "$uname" == "inerium-desktop-indexer"* ]]; then
            continue
        fi
        link_item "$unit" "$HOME/.config/systemd/user/$uname"
    done
    link_item "$REPO_DIR/config/systemd/user/hypridle.service.d/slice.conf" "$HOME/.config/systemd/user/hypridle.service.d/slice.conf"
    link_item "$REPO_DIR/config/systemd/user/mako.service.d/slice.conf" "$HOME/.config/systemd/user/mako.service.d/slice.conf"
    link_item "$REPO_DIR/config/systemd/user/waybar.service.d/bridge.conf" "$HOME/.config/systemd/user/waybar.service.d/bridge.conf"
    link_item "$REPO_DIR/config/systemd/user/waybar.service.d/slice.conf" "$HOME/.config/systemd/user/waybar.service.d/slice.conf"

    local USER_UNITS="cliphist.service cliphist-images.service hypr-ipc-bridge.service hyprpaper.service hypridle.service inerium-osd-watcher.service inerium-polkit.service inerium-popups.service inerium-updates.timer mako.service swayosd-server.service waybar.service"
    if is_russian_env; then
        USER_UNITS="$USER_UNITS inerium-desktop-indexer.path"
    fi
    for u in $USER_UNITS; do
        manifest_record_user_unit "$u"
    done
    local MASK_UNITS="hyprpolkitagent.service at-spi2-registryd.service at-spi-dbus-bus.service"

    if systemctl --user show-environment >/dev/null 2>&1; then
        log "Refreshing systemd user daemon & enabling user services..."
        run "systemctl --user daemon-reload"
        run "systemctl --user mask $MASK_UNITS >/dev/null 2>&1 || true"
        for u in $USER_UNITS; do
            if systemctl --user list-unit-files "$u" >/dev/null 2>&1 || [[ -f "$HOME/.config/systemd/user/$u" ]]; then
                run "systemctl --user enable '$u' >/dev/null 2>&1 || true"
            fi
        done
    else
        log "No active systemd --user D-Bus session (TTY/chroot); creating offline .wants/mask links..."
        for m in $MASK_UNITS; do
            run "ln -sfn /dev/null '$HOME/.config/systemd/user/$m'"
        done
        for u in $USER_UNITS; do
            local unit_src
            if ! unit_src="$(user_unit_path "$u")"; then
                log "WARNING: user unit $u not found; skipping offline enable"
                continue
            fi
            for target in $(sed -n 's/^WantedBy=//p' "$unit_src"); do
                run "mkdir -p '$HOME/.config/systemd/user/$target.wants'"
                run "ln -sfn '$unit_src' '$HOME/.config/systemd/user/$target.wants/$u'"
            done
        done
    fi

    # Set default web browser only if not already configured in the system
    if command -v xdg-settings >/dev/null 2>&1; then
        local cur_browser
        cur_browser="$(xdg-settings get default-web-browser 2>/dev/null || true)"
        if [[ -z "$cur_browser" ]]; then
            for b_desk in google-chrome.desktop firefox.desktop brave-browser.desktop chromium.desktop; do
                if [[ -f "/usr/share/applications/$b_desk" || -f "$HOME/.local/share/applications/$b_desk" ]]; then
                    run "xdg-settings set default-web-browser '$b_desk' >/dev/null 2>&1 || true"
                    run "xdg-mime default '$b_desk' x-scheme-handler/http >/dev/null 2>&1 || true"
                    run "xdg-mime default '$b_desk' x-scheme-handler/https >/dev/null 2>&1 || true"
                    run "xdg-mime default '$b_desk' text/html >/dev/null 2>&1 || true"
                    break
                fi
            done
        fi
    fi

    # Set default file manager for Inerium: prefer Dolphin if user hasn't explicitly configured inode/directory
    if command -v xdg-mime >/dev/null 2>&1; then
        local user_mime="${XDG_CONFIG_HOME:-$HOME/.config}/mimeapps.list"
        if ! grep -q "^inode/directory=" "$user_mime" 2>/dev/null; then
            for fm_desk in org.kde.dolphin.desktop org.gnome.Nautilus.desktop thunar.desktop nemo.desktop pcmanfm-qt.desktop pcmanfm.desktop; do
                if [[ -f "/usr/share/applications/$fm_desk" || -f "$HOME/.local/share/applications/$fm_desk" ]]; then
                    run "xdg-mime default '$fm_desk' inode/directory >/dev/null 2>&1 || true"
                    break
                fi
            done
        fi
    fi
}

refresh_desktop_caches() {
    log "Refreshing desktop caches & environment..."
    if command -v kbuildsycoca6 >/dev/null 2>&1; then
        run "kbuildsycoca6 --noincremental >/dev/null 2>&1 || true"
    fi
    if command -v update-desktop-database >/dev/null 2>&1; then
        run "update-desktop-database '$HOME/.local/share/applications' >/dev/null 2>&1 || true"
    fi
    if command -v udevadm >/dev/null 2>&1; then
        run "sudo udevadm control --reload-rules >/dev/null 2>&1 || true"
        run "sudo udevadm trigger >/dev/null 2>&1 || true"
    fi
}
