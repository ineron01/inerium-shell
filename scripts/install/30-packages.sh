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
    local -a wanted
    mapfile -t wanted < <(grep -vE '^\s*(#|$)' "$1" | awk '{print $1}')
    install_pkgs "${wanted[@]}"
}

install_base_packages() {
    if [[ "$SKIP_PKGS" -eq 1 ]]; then
        log "Skipping package installation (--skip-pkgs)."
        return 0
    fi

    if [[ "${WITH_SYSUPGRADE:-0}" -eq 1 ]]; then
        local title="Upgrading system (pacman -Syu)..."
        [[ "${INSTALL_LANG:-}" == "ru" ]] && title="Полное обновление системы (pacman -Syu)..."
        run_step "$title" sudo pacman -Syu --noconfirm
    else
        local msg="Fast dependency check (resolving missing packages only)..."
        [[ "${INSTALL_LANG:-}" == "ru" ]] && msg="Быстрая проверка зависимостей (установка только недостающих пакетов)..."
        log "$msg"
    fi

    if ! command -v paru >/dev/null 2>&1; then
        if [[ "$DRY_RUN" -eq 1 ]]; then
            printf '[DRY-RUN] ensure_paru\n'
        else
            run_step "Installing AUR helper (paru)..." ensure_paru
        fi
    fi

    run_step "Installing base packages from pkglist-base.txt..." install_pkglist "$REPO_DIR/pkglist-base.txt"

    if [[ "$INSTALL_MODE" == "source" && "$SKIP_BUILD" -eq 0 ]]; then
        run_step "Installing Rust build toolchain (--from-source)..." install_pkgs rust base-devel pkgconf clang mold
    fi

    if [[ ! -d /usr/share/icons/Bibata-Modern-Ice && ! -d "$HOME/.local/share/icons/Bibata-Modern-Ice" ]]; then
        run_step "Installing Bibata cursor theme..." bash -c 'paru -S --needed --noconfirm bibata-cursor-theme-bin || true'
    fi

    if [[ ! -d /usr/share/icons/Colloid-Dark && ! -d "$HOME/.local/share/icons/Colloid-Dark" ]]; then
        run_step "Installing Colloid icon theme..." bash -c '
            tmp_colloid=$(mktemp -d)
            if git clone --depth 1 https://github.com/vinceliuice/Colloid-icon-theme.git "$tmp_colloid" 2>/dev/null; then
                "$tmp_colloid/install.sh" -d "$HOME/.local/share/icons" -notint
                for t in Colloid Colloid-Dark Colloid-Light; do
                    rm -rf "$HOME/.local/share/icons/$t"/apps*
                    gtk-update-icon-cache -f "$HOME/.local/share/icons/$t" 2>/dev/null || true
                done
                rm -rf "$tmp_colloid"
            else
                paru -S --needed --noconfirm colloid-icon-theme-git || true
            fi
        '
    fi

    for svc in NetworkManager.service bluetooth.service power-profiles-daemon.service; do
        if ! systemctl is-enabled --quiet "$svc" 2>/dev/null && systemctl list-unit-files "$svc" >/dev/null 2>&1; then
            run "sudo systemctl enable '$svc' >/dev/null 2>&1 || true"
        fi
    done
}
