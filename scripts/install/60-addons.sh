#!/usr/bin/env bash
# ============================================================================
# scripts/install/60-addons.sh — Optional Modules: MSI Cooling, Camera, noVPN
# ============================================================================

install_msi_ec_addon() {
    log "Setting up MSI Cooling module (msi-ec DKMS & msi-fan-regulator)..."
    if [[ "$SKIP_PKGS" -eq 0 ]]; then
        local -a headers=()
        local f
        for f in /usr/lib/modules/*/pkgbase; do
            [[ -f "$f" ]] && headers+=("$(<"$f")-headers")
        done
        install_pkgs dkms envycontrol "${headers[@]}"
    fi

    local src_dir="$REPO_DIR/hardware/msi-ec"
    local version="0.13"
    [[ -f "$src_dir/Makefile.vars" ]] && version="$(awk -F':=' '/^VERSION/{gsub(/[[:space:]]/, "", $2); print $2}' "$src_dir/Makefile.vars")"
    local dst_dir="/usr/src/msi-ec-${version}"

    # Remove any conflicting packaged msi-ec
    local pkg
    for pkg in msi-ec-dkms-git msi-ec-dkms msi-ec-git; do
        if pacman -Qq "$pkg" >/dev/null 2>&1; then
            run "sudo pacman -Rn --noconfirm $pkg || true"
        fi
    done

    if [[ "$DRY_RUN" -eq 0 && -d "$src_dir" ]]; then
        local stage
        stage="$(mktemp -d)"
        for f in dkms.conf Makefile Makefile.vars msi-ec.c ec_memory_configuration.h; do
            [[ -f "$src_dir/$f" ]] && install -m 644 "$src_dir/$f" "$stage/$f"
        done
        sed -i "s/@VERSION@/$version/g" "$stage/dkms.conf"
        if ! (diff -rq "$stage" "$dst_dir" >/dev/null 2>&1 && dkms status -m msi-ec -v "$version" -k "$(uname -r)" 2>/dev/null | grep -q 'installed'); then
            log "Building/installing patched msi-ec via DKMS ($version)..."
            sudo rm -rf "$dst_dir"
            sudo install -d -m 755 "$dst_dir"
            sudo install -m 644 "$stage"/* "$dst_dir/"
            sudo dkms install --force -m msi-ec -v "$version" || true
            MSI_EC_REBUILT=1
        fi
        rm -rf "$stage"
    fi

    local sys_dir="$REPO_DIR/system/msi-cooling"
    if [[ -d "$sys_dir" ]]; then
        install_root_if_changed 644 "$sys_dir/udev/rules.d/99-msi-ec.rules" /etc/udev/rules.d/99-msi-ec.rules
        install_root_if_changed 644 "$sys_dir/modprobe.d/msi-fan-regulator.conf" /etc/modprobe.d/msi-fan-regulator.conf
        install_root_if_changed 644 "$sys_dir/modules-load.d/msi-fan-regulator.conf" /etc/modules-load.d/msi-fan-regulator.conf
        install_root_if_changed 644 "$sys_dir/modules-load.d/msi-ec.conf" /etc/modules-load.d/msi-ec.conf
        if [[ -f "$sys_dir/sudoers.d/inerium-msi.template" ]]; then
            local sudo_tmp
            sudo_tmp="$(mktemp)"
            sed "s/__USER__/$USER/g" "$sys_dir/sudoers.d/inerium-msi.template" > "$sudo_tmp"
            if visudo -cf "$sudo_tmp" >/dev/null 2>&1; then
                install_root_if_changed 440 "$sudo_tmp" /etc/sudoers.d/inerium-msi
            fi
            rm -f "$sudo_tmp"
        fi
        install_root_if_changed 644 "$sys_dir/systemd/system/msi-fan-regulator.service" /etc/systemd/system/msi-fan-regulator.service
    fi

    if [[ "$SKIP_BUILD" -eq 0 && "$DRY_RUN" -eq 0 ]]; then
        run_step "Compiling msi-fan-regulator (--features msi-ec)..." \
            cargo build --manifest-path "$REPO_DIR/Cargo.toml" --release -p inerium-tools --features msi-ec --bin msi-fan-regulator
        install_root_if_changed 755 "$REPO_DIR/target/release/msi-fan-regulator" /usr/local/bin/msi-fan-regulator
        sudo udevadm control --reload-rules || true
        sudo udevadm trigger || true
        sudo systemctl daemon-reload || true
        sudo systemctl enable msi-fan-regulator.service || true
        if systemctl is-active --quiet msi-fan-regulator.service 2>/dev/null; then
            sudo systemctl restart msi-fan-regulator.service || true
        fi
    fi
}

install_camera_addon() {
    log "Setting up Inerium Camera module (v4l2loopback & inerium-camera)..."
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
        run_step "Compiling inerium-camera..." \
            cargo build --manifest-path "$REPO_DIR/Cargo.toml" --release -p inerium-camera
        install -m 755 "$REPO_DIR/target/release/inerium-camera" "$HOME/.local/bin/inerium-camera"
        ln -sfn inerium-camera "$HOME/.local/bin/phone-webcam"
        ln -sfn inerium-camera "$HOME/.local/bin/phone-webcam-gui"
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
    if [[ "$SKIP_PKGS" -eq 0 ]]; then
        install_pkgs iptables iproute2
    fi

    local novpn_sys="$REPO_DIR/system/novpn"
    if [[ -d "$novpn_sys" ]]; then
        for script in "$novpn_sys/bin"/novpn*; do
            [[ -f "$script" ]] && install_root_if_changed 755 "$script" "/usr/local/bin/$(basename "$script")"
        done
        install_root_if_changed 644 "$novpn_sys/systemd/novpn.service" /etc/systemd/system/novpn.service
        sudo systemctl daemon-reload || true
        sudo systemctl enable novpn.service || true
    fi
}

deploy_selected_addons() {
    if [[ "$WITH_MSI_EC" -eq 1 ]]; then
        run_step "Configuring MSI Cooling addon..." install_msi_ec_addon
    fi
    if [[ "$WITH_CAMERA" -eq 1 ]]; then
        run_step "Configuring Inerium Camera addon..." install_camera_addon
    fi
    if [[ "$WITH_NOVPN" -eq 1 ]]; then
        run_step "Configuring noVPN addon..." install_novpn_addon
    fi
}
