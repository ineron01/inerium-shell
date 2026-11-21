#!/usr/bin/env bash
# ============================================================================
# scripts/install/60-addons.sh — Optional Modules: MSI Cooling, Camera, noVPN
# ============================================================================

install_msi_ec_addon() {
    log "Setting up MSI EC kernel module (msi-ec DKMS)..."
    manifest_record_addon "msi-ec"
    if [[ "$SKIP_PKGS" -eq 0 ]]; then
        local -a headers=()
        local f
        for f in /usr/lib/modules/*/pkgbase; do
            [[ -f "$f" ]] && headers+=("$(<"$f")-headers")
        done
        install_pkgs dkms msi-ec-dkms-git envycontrol "${headers[@]}"
        MSI_EC_REBUILT=1
    fi

    local sys_dir="$REPO_DIR/system/msi-cooling"
    if [[ -d "$sys_dir" ]]; then
        install_root_if_changed 644 "$sys_dir/udev/rules.d/99-msi-ec.rules" /etc/udev/rules.d/99-msi-ec.rules
        install_root_if_changed 644 "$sys_dir/modules-load.d/msi-ec.conf" /etc/modules-load.d/msi-ec.conf
    fi

    if [[ "$WITH_FAN_REGULATOR" -eq 1 ]]; then
        log "Setting up MSI Fan Regulator daemon (fan curve control)..."
        manifest_record_addon "fan-regulator"
        manifest_record_system_unit "msi-fan-regulator.service"
        if [[ -d "$sys_dir" ]]; then
            install_root_if_changed 644 "$sys_dir/modprobe.d/msi-fan-regulator.conf" /etc/modprobe.d/msi-fan-regulator.conf
            install_root_if_changed 644 "$sys_dir/modules-load.d/msi-fan-regulator.conf" /etc/modules-load.d/msi-fan-regulator.conf
            if [[ -f "$sys_dir/sudoers.d/inerium-msi.template" ]]; then
                install_sudoers_template "$sys_dir/sudoers.d/inerium-msi.template" /etc/sudoers.d/inerium-msi
            fi
            install_root_if_changed 644 "$sys_dir/systemd/system/msi-fan-regulator.service" /etc/systemd/system/msi-fan-regulator.service
        fi

        local fan_bin_src=""
        if [[ -f "$REPO_DIR/target/release/msi-fan-regulator" ]]; then
            fan_bin_src="$REPO_DIR/target/release/msi-fan-regulator"
        elif [[ "$SKIP_BUILD" -eq 0 && "$DRY_RUN" -eq 0 ]]; then
            run_step "Compiling msi-fan-regulator (--features msi-ec)..." \
                cargo build --manifest-path "$REPO_DIR/Cargo.toml" --release -p inerium-tools --features msi-ec --bin msi-fan-regulator
            fan_bin_src="$REPO_DIR/target/release/msi-fan-regulator"
        fi

        if [[ -n "$fan_bin_src" && -f "$fan_bin_src" ]]; then
            install_root_if_changed 755 "$fan_bin_src" /usr/local/bin/msi-fan-regulator
            if [[ "$DRY_RUN" -eq 0 ]]; then
                sudo udevadm control --reload-rules || true
                sudo udevadm trigger || true
                sudo systemctl daemon-reload || true
                sudo systemctl enable msi-fan-regulator.service || true
                if systemctl is-active --quiet msi-fan-regulator.service 2>/dev/null; then
                    sudo systemctl restart msi-fan-regulator.service || true
                fi
            fi
        fi
    fi
}

install_camera_addon() {
    log "Setting up Inerium Camera module (v4l2loopback & inerium-camera)..."
    manifest_record_addon "camera"
    manifest_record_user_unit "inerium-camera-bridge.service"
    manifest_record_binary "$HOME/.local/bin/inerium-camera"
    manifest_record_symlink "$HOME/.local/bin/phone-webcam" "inerium-camera"
    manifest_record_symlink "$HOME/.local/bin/phone-webcam-gui" "inerium-camera"
    if [[ "$SKIP_PKGS" -eq 0 ]]; then
        install_pkgs v4l2loopback-dkms v4l2loopback-utils android-tools scrcpy gstreamer gst-plugins-base-libs gst-plugins-good
    fi

    local cam_sys="$REPO_DIR/system/camera"
    if [[ -d "$cam_sys" ]]; then
        install_root_if_changed 644 "$cam_sys/udev/rules.d/99-phone-webcam.rules" /etc/udev/rules.d/99-phone-webcam.rules
        install_root_if_changed 644 "$cam_sys/modprobe.d/v4l2loopback.conf" /etc/modprobe.d/v4l2loopback.conf
        install_root_if_changed 644 "$cam_sys/modules-load.d/v4l2loopback.conf" /etc/modules-load.d/v4l2loopback.conf
    fi

    if [[ "$SKIP_BUILD" -eq 0 && "$DRY_RUN" -eq 0 ]]; then
        if [[ ! -x "$HOME/.local/bin/inerium-camera" || "$INSTALL_MODE" == "source" ]]; then
            run_step "Compiling inerium-camera..." \
                cargo build --manifest-path "$REPO_DIR/Cargo.toml" --release -p inerium-camera
            install -m 755 "$REPO_DIR/target/release/inerium-camera" "$HOME/.local/bin/inerium-camera"
        fi
        ln -sfn inerium-camera "$HOME/.local/bin/phone-webcam"
        ln -sfn inerium-camera "$HOME/.local/bin/phone-webcam-gui"
    elif [[ -x "$HOME/.local/bin/inerium-camera" ]]; then
        ln -sfn inerium-camera "$HOME/.local/bin/phone-webcam"
        ln -sfn inerium-camera "$HOME/.local/bin/phone-webcam-gui"
    fi

    if [[ -f "$REPO_DIR/local/share/applications/phone-webcam.desktop.template" ]]; then
        render_template "$REPO_DIR/local/share/applications/phone-webcam.desktop.template" "$HOME/.local/share/applications/phone-webcam.desktop"
    fi

    local bridge_src="$REPO_DIR/config/systemd/user/inerium-camera-bridge.service"
    local bridge_dst="$HOME/.config/systemd/user/inerium-camera-bridge.service"
    if [[ -f "$bridge_src" ]]; then
        mkdir -p "$HOME/.config/systemd/user"
        link_item "$bridge_src" "$bridge_dst"
        if [[ -n "${XDG_RUNTIME_DIR:-}" ]] && systemctl --user list-unit-files >/dev/null 2>&1; then
            systemctl --user daemon-reload || true
            systemctl --user enable inerium-camera-bridge.service || true
            if [[ -n "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ]]; then
                systemctl --user restart inerium-camera-bridge.service || true
            fi
        fi
    fi
}

install_novpn_addon() {
    log "Setting up noVPN Split-Tunneling module..."
    manifest_record_addon "novpn"
    manifest_record_system_unit "novpn.service"
    if [[ "$SKIP_PKGS" -eq 0 ]]; then
        install_pkgs iptables iproute2
    fi

    local novpn_sys="$REPO_DIR/system/novpn"
    if [[ -d "$novpn_sys" ]]; then
        for script in "$novpn_sys/bin"/novpn*; do
            [[ -f "$script" ]] && install_root_if_changed 755 "$script" "/usr/local/bin/$(basename "$script")"
        done
        install_root_if_changed 644 "$novpn_sys/systemd/novpn.service" /etc/systemd/system/novpn.service
        if [[ -f "$novpn_sys/sudoers.d/inerium-novpn.template" ]]; then
            install_sudoers_template "$novpn_sys/sudoers.d/inerium-novpn.template" /etc/sudoers.d/inerium-novpn
        fi
        if [[ -f "$novpn_sys/dispatcher.d/98-novpn-sync.sh" ]]; then
            install_root_if_changed 755 "$novpn_sys/dispatcher.d/98-novpn-sync.sh" /etc/NetworkManager/dispatcher.d/98-novpn-sync.sh
        fi
        if [[ "$DRY_RUN" -eq 0 ]]; then
            sudo systemctl daemon-reload || true
            sudo systemctl enable novpn.service || true
        fi
    fi

    local novpn_first_login="$REPO_DIR/first-login.d/30-novpn"
    if [[ -f "$novpn_first_login" ]]; then
        mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/inerium/first-login.d"
        link_item "$novpn_first_login" "${XDG_CONFIG_HOME:-$HOME/.config}/inerium/first-login.d/30-novpn"
    fi
}

install_gdrive_addon() {
    log "Setting up Google Drive Dolphin integration (rclone)..."
    manifest_record_addon "gdrive"
    manifest_record_user_unit "gdrive.service"
    if [[ "$SKIP_PKGS" -eq 0 ]]; then
        install_pkgs rclone
    fi

    mkdir -p "$HOME/.config/rclone"
    link_item "$REPO_DIR/config/rclone/filter.txt" "$HOME/.config/rclone/filter.txt"

    local gdrive_unit="$REPO_DIR/config/systemd/user/gdrive.service"
    if [[ -f "$gdrive_unit" ]]; then
        mkdir -p "$HOME/.config/systemd/user"
        link_item "$gdrive_unit" "$HOME/.config/systemd/user/gdrive.service"
    fi

    local gdrive_first_login="$REPO_DIR/first-login.d/20-gdrive"
    if [[ -f "$gdrive_first_login" ]]; then
        mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/inerium/first-login.d"
        link_item "$gdrive_first_login" "${XDG_CONFIG_HOME:-$HOME/.config}/inerium/first-login.d/20-gdrive"
    fi
}

install_converter_addon() {
    log "Setting up Dolphin Converter addon (media/doc conversion & ONNX background removal)..."
    manifest_record_addon "converter"
    if [[ "$SKIP_PKGS" -eq 0 ]]; then
        install_pkgs ffmpeg imagemagick poppler pandoc-cli typst libreoffice-fresh onnxruntime-cpu
    fi

    log "Linking Dolphin KIO servicemenus into ~/.local/share/kio/servicemenus..."
    mkdir -p "$HOME/.local/share/kio/servicemenus"
    for desktop in "$REPO_DIR"/local/share/kio/servicemenus/*.desktop; do
        [[ -f "$desktop" ]] || continue
        local name="$(basename "$desktop")"
        link_item "$desktop" "$HOME/.local/share/kio/servicemenus/$name"
    done

    fetch_silueta_model
}

deploy_selected_addons() {
    if [[ "$WITH_GDRIVE" -eq 1 ]]; then
        run_step "Configuring Google Drive addon..." install_gdrive_addon
    else
        rm -f "${XDG_CONFIG_HOME:-$HOME/.config}/inerium/first-login.d/20-gdrive"
    fi
    if [[ "$WITH_CONVERTER" -eq 1 ]]; then
        run_step "Configuring Dolphin Converter addon..." install_converter_addon
    elif [[ "$FETCH_MODEL" -eq 1 ]]; then
        fetch_silueta_model
    fi
    if [[ "$WITH_MSI_EC" -eq 1 ]]; then
        run_step "Configuring MSI EC / Cooling addon..." install_msi_ec_addon
    fi
    if [[ "$WITH_CAMERA" -eq 1 ]]; then
        run_step "Configuring Inerium Camera addon..." install_camera_addon
    fi
    if [[ "$WITH_NOVPN" -eq 1 ]]; then
        run_step "Configuring noVPN addon..." install_novpn_addon
    else
        rm -f "${XDG_CONFIG_HOME:-$HOME/.config}/inerium/first-login.d/30-novpn"
    fi
}
