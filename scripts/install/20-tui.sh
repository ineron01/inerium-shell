#!/usr/bin/env bash
# ============================================================================
# scripts/install/20-tui.sh — Interactive Bilingual TUI (gum) & prompts
# ============================================================================

ensure_gum_installed() {
    if [[ "$DRY_RUN" -eq 0 && "$NON_INTERACTIVE" -eq 0 && -t 0 && -t 1 ]] && ! command -v gum >/dev/null 2>&1; then
        if [[ "$SKIP_PKGS" -eq 0 ]]; then
            sudo pacman -S --needed --noconfirm gum >> "$LOG_FILE" 2>&1 || true
        fi
    fi
}

run_tui_prompts() {
    local gpu_info chassis_info dm_info sess_info is_msi
    gpu_info="$(detect_gpu_label)"
    chassis_info="$(detect_chassis_label)"
    dm_info="$(detect_current_dm)"
    sess_info="$(detect_current_session)"

    is_msi=0
    if detect_is_msi_hardware; then
        is_msi=1
    fi

    # Select language if not explicitly provided
    if [[ -z "$INSTALL_LANG" ]]; then
        local def_lang="Русский"
        if [[ ! "${LANG:-}" =~ ^ru ]] && [[ ! "${LC_ALL:-}" =~ ^ru ]]; then
            def_lang="English"
        fi
        local lang_choice
        lang_choice="$(gum choose --header "Select installer language / Выберите язык установки:" \
            --selected "$def_lang" "Русский" "English")"
        if [[ "$lang_choice" == "Русский" ]]; then
            INSTALL_LANG="ru"
        else
            INSTALL_LANG="en"
        fi
    fi

    if [[ "$INSTALL_LANG" == "ru" ]]; then
        gum style \
            --border rounded \
            --border-foreground 212 \
            --padding "1 3" \
            --margin "1 0" \
            --bold \
            "INERIUM SHELL" \
            "Окружение рабочего стола Hyprland Lua + Rust"

        gum style \
            --border normal \
            --border-foreground 99 \
            --padding "0 2" \
            --margin "0 0 1 0" \
            "Обнаруженная система:" \
            "  GPU:                 $gpu_info" \
            "  Шасси:               $chassis_info" \
            "  Дисплейный менеджер: $dm_info" \
            "  Сессия:              $sess_info"

        if [[ "$SKIP_BUILD" -eq 0 ]]; then
            local mode_fast="Быстрая (готовые бинарники, ~1-2 мин)"
            local mode_src="Из исходников (локальная сборка Cargo, ~15 мин)"
            local default_mode="$mode_fast"
            [[ "$INSTALL_MODE" == "source" ]] && default_mode="$mode_src"
            local chosen_mode
            chosen_mode="$(gum choose --header "Режим установки:" --selected "$default_mode" "$mode_fast" "$mode_src")"
            if [[ "$chosen_mode" == "$mode_src" ]]; then
                INSTALL_MODE="source"
            else
                INSTALL_MODE="bin"
            fi
        fi

        local dm_greeter="Включить inerium-greeter (greetd)"
        local dm_keep="Оставить текущий дисплейный менеджер"
        local default_dm="$dm_greeter"
        [[ "$WITH_GREETER" -eq 0 ]] && default_dm="$dm_keep"
        local chosen_dm
        chosen_dm="$(gum choose --header "Экран входа (Display Manager):" --selected "$default_dm" "$dm_greeter" "$dm_keep")"
        if [[ "$chosen_dm" == "$dm_greeter" ]]; then
            WITH_GREETER=1
        else
            WITH_GREETER=0
        fi

        local opt_gdrive="Google Drive — интеграция облака в Dolphin (rclone)"
        local opt_converter="Dolphin Converter — конвертация файлов (LibreOffice, Pandoc, ONNX)"
        local opt_cam="Inerium Camera — веб-камера из Android (USB/Wi-Fi)"
        local opt_msi="Охлаждение MSI — msi-ec DKMS + авторегулятор кулеров (MSI)"
        local opt_novpn="noVPN — запуск отдельных программ в обход VPN"
        local header_addons="Дополнительные модули (Пробел — выбор/снятие, Enter — подтвердить):"
        local chosen_addons
        chosen_addons="$(gum choose --no-limit --header "$header_addons" --selected "$opt_gdrive,$opt_novpn" "$opt_gdrive" "$opt_converter" "$opt_novpn" "$opt_cam" "$opt_msi")"
        [[ "$chosen_addons" =~ "$opt_gdrive" ]] && WITH_GDRIVE=1
        [[ "$chosen_addons" =~ "$opt_converter" ]] && WITH_CONVERTER=1
        [[ "$chosen_addons" =~ "$opt_cam" ]] && WITH_CAMERA=1
        [[ "$chosen_addons" =~ "$opt_msi" ]] && WITH_MSI_EC=1
        [[ "$chosen_addons" =~ "$opt_novpn" ]] && WITH_NOVPN=1

        if [[ "$WITH_CONVERTER" -eq 1 ]]; then
            if gum confirm "Скачать нейросеть silueta.onnx (~43 МБ для вырезания фона в Dolphin)?" \
                --default="$([[ "$FETCH_MODEL" -eq 1 ]] && echo true || echo false)"; then
                FETCH_MODEL=1
            else
                FETCH_MODEL=0
            fi
        fi

    else
        gum style \
            --border rounded \
            --border-foreground 212 \
            --padding "1 3" \
            --margin "1 0" \
            --bold \
            "INERIUM SHELL" \
            "Hyprland Lua + Rust Desktop Environment"

        gum style \
            --border normal \
            --border-foreground 99 \
            --padding "0 2" \
            --margin "0 0 1 0" \
            "Detected System:" \
            "  GPU:             $gpu_info" \
            "  Chassis:         $chassis_info" \
            "  Display Manager: $dm_info" \
            "  Session:         $sess_info"

        if [[ "$SKIP_BUILD" -eq 0 ]]; then
            local mode_fast="Fast (prebuilt binaries, ~1-2 min)"
            local mode_src="From source (local Cargo build, ~15 min)"
            local default_mode="$mode_fast"
            [[ "$INSTALL_MODE" == "source" ]] && default_mode="$mode_src"
            local chosen_mode
            chosen_mode="$(gum choose --header "Installation mode:" --selected "$default_mode" "$mode_fast" "$mode_src")"
            if [[ "$chosen_mode" == "$mode_src" ]]; then
                INSTALL_MODE="source"
            else
                INSTALL_MODE="bin"
            fi
        fi

        local dm_greeter="Enable inerium-greeter (greetd)"
        local dm_keep="Keep current DM untouched"
        local default_dm="$dm_greeter"
        [[ "$WITH_GREETER" -eq 0 ]] && default_dm="$dm_keep"
        local chosen_dm
        chosen_dm="$(gum choose --header "Login screen (Display Manager):" --selected "$default_dm" "$dm_greeter" "$dm_keep")"
        if [[ "$chosen_dm" == "$dm_greeter" ]]; then
            WITH_GREETER=1
        else
            WITH_GREETER=0
        fi

        local opt_gdrive="Google Drive — cloud integration in Dolphin (rclone)"
        local opt_converter="Dolphin Converter — media/doc conversion & ONNX background removal"
        local opt_cam="Inerium Camera — Android phone webcam over USB/Wi-Fi"
        local opt_msi="MSI Cooling — msi-ec DKMS + fan regulator (MSI)"
        local opt_novpn="noVPN — run specific apps bypassing VPN tunnel"
        local header_addons="Optional Add-on Modules (Space to toggle, Enter to confirm):"
        local chosen_addons
        chosen_addons="$(gum choose --no-limit --header "$header_addons" --selected "$opt_gdrive,$opt_novpn" "$opt_gdrive" "$opt_converter" "$opt_novpn" "$opt_cam" "$opt_msi")"
        [[ "$chosen_addons" =~ "$opt_gdrive" ]] && WITH_GDRIVE=1
        [[ "$chosen_addons" =~ "$opt_converter" ]] && WITH_CONVERTER=1
        [[ "$chosen_addons" =~ "$opt_cam" ]] && WITH_CAMERA=1
        [[ "$chosen_addons" =~ "$opt_msi" ]] && WITH_MSI_EC=1
        [[ "$chosen_addons" =~ "$opt_novpn" ]] && WITH_NOVPN=1

        if [[ "$WITH_CONVERTER" -eq 1 ]]; then
            if gum confirm "Download silueta.onnx (~43 MB for Dolphin AI background removal)?" \
                --default="$([[ "$FETCH_MODEL" -eq 1 ]] && echo true || echo false)"; then
                FETCH_MODEL=1
            else
                FETCH_MODEL=0
            fi
        fi
    fi
}
