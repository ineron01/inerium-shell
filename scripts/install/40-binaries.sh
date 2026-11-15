#!/usr/bin/env bash
# ============================================================================
# scripts/install/40-binaries.sh — Prebuilt binary delivery & Cargo build
# ============================================================================

USER_BINS_STR="hypr-ipc-bridge inerium-battery inerium-bluetooth inerium-clipboard inerium-clock inerium-dnd inerium-hotspot inerium-hwmon inerium-layout inerium-network inerium-osd-watcher inerium-popup inerium-updates inerium-shell inerium-setup dolphin-convert"

install_from_binaries() {
    local tmp_dir bin missing_libs
    local -a user_bins
    read -ra user_bins <<< "$USER_BINS_STR"
    tmp_dir="$(mktemp -d)"

    if ! curl -fsSL "${RELEASE_BASE_URL}/${BIN_ARCHIVE}" -o "${tmp_dir}/${BIN_ARCHIVE}"; then
        warn "Failed to download ${RELEASE_BASE_URL}/${BIN_ARCHIVE}"
        rm -rf "$tmp_dir"
        return 1
    fi
    if ! curl -fsSL "${RELEASE_BASE_URL}/${BIN_SHA}" -o "${tmp_dir}/${BIN_SHA}"; then
        warn "Failed to download ${RELEASE_BASE_URL}/${BIN_SHA}"
        rm -rf "$tmp_dir"
        return 1
    fi
    if ! (cd "$tmp_dir" && sha256sum -c "$BIN_SHA" >/dev/null 2>&1); then
        warn "SHA256 verification failed for ${BIN_ARCHIVE}"
        rm -rf "$tmp_dir"
        return 1
    fi

    mkdir -p "${tmp_dir}/dist"
    tar -xzf "${tmp_dir}/${BIN_ARCHIVE}" -C "${tmp_dir}/dist"

    # Smoke-test shared library dependencies via ldd
    for bin in "${user_bins[@]}" inerium-polkit inerium-greeter; do
        if [[ ! -x "${tmp_dir}/dist/${bin}" ]]; then
            warn "Missing binary in release archive: ${bin}"
            rm -rf "$tmp_dir"
            return 1
        fi
        missing_libs="$(ldd "${tmp_dir}/dist/${bin}" 2>/dev/null | grep "not found" || true)"
        if [[ -n "$missing_libs" ]]; then
            warn "Shared library mismatch in ${bin}:"
            printf '%s\n' "$missing_libs" >&2
            rm -rf "$tmp_dir"
            return 1
        fi
    done

    mkdir -p "$HOME/.local/bin"
    for bin in "${user_bins[@]}" inerium-polkit; do
        install -m 755 "${tmp_dir}/dist/${bin}" "$HOME/.local/bin/${bin}"
    done
    install_root_if_changed 755 "${tmp_dir}/dist/inerium-greeter" /usr/local/bin/inerium-greeter

    rm -rf "$tmp_dir"
    return 0
}

build_from_source() {
    local -a user_bins
    read -ra user_bins <<< "$USER_BINS_STR"
    if command -v rustup >/dev/null 2>&1 && ! rustc --version >/dev/null 2>&1; then
        run_step "Initializing default Rust toolchain..." rustup default stable
    fi
    run_step "Compiling universal Rust workspace (this may take 10-15 min)..." \
        cargo build --manifest-path "$REPO_DIR/Cargo.toml" --workspace --release
    run_step "Compiling standalone inerium-polkit..." \
        cargo build --manifest-path "$REPO_DIR/standalone/inerium-polkit/Cargo.toml" --release

    log "Installing compiled binaries atomically..."
    mkdir -p "$HOME/.local/bin"
    for bin in "${user_bins[@]}"; do
        install -m 755 "$REPO_DIR/target/release/$bin" "$HOME/.local/bin/$bin"
    done
    install -m 755 "$REPO_DIR/standalone/inerium-polkit/target/release/inerium-polkit" "$HOME/.local/bin/inerium-polkit"
    install_root_if_changed 755 "$REPO_DIR/target/release/inerium-greeter" /usr/local/bin/inerium-greeter
}

deliver_binaries() {
    if [[ "$SKIP_BUILD" -eq 1 ]]; then
        log "Skipping binary delivery (--skip-build)."
        return 0
    fi

    if [[ "$DRY_RUN" -eq 1 ]]; then
        if [[ "$INSTALL_MODE" == "bin" ]]; then
            printf '[DRY-RUN] Download %s/%s + %s, verify sha256, run ldd smoke-test, install to ~/.local/bin & /usr/local/bin\n' \
                "$RELEASE_BASE_URL" "$BIN_ARCHIVE" "$BIN_SHA"
        else
            printf '[DRY-RUN] cargo build --manifest-path %s/Cargo.toml --workspace --release\n' "$REPO_DIR"
            printf '[DRY-RUN] cargo build --manifest-path %s/standalone/inerium-polkit/Cargo.toml --release\n' "$REPO_DIR"
            printf '[DRY-RUN] install 17 binaries -> ~/.local/bin/ and inerium-greeter -> /usr/local/bin/\n'
        fi
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
                    run_step "Installing Rust build toolchain..." install_pkgs rust base-devel pkgconf clang mold
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
