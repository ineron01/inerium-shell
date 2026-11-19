#!/usr/bin/env bash
# ============================================================================
# scripts/install/70-summary.sh — Final Summary, notices & reboot advice
# ============================================================================

print_summary() {
    restore_terminal
    local msg1 msg2 extra_reboot=""
    if [[ "$MSI_EC_REBUILT" -eq 1 ]]; then
        if [[ "$INSTALL_LANG" == "ru" ]]; then
            extra_reboot="[ВНИМАНИЕ] Модуль ядра msi-ec был пересобран через DKMS. Перезагрузите компьютер (sudo reboot), чтобы применить его."
        else
            extra_reboot="[NOTICE] msi-ec kernel module was built via DKMS. Reboot (sudo reboot) to load the new module."
        fi
    fi

    if [[ "$INSTALL_LANG" == "ru" ]]; then
        msg1="Установка Inerium Shell успешно завершена!"
        if [[ -n "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ]]; then
            msg2="Текущая сессия сохранена. Перезагрузитесь (sudo reboot) или завершите сеанс и выберите 'Hyprland (Waybar)' на экране входа."
        else
            msg2="Выполните 'sudo reboot' для запуска inerium-greeter и входа в Hyprland."
        fi
    else
        msg1="Inerium Shell installation complete!"
        if [[ -n "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ]]; then
            msg2="Your current desktop session was kept intact. Reboot (sudo reboot) or log out and select 'Hyprland (Waybar)' at the login screen."
        else
            msg2="Run 'sudo reboot' to start inerium-greeter and enter Hyprland."
        fi
    fi

    if use_tui; then
        local -a lines=("$msg1" "$msg2")
        [[ -n "$extra_reboot" ]] && lines+=("$extra_reboot")
        lines+=("Install log: $LOG_FILE")
        gum style \
            --border rounded \
            --border-foreground 82 \
            --padding "1 2" \
            --margin "1 0" \
            "${lines[@]}"

        echo ""
        if [[ "$INSTALL_LANG" == "ru" ]]; then
            if gum confirm "Рекомендуется перезагрузить систему для чистого применения изменений. Перезагрузить сейчас?" --default=false; then
                log "Перезагрузка системы..."
                sudo reboot
            fi
        else
            if gum confirm "A system reboot is recommended to apply all changes cleanly. Reboot now?" --default=false; then
                log "Rebooting system..."
                sudo reboot
            fi
        fi
    else
        log "$msg1"
        log "$msg2"
        [[ -n "$extra_reboot" ]] && log "$extra_reboot"
        log "Install log: $LOG_FILE"
    fi
    restore_terminal
}
