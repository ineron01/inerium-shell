#!/usr/bin/env bash
# ============================================================================
# scripts/install/50-configs.sh — Configs, templates, overlays, and services
# ============================================================================

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
            [[ "$step_name" == "20-gdrive" ]] && continue  # Managed by --with-gdrive
            link_item "$step" "${XDG_CONFIG_HOME:-$HOME/.config}/inerium/first-login.d/$step_name"
        done
    fi

    install_root_if_changed 755 "$REPO_DIR/system/usr/local/bin/inerium-desktop-indexer" /usr/local/bin/inerium-desktop-indexer
    install_sudoers_template "$REPO_DIR/system/etc/sudoers.d/inerium-desktop-indexer.template" /etc/sudoers.d/inerium-desktop-indexer
    if [[ -f "$REPO_DIR/system/etc/udev/rules.d/99-inerium-hw.rules" ]]; then
        install_root_if_changed 644 "$REPO_DIR/system/etc/udev/rules.d/99-inerium-hw.rules" /etc/udev/rules.d/99-inerium-hw.rules
    fi
}

deploy_desktop_configs() {
    log "Linking desktop configurations into ~/.config..."
    for dir in environment.d gtk-3.0 gtk-4.0 kitty mako rofi uwsm waybar; do
        link_item "$REPO_DIR/config/$dir" "$HOME/.config/$dir"
    done
    link_item "$REPO_DIR/config/btop/btop.conf" "$HOME/.config/btop/btop.conf"

    # Fish shell (preserve runtime fish_variables)
    run "mkdir -p '$HOME/.config/fish/functions'"
    link_item "$REPO_DIR/config/fish/config.fish" "$HOME/.config/fish/config.fish"
    for func in "$REPO_DIR"/config/fish/functions/*.fish; do
        [[ -f "$func" ]] || continue
        link_item "$func" "$HOME/.config/fish/functions/$(basename "$func")"
    done

    # Qt5/Qt6 & SwayOSD
    link_item "$REPO_DIR/config/qt5ct/colors" "$HOME/.config/qt5ct/colors"
    link_item "$REPO_DIR/config/qt6ct/colors" "$HOME/.config/qt6ct/colors"
    link_item "$REPO_DIR/config/swayosd/style.css" "$HOME/.config/swayosd/style.css"
    render_template "$REPO_DIR/config/qt5ct/qt5ct.conf.template" "$HOME/.config/qt5ct/qt5ct.conf"
    render_template "$REPO_DIR/config/qt6ct/qt6ct.conf.template" "$HOME/.config/qt6ct/qt6ct.conf"
    render_template "$REPO_DIR/config/swayosd/config.toml.template" "$HOME/.config/swayosd/config.toml"

    # Hyprland
    run "mkdir -p '$HOME/.config/hypr/config'"
    for f in AGENTS.md hypridle.conf hyprland.lua hyprlock.conf hyprtoolkit.conf xdph.conf; do
        [[ -f "$REPO_DIR/config/hypr/$f" ]] && link_item "$REPO_DIR/config/hypr/$f" "$HOME/.config/hypr/$f"
    done
    for f in "$REPO_DIR"/config/hypr/config/*.lua "$REPO_DIR"/config/hypr/config/theme_settings.json; do
        [[ -f "$f" ]] || continue
        base="$(basename "$f")"
        if [[ "$base" == "autostart_backup.lua" || "$base" == "binds_backup.lua" ]]; then
            if [[ ! -e "$HOME/.config/hypr/config/$base" ]]; then
                run "install -m 644 '$f' '$HOME/.config/hypr/config/$base'"
            fi
            continue
        fi
        link_item "$f" "$HOME/.config/hypr/config/$base"
    done
    render_template "$REPO_DIR/config/hypr/hyprpaper.conf.template" "$HOME/.config/hypr/hyprpaper.conf" 1
    render_template "$REPO_DIR/config/hypr/profile.default" "$HOME/.config/hypr/profile" 1

    local ws_count_file="$HOME/.config/hypr/config/workspaces_count"
    if [[ -n "$WORKSPACES_PER_MON" && "$WORKSPACES_PER_MON" =~ ^[1-9]$ ]]; then
        if [[ "$DRY_RUN" -eq 1 ]]; then
            printf '[DRY-RUN] write %s -> %s\n' "$WORKSPACES_PER_MON" "$ws_count_file"
        else
            printf '%s\n' "$WORKSPACES_PER_MON" > "$ws_count_file"
        fi
    fi
}

detect_and_generate_local_overlays() {
    local env_local="$HOME/.config/hypr/config/environment_local.lua"
    local idle_local="$HOME/.config/hypr/hypridle_local.conf"
    local idle_dropin="$HOME/.config/systemd/user/hypridle.service.d/local.conf"
    local inputs_local="$HOME/.config/hypr/config/inputs_local.lua"
    local mons_local="$HOME/.config/hypr/config/monitors_local.lua"

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
                    echo 'hl.env("GBM_BACKEND", "nvidia-drm")'
                    echo 'hl.env("__GLX_VENDOR_LIBRARY_NAME", "nvidia")'
                    echo 'hl.env("NVD_BACKEND", "direct")'
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
    if [[ ! -e "$inputs_local" ]]; then
        local kb_layout="us,ru"
        if command -v localectl >/dev/null 2>&1; then
            local x11_layout
            x11_layout="$(localectl status 2>/dev/null | awk -F: '/X11 Layout/ {gsub(/^[ \t]+|[ \t]+$/, "", $2); print $2; exit}' || true)"
            if [[ -n "$x11_layout" && "$x11_layout" != "n/a" && "$x11_layout" != "us" ]]; then
                if [[ "$x11_layout" == *","* ]]; then
                    kb_layout="$x11_layout"
                else
                    kb_layout="us,$x11_layout"
                fi
            fi
        fi
        if [[ "$DRY_RUN" -eq 1 ]]; then
            printf '[DRY-RUN] generate inputs overlay %s (kb_layout=%s)\n' "$inputs_local" "$kb_layout"
        else
            mkdir -p "$(dirname "$inputs_local")"
            cat > "${inputs_local}.tmp" <<EOF
-- Auto-generated by inerium-shell (gitignored)
hl.config({
    input = {
        kb_layout  = "${kb_layout}",
        kb_options = "grp:win_space_toggle",
    },
})
EOF
            mv -f "${inputs_local}.tmp" "$inputs_local"
        fi
    fi

    # 4. HiDPI monitor scale detection
    if [[ ! -e "$mons_local" ]] && command -v hyprctl >/dev/null 2>&1 && command -v python3 >/dev/null 2>&1; then
        local mon_json
        mon_json="$(hyprctl monitors -j 2>/dev/null || true)"
        if [[ -n "$mon_json" && "$mon_json" == "["* ]]; then
            if [[ "$DRY_RUN" -eq 1 ]]; then
                printf '[DRY-RUN] generate monitors overlay %s from hyprctl monitors -j\n' "$mons_local"
            else
                python3 - "$mon_json" "$mons_local" <<'PYEOF' || true
import json, math, os, sys
mons = json.loads(sys.argv[1])
lines = ["-- Auto-generated by inerium-shell HiDPI detection (gitignored)\n"]
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
        f'hl.monitor({{\n    output   = "{name}",\n    mode     = "preferred",\n    position = "auto",\n    scale    = {scale},\n}})\n'
    )
os.makedirs(os.path.dirname(sys.argv[2]), exist_ok=True)
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
    if command -v xdg-user-dirs-update >/dev/null 2>&1 && [[ ! -d "$HOME/Pictures" ]]; then
        run "xdg-user-dirs-update"
    fi
    run "mkdir -p '$HOME/Pictures/Wallpapers'"
    for wall in "$REPO_DIR"/assets/wallpapers/*; do
        [[ -f "$wall" ]] || continue
        wname="$(basename "$wall")"
        if [[ ! -e "$HOME/Pictures/Wallpapers/$wname" ]]; then
            run "install -m 644 '$wall' '$HOME/Pictures/Wallpapers/$wname'"
        fi
    done
}

fetch_silueta_model() {
    if [[ -s "$SILUETA_PATH" ]]; then
        log "Found silueta.onnx at $SILUETA_PATH"
    elif [[ "$FETCH_MODEL" -eq 1 ]]; then
        if [[ "$DRY_RUN" -eq 1 ]]; then
            printf '[DRY-RUN] mkdir -p %s && curl -fsSL %s -o %s\n' "$(dirname "$SILUETA_PATH")" "$SILUETA_URL" "$SILUETA_PATH"
        else
            mkdir -p "$(dirname "$SILUETA_PATH")"
            run_step "Downloading silueta.onnx (~43 MB)..." curl -fsSL "$SILUETA_URL" -o "$SILUETA_PATH"
        fi
    else
        log "NOTE: silueta.onnx not found at $SILUETA_PATH."
        log "      Run with --fetch-model for Dolphin AI background removal."
    fi
}

configure_greeter() {
    for sess in "$REPO_DIR"/system/usr/share/wayland-sessions/*.desktop; do
        [[ -f "$sess" ]] || continue
        install_root_if_changed 644 "$sess" "/usr/share/wayland-sessions/$(basename "$sess")"
    done

    if [[ "$WITH_GREETER" -eq 1 ]]; then
        log "Configuring greetd + inerium-greeter..."
        if [[ "$SKIP_PKGS" -eq 0 ]]; then
            if [[ "$DRY_RUN" -eq 1 ]]; then
                printf '[DRY-RUN] install_pkgs greetd cage\n'
            else
                run_step "Installing greetd & cage..." install_pkgs greetd cage
            fi
        fi
        install_root_if_changed 644 "$REPO_DIR/assets/backgrounds/inerium-greeter.jpg" /usr/share/backgrounds/inerium-greeter.jpg
        if [[ -f /etc/greetd/config.toml ]] && ! cmp -s "$REPO_DIR/system/etc/greetd/config.toml" /etc/greetd/config.toml 2>/dev/null; then
            log "Backing up existing /etc/greetd/config.toml -> /etc/greetd/config.toml.bak.${BACKUP_STAMP}"
            run "sudo cp -a /etc/greetd/config.toml '/etc/greetd/config.toml.bak.${BACKUP_STAMP}'"
        fi
        install_root_if_changed 644 "$REPO_DIR/system/etc/greetd/config.toml" /etc/greetd/config.toml
        install_root_if_changed 644 "$REPO_DIR/system/etc/greetd/environments" /etc/greetd/environments
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
        if ! systemctl is-enabled --quiet greetd.service 2>/dev/null; then
            log "Enabling greetd.service..."
            run "sudo systemctl enable --force greetd.service"
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
        link_item "$unit" "$HOME/.config/systemd/user/$uname"
    done
    link_item "$REPO_DIR/config/systemd/user/hypridle.service.d/slice.conf" "$HOME/.config/systemd/user/hypridle.service.d/slice.conf"
    link_item "$REPO_DIR/config/systemd/user/mako.service.d/slice.conf" "$HOME/.config/systemd/user/mako.service.d/slice.conf"
    link_item "$REPO_DIR/config/systemd/user/waybar.service.d/bridge.conf" "$HOME/.config/systemd/user/waybar.service.d/bridge.conf"
    link_item "$REPO_DIR/config/systemd/user/waybar.service.d/slice.conf" "$HOME/.config/systemd/user/waybar.service.d/slice.conf"

    local USER_UNITS="cliphist.service cliphist-images.service hypr-ipc-bridge.service hyprpaper.service hypridle.service inerium-desktop-indexer.path inerium-osd-watcher.service inerium-polkit.service inerium-popups.service inerium-updates.timer mako.service swayosd-server.service waybar.service"
    local MASK_UNITS="hyprpolkitagent.service at-spi2-registryd.service at-spi-dbus-bus.service"

    if systemctl --user show-environment >/dev/null 2>&1; then
        log "Refreshing systemd user daemon & enabling user services..."
        run "systemctl --user daemon-reload"
        run "systemctl --user mask $MASK_UNITS >/dev/null 2>&1 || true"
        run "systemctl --user enable $USER_UNITS"
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

    if command -v kbuildsycoca6 >/dev/null 2>&1; then
        run "kbuildsycoca6 --noincremental >/dev/null 2>&1 || true"
    fi

    # Set Google Chrome as default web browser if installed
    if [[ -f /usr/share/applications/google-chrome.desktop ]]; then
        if command -v xdg-mime >/dev/null 2>&1; then
            run "xdg-mime default google-chrome.desktop x-scheme-handler/http >/dev/null 2>&1 || true"
            run "xdg-mime default google-chrome.desktop x-scheme-handler/https >/dev/null 2>&1 || true"
            run "xdg-mime default google-chrome.desktop text/html >/dev/null 2>&1 || true"
        fi
        if command -v xdg-settings >/dev/null 2>&1; then
            BROWSER="" run "xdg-settings set default-web-browser google-chrome.desktop >/dev/null 2>&1 || true"
        fi
    fi
}
