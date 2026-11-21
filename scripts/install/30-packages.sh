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

    # Fast path: if all missing packages exist in official sync repos, install directly via sudo pacman.
    # This completely avoids paru interactive review prompts, AUR queries, and nested sudo deadlocks.
    if pacman -Si "${missing[@]}" >/dev/null 2>&1; then
        if [[ "$DRY_RUN" -eq 1 ]]; then
            printf '[DRY-RUN] sudo pacman -S --needed --noconfirm %s\n' "${missing[*]}"
        else
            sudo pacman -S --needed --noconfirm "${missing[@]}"
        fi
        return 0
    fi

    # Mixed path: separate official repository packages from AUR packages
    local -a repo_pkgs=()
    local -a aur_pkgs=()
    local -A is_repo=()

    while IFS= read -r p; do
        [[ -n "$p" ]] && is_repo["$p"]=1
    done < <(pacman -Slq | sort -u)

    for pkg in "${missing[@]}"; do
        if [[ -n "${is_repo["$pkg"]:-}" ]]; then
            repo_pkgs+=("$pkg")
        else
            aur_pkgs+=("$pkg")
        fi
    done

    if [[ ${#repo_pkgs[@]} -gt 0 ]]; then
        if [[ "$DRY_RUN" -eq 1 ]]; then
            printf '[DRY-RUN] sudo pacman -S --needed --noconfirm %s\n' "${repo_pkgs[*]}"
        else
            sudo pacman -S --needed --noconfirm "${repo_pkgs[@]}"
        fi
    fi

    if [[ ${#aur_pkgs[@]} -gt 0 ]]; then
        if [[ "$DRY_RUN" -eq 1 ]]; then
            printf '[DRY-RUN] paru -S --needed --noconfirm --skipreview --sudoloop %s\n' "${aur_pkgs[*]}"
        else
            paru -S --needed --noconfirm --skipreview --sudoloop "${aur_pkgs[@]}"
        fi
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

    # Clean up stale pacman db.lck if left behind by an interrupted run
    if [[ -f /var/lib/pacman/db.lck ]]; then
        if ! pgrep -x pacman >/dev/null 2>&1 && ! pgrep -x paru >/dev/null 2>&1; then
            log "Removing stale /var/lib/pacman/db.lck (no active pacman/paru process found)..."
            if [[ "$DRY_RUN" -eq 1 ]]; then
                printf '[DRY-RUN] sudo rm -f /var/lib/pacman/db.lck\n'
            else
                sudo rm -f /var/lib/pacman/db.lck || true
            fi
        fi
    fi

    if [[ "${WITH_SYSUPGRADE:-0}" -eq 1 ]]; then
        local title="Upgrading system (pacman -Syu)..."
        [[ "${INSTALL_LANG:-}" == "ru" ]] && title="Полное обновление системы (pacman -Syu)..."
        run_live_step "$title" sudo pacman -Syu --noconfirm
    else
        local msg="Fast dependency check (resolving missing packages only)..."
        [[ "${INSTALL_LANG:-}" == "ru" ]] && msg="Быстрая проверка зависимостей (установка только недостающих пакетов)..."
        log "$msg"
    fi

    if ! command -v paru >/dev/null 2>&1; then
        if [[ "$DRY_RUN" -eq 1 ]]; then
            printf '[DRY-RUN] ensure_paru\n'
        else
            run_live_step "Installing AUR helper (paru)..." ensure_paru
        fi
    fi

    install_base_pkglist() {
        if ! install_pkglist "$REPO_DIR/pkglist-base.txt"; then
            warn "Package installation failed. Your package databases may be outdated."
            warn "Hint: run the installer with --sysupgrade (or execute 'sudo pacman -Syu')."
            return 1
        fi
    }
    run_live_step "Installing base packages from pkglist-base.txt..." install_base_pkglist

    if [[ "$INSTALL_MODE" == "source" && "$SKIP_BUILD" -eq 0 ]]; then
        run_live_step "Installing Rust build toolchain (--from-source)..." install_pkgs rust base-devel pkgconf clang mold
    fi

    install_bibata_cursor() {
        if [[ "$DRY_RUN" -eq 1 ]]; then
            printf '[DRY-RUN] paru -S --needed --noconfirm --skipreview --sudoloop bibata-cursor-theme-bin\n'
            return 0
        fi
        if ! paru -S --needed --noconfirm --skipreview --sudoloop bibata-cursor-theme-bin; then
            warn "Failed to install bibata-cursor-theme-bin via paru; continuing without custom cursor..."
        fi
    }

    if [[ ! -d /usr/share/icons/Bibata-Modern-Ice && ! -d "$HOME/.local/share/icons/Bibata-Modern-Ice" ]]; then
        run_live_step "Installing Bibata cursor theme..." install_bibata_cursor
    fi

    install_colloid_icons() {
        if [[ "$DRY_RUN" -eq 1 ]]; then
            printf '[DRY-RUN] install Colloid icon theme (fast)\n'
            return 0
        fi
        local tmp_colloid
        tmp_colloid="$(mktemp -d)"
        if git clone --depth 1 https://github.com/vinceliuice/Colloid-icon-theme.git "$tmp_colloid" 2>/dev/null; then
            # Strip heavy scalable application icons before installation so only needed system/folder icons are built
            rm -rf "$tmp_colloid/src/apps/scalable"
            mkdir -p "$tmp_colloid/src/apps/scalable" "$HOME/.local/share/icons"
            "$tmp_colloid/install.sh" -d "$HOME/.local/share/icons" -notint
            for t in Colloid Colloid-Dark Colloid-Light; do
                rm -rf "$HOME/.local/share/icons/$t"/apps*
            done
            rm -rf "$tmp_colloid"
        else
            warn "Git clone of Colloid-icon-theme failed; attempting AUR fallback..."
            paru -S --needed --noconfirm --skipreview --sudoloop colloid-icon-theme-git || warn "Failed to install colloid-icon-theme-git."
        fi
    }

    if [[ ! -d /usr/share/icons/Colloid-Dark && ! -d "$HOME/.local/share/icons/Colloid-Dark" ]]; then
        run_live_step "Installing Colloid icon theme..." install_colloid_icons
    fi

    for svc in NetworkManager.service bluetooth.service power-profiles-daemon.service; do
        if ! systemctl is-enabled --quiet "$svc" 2>/dev/null && systemctl list-unit-files "$svc" >/dev/null 2>&1; then
            run "sudo systemctl enable '$svc' >/dev/null 2>&1 || true"
        fi
    done
}
