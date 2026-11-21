#!/usr/bin/env bash
# ============================================================================
# scripts/install/40-binaries.sh — Prebuilt binary delivery & Cargo build
# ============================================================================

# Core universal binaries installed for all configurations
USER_BINS_STR="hypr-ipc-bridge inerium-battery inerium-bluetooth inerium-clipboard inerium-clock inerium-dnd inerium-hotspot inerium-hwmon inerium-layout inerium-network inerium-osd-watcher inerium-popup inerium-updates inerium-shell inerium-setup inerium-polkit inerium-wallpaper inerium-theme"

install_from_binaries() {
    local bin missing_libs dl_tmp
    local -a user_bins
    read -ra user_bins <<< "$USER_BINS_STR"

    if [[ -z "${RELEASE_DIST_DIR:-}" || ! -d "$RELEASE_DIST_DIR" ]]; then
        RELEASE_DIST_DIR="$(mktemp -d /tmp/inerium-dist-XXXXXX)"
    fi

    dl_tmp="$(mktemp -d)"
    trap 'rm -rf "$dl_tmp"' RETURN

    log "Fetching release archive ${RELEASE_BASE_URL}/${BIN_ARCHIVE}..."
    if ! download_file "${RELEASE_BASE_URL}/${BIN_ARCHIVE}" "${dl_tmp}/${BIN_ARCHIVE}" 300; then
        warn "Failed to download ${RELEASE_BASE_URL}/${BIN_ARCHIVE}"
        return 1
    fi
    log "Fetching checksum ${RELEASE_BASE_URL}/${BIN_SHA}..."
    if ! download_file "${RELEASE_BASE_URL}/${BIN_SHA}" "${dl_tmp}/${BIN_SHA}" 60; then
        warn "Failed to download ${RELEASE_BASE_URL}/${BIN_SHA}"
        return 1
    fi
    if ! (cd "$dl_tmp" && sha256sum -c "$BIN_SHA" >/dev/null 2>&1); then
        warn "SHA256 verification failed for ${BIN_ARCHIVE}"
        return 1
    fi

    tar -xzf "${dl_tmp}/${BIN_ARCHIVE}" -C "$RELEASE_DIST_DIR"

    # Smoke-test shared library dependencies via ldd for core binaries and active optional modules
    local -a test_bins=("${user_bins[@]}")
    [[ "${WITH_GREETER:-0}" -eq 1 ]] && test_bins+=(inerium-greeter)
    [[ "${WITH_CONVERTER:-0}" -eq 1 ]] && test_bins+=(dolphin-convert)
    [[ "${WITH_CAMERA:-0}" -eq 1 ]] && test_bins+=(inerium-camera)
    [[ "${WITH_FAN_REGULATOR:-0}" -eq 1 ]] && test_bins+=(msi-fan-regulator)

    for bin in "${test_bins[@]}"; do
        if [[ ! -x "${RELEASE_DIST_DIR}/${bin}" ]]; then
            warn "Missing binary in release archive: ${bin}"
            return 1
        fi
        missing_libs="$(ldd "${RELEASE_DIST_DIR}/${bin}" 2>/dev/null | grep "not found" || true)"
        if [[ -n "$missing_libs" ]]; then
            warn "Shared library mismatch in ${bin}:"
            printf '%s\n' "$missing_libs" >&2
            return 1
        fi
    done

    mkdir -p "$HOME/.local/bin"
    for bin in "${user_bins[@]}"; do
        install -m 755 "${RELEASE_DIST_DIR}/${bin}" "$HOME/.local/bin/${bin}"
        manifest_record_binary "$HOME/.local/bin/${bin}"
    done
    ln -sfn inerium-wallpaper "$HOME/.local/bin/set-wallpaper"
    manifest_record_symlink "$HOME/.local/bin/set-wallpaper" "inerium-wallpaper"

    return 0
}

build_from_source() {
    local -a user_bins
    read -ra user_bins <<< "$USER_BINS_STR"
    if command -v rustup >/dev/null 2>&1 && ! rustc --version >/dev/null 2>&1; then
        run_live_step "Initializing default Rust toolchain..." rustup default stable
    fi

    run_live_step "Compiling core Rust workspace (this may take 10-15 min)..." \
        cargo build --manifest-path "$REPO_DIR/Cargo.toml" --release \
            -p inerium-tools -p inerium-shell -p inerium-setup -p inerium-polkit -p inerium-theme

    log "Installing core binaries atomically..."
    mkdir -p "$HOME/.local/bin"
    for bin in "${user_bins[@]}"; do
        install -m 755 "$REPO_DIR/target/release/$bin" "$HOME/.local/bin/$bin"
        manifest_record_binary "$HOME/.local/bin/$bin"
    done
    ln -sfn inerium-wallpaper "$HOME/.local/bin/set-wallpaper"
    manifest_record_symlink "$HOME/.local/bin/set-wallpaper" "inerium-wallpaper"
}

check_installed_binaries_abi() {
    local -a user_bins
    read -ra user_bins <<< "$USER_BINS_STR"
    local -a all_check_bins=("${user_bins[@]}" dolphin-convert inerium-camera)
    local has_broken=0
    for bin in "${all_check_bins[@]}"; do
        local bpath="$HOME/.local/bin/$bin"
        if [[ -x "$bpath" ]]; then
            local missing
            missing="$(ldd "$bpath" 2>/dev/null | grep "not found" || true)"
            if [[ -n "$missing" ]]; then
                warn "ABI mismatch in $bin:"
                printf '%s\n' "$missing" >&2
                has_broken=1
            fi
        fi
    done
    for sbin in /usr/local/bin/inerium-greeter /usr/local/bin/msi-fan-regulator; do
        if [[ -x "$sbin" ]]; then
            local missing
            missing="$(ldd "$sbin" 2>/dev/null | grep "not found" || true)"
            if [[ -n "$missing" ]]; then
                warn "ABI mismatch in $sbin:"
                printf '%s\n' "$missing" >&2
                has_broken=1
            fi
        fi
    done

    if [[ "$has_broken" -eq 1 ]]; then
        warn "One or more binaries have broken dynamic library dependencies."
        return 1
    fi
    log "All installed binaries have healthy library dependencies."
    return 0
}

deliver_binaries() {
    if [[ "$SKIP_BUILD" -eq 1 ]]; then
        log "Skipping binary delivery (--skip-build)."
        local -a user_bins
        read -ra user_bins <<< "$USER_BINS_STR"
        for bin in "${user_bins[@]}"; do
            [[ -x "$HOME/.local/bin/$bin" ]] && manifest_record_binary "$HOME/.local/bin/$bin"
        done
        [[ -e "$HOME/.local/bin/set-wallpaper" ]] && manifest_record_symlink "$HOME/.local/bin/set-wallpaper" "inerium-wallpaper"
        [[ -e "/usr/local/bin/inerium-greeter" ]] && manifest_record_system_file "/usr/local/bin/inerium-greeter"
        return 0
    fi

    if [[ "$DRY_RUN" -eq 1 ]]; then
        if [[ "$INSTALL_MODE" == "bin" ]]; then
            printf '[DRY-RUN] Download %s/%s + %s, verify sha256, run ldd smoke-test, install core binaries to ~/.local/bin\n' \
                "$RELEASE_BASE_URL" "$BIN_ARCHIVE" "$BIN_SHA"
        else
            printf '[DRY-RUN] cargo build --manifest-path %s/Cargo.toml --release -p inerium-tools -p inerium-shell -p inerium-setup -p inerium-polkit -p inerium-theme\n' "$REPO_DIR"
            printf '[DRY-RUN] install core binaries -> ~/.local/bin/\n'
        fi
        printf '[DRY-RUN] ln -sfn inerium-wallpaper -> ~/.local/bin/set-wallpaper\n'
        return 0
    fi

    if [[ "$INSTALL_MODE" == "bin" ]]; then
        if ! run_step "Downloading & verifying prebuilt binaries..." install_from_binaries; then
            warn "Prebuilt binary installation failed or shared libraries did not match."
            local do_fallback=1
            if use_tui; then
                if ! gum confirm "Fall back to local Cargo compilation (--from-source)?" --default=true; then
                    do_fallback=0
                fi
            fi
            if [[ "$do_fallback" -eq 1 ]]; then
                INSTALL_MODE="source"
                if [[ "$SKIP_PKGS" -eq 0 ]]; then
                    run_live_step "Installing Rust build toolchain..." install_pkgs rust base-devel pkgconf clang mold
                fi
                build_from_source
            else
                exit 1
            fi
        fi
    else
        build_from_source
    fi
}
