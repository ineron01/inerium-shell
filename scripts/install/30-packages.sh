#!/usr/bin/env bash
# ============================================================================
# scripts/install/30-packages.sh — System upgrade, paru & package installation
# ============================================================================

ensure_paru() {
    if command -v paru >/dev/null 2>&1; then
        return 0
    fi
    sudo pacman -S --needed --noconfirm base-devel git
    if pacman -Si paru >/dev/null 2>&1; then
        sudo pacman -S --needed --noconfirm paru
    else
        local tmp_dir
        tmp_dir="$(mktemp -d)"
        git clone --depth 1 https://aur.archlinux.org/paru-bin.git "$tmp_dir/paru-bin"
        (cd "$tmp_dir/paru-bin" && makepkg -si --noconfirm) || { rm -rf "$tmp_dir"; return 1; }
        rm -rf "$tmp_dir"
    fi
}

install_pkgs() {
    local -a missing
    # pacman -T lists only deps not satisfied by installed packages or their provides,
    # so e.g. "rust" is skipped when rustup is installed instead of hitting a conflict prompt.
    mapfile -t missing < <(pacman -T "$@" 2>/dev/null || true)
    if [[ ${#missing[@]} -eq 0 ]]; then
        return 0
    fi
    if [[ "$DRY_RUN" -eq 1 ]]; then
        printf '[DRY-RUN] paru -S --needed --noconfirm %s\n' "${missing[*]}"
    else
        paru -S --needed --noconfirm "${missing[@]}"
    fi
}

install_pkglist() {
    local -a wanted filtered
    mapfile -t wanted < <(grep -vE '^\s*(#|$)' "$1" | awk '{print $1}')
    if [[ "$INSTALL_MODE" == "bin" && "$SKIP_BUILD" -eq 0 ]]; then
        local pkg
        for pkg in "${wanted[@]}"; do
            case "$pkg" in
                rust|clang|mold) continue ;;
                *) filtered+=("$pkg") ;;
            esac
        done
        install_pkgs "${filtered[@]}"
    else
        install_pkgs "${wanted[@]}"
    fi
}

install_base_packages() {
    if [[ "$SKIP_PKGS" -eq 1 ]]; then
        log "Skipping package installation (--skip-pkgs)."
        return 0
    fi

    run_step "Upgrading system (pacman -Syu)..." sudo pacman -Syu --noconfirm
    if ! command -v paru >/dev/null 2>&1; then
        if [[ "$DRY_RUN" -eq 1 ]]; then
            printf '[DRY-RUN] ensure_paru\n'
        else
            run_step "Installing AUR helper (paru)..." ensure_paru
        fi
    fi

    run_step "Installing base packages from pkglist-base.txt..." install_pkglist "$REPO_DIR/pkglist-base.txt"

    if [[ ! -d /usr/share/icons/Bibata-Modern-Ice && ! -d "$HOME/.local/share/icons/Bibata-Modern-Ice" ]]; then
        run_step "Installing Bibata cursor theme..." bash -c 'paru -S --needed --noconfirm bibata-cursor-theme-bin || true'
    fi

    for svc in NetworkManager.service bluetooth.service power-profiles-daemon.service; do
        if ! systemctl is-enabled --quiet "$svc" 2>/dev/null && systemctl list-unit-files "$svc" >/dev/null 2>&1; then
            run "sudo systemctl enable '$svc' >/dev/null 2>&1 || true"
        fi
    done
}
