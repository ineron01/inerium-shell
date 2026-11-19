#!/usr/bin/env bash
# ============================================================================
# scripts/install/20-tui.sh — Interactive Bilingual TUI (gum) & prompts
# ============================================================================

ensure_console_cyrillic_font() {
    # If running in a Linux virtual terminal (TTY), ensure a font with Cyrillic glyphs is loaded
    if [[ "${TERM:-}" == "linux" ]] && command -v setfont >/dev/null 2>&1; then
        for font in ter-v16b cyr-sun16 latarcyrheb-sun16; do
            if setfont "$font" >/dev/null 2>&1 || sudo setfont "$font" >/dev/null 2>&1; then
                break
            fi
        done
    fi
}

gum_choose_multiselect() {
    local header="$1"
    local selected="$2"
    shift 2
    if [[ -n "$selected" ]]; then
        gum choose --no-limit \
            --selected-prefix="[✓] " \
            --unselected-prefix="[ ] " \
            --selected.foreground="212" \
            --item.foreground="245" \
            --header="$header" \
            --selected="$selected" \
            "$@"
    else
        gum choose --no-limit \
            --selected-prefix="[✓] " \
            --unselected-prefix="[ ] " \
            --selected.foreground="212" \
            --item.foreground="245" \
            --header="$header" \
            "$@"
    fi
}

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

    # Update Mode prompt workflow
    if [[ "${IS_UPDATE:-0}" -eq 1 ]]; then
        if [[ -z "$INSTALL_LANG" ]]; then
            if [[ -f "$HOME/.config/locale.conf" ]] && grep -q "ru_RU" "$HOME/.config/locale.conf"; then
                INSTALL_LANG="ru"
            elif [[ "${LANG:-}" =~ ^ru || "${LC_ALL:-}" =~ ^ru ]]; then
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
                "INERIUM SHELL — ОБНОВЛЕНИЕ" \
                "Быстрое обновление компонентов и конфигураций"

            local -a config_info=(
                "Текущая конфигурация:"
                "  GPU:                 $gpu_info"
                "  Дисплейный менеджер: $dm_info"
                "  noVPN:               $([[ "$WITH_NOVPN" -eq 1 ]] && echo 'Включен' || echo 'Выключен')"
                "  Google Drive:        $([[ "$WITH_GDRIVE" -eq 1 ]] && echo 'Включен' || echo 'Выключен')"
                "  Dolphin Converter:   $([[ "$WITH_CONVERTER" -eq 1 ]] && echo 'Включен' || echo 'Выключен')"
                "  Inerium Camera:      $([[ "$WITH_CAMERA" -eq 1 ]] && echo 'Включен' || echo 'Выключен')"
            )
            if [[ "$is_msi" -eq 1 ]]; then
                config_info+=(
                    "  MSI EC (DKMS):       $([[ "$WITH_MSI_EC" -eq 1 ]] && echo 'Включен' || echo 'Выключен')"
                    "  MSI Fan Regulator:   $([[ "$WITH_FAN_REGULATOR" -eq 1 ]] && echo 'Включен' || echo 'Выключен')"
                )
            fi

            gum style \
                --border normal \
                --border-foreground 99 \
                --padding "0 2" \
                --margin "0 0 1 0" \
                "${config_info[@]}"

            local modify_choice=0
            if gum confirm "Хотите добавить новые компоненты или изменить состав модулей?" --default=false; then
                modify_choice=1
            fi

            if [[ "$modify_choice" -eq 1 ]]; then
                local opt_gdrive="Google Drive — интеграция облака в Dolphin (rclone)"
                local opt_converter="Dolphin Converter — конвертация файлов (LibreOffice, Pandoc, ONNX)"
                local opt_cam="Inerium Camera — веб-камера из Android (USB/Wi-Fi)"
                local opt_novpn="noVPN — запуск отдельных программ в обход VPN"
                local opt_msi="Драйвер MSI EC — msi-ec DKMS (лимит батареи 80%, чтение датчиков EC)"
                local opt_fan="Регулятор кулеров MSI — msi-fan-regulator (Внимание: штатный BIOS надежнее)"

                local -a addon_items=("$opt_gdrive" "$opt_converter" "$opt_novpn" "$opt_cam")
                if [[ "$is_msi" -eq 1 ]]; then
                    addon_items+=("$opt_msi" "$opt_fan")
                fi

                local sel=""
                [[ "$WITH_GDRIVE" -eq 1 ]] && sel="${sel:+$sel,}$opt_gdrive"
                [[ "$WITH_CONVERTER" -eq 1 ]] && sel="${sel:+$sel,}$opt_converter"
                [[ "$WITH_NOVPN" -eq 1 ]] && sel="${sel:+$sel,}$opt_novpn"
                [[ "$WITH_CAMERA" -eq 1 ]] && sel="${sel:+$sel,}$opt_cam"
                [[ "$WITH_MSI_EC" -eq 1 ]] && sel="${sel:+$sel,}$opt_msi"
                [[ "$WITH_FAN_REGULATOR" -eq 1 ]] && sel="${sel:+$sel,}$opt_fan"

                local header_addons="Выберите состав модулей (клавиша 'x' — отметить/снять, Enter — подтвердить):"
                local chosen_addons
                chosen_addons="$(gum_choose_multiselect "$header_addons" "$sel" "${addon_items[@]}")"

                WITH_GDRIVE=0; WITH_CONVERTER=0; WITH_CAMERA=0; WITH_MSI_EC=0; WITH_FAN_REGULATOR=0; WITH_NOVPN=0
                [[ "$chosen_addons" =~ "$opt_gdrive" ]] && WITH_GDRIVE=1
                [[ "$chosen_addons" =~ "$opt_converter" ]] && WITH_CONVERTER=1
                [[ "$chosen_addons" =~ "$opt_cam" ]] && WITH_CAMERA=1
                [[ "$chosen_addons" =~ "$opt_msi" ]] && WITH_MSI_EC=1
                [[ "$chosen_addons" =~ "$opt_fan" ]] && WITH_FAN_REGULATOR=1 && WITH_MSI_EC=1
                [[ "$chosen_addons" =~ "$opt_novpn" ]] && WITH_NOVPN=1

                if [[ "$WITH_CONVERTER" -eq 1 && ! -f "$HOME/.local/share/dolphin-converter/models/silueta.onnx" ]]; then
                    if gum confirm "Скачать нейросеть silueta.onnx (~43 МБ для вырезания фона в Dolphin)?" --default=true; then
                        FETCH_MODEL=1
                    fi
                fi
            else
                log "Сохранение текущего набора модулей. Запуск быстрого обновления..."
            fi

            local cfg_act_update="Обновить конфигурации (изменённые файлы будут забэкаплены в .bak)"
            local cfg_act_keep="Оставить мои текущие конфигурации без изменений"
            local default_cfg="$cfg_act_update"
            [[ "${UPDATE_CONFIGS:-1}" -eq 0 ]] && default_cfg="$cfg_act_keep"
            local chosen_cfg
            chosen_cfg="$(gum choose --header "Конфигурационные файлы (~/.config):" --selected "$default_cfg" "$cfg_act_update" "$cfg_act_keep")"
            if [[ "$chosen_cfg" == "$cfg_act_update" ]]; then
                UPDATE_CONFIGS=1
            else
                UPDATE_CONFIGS=0
            fi

            local opt_pkg_fast="Быстрое обновление Inerium (~20 МБ, экономия трафика)"
            local opt_pkg_full="Полное обновление системы (Inerium + pacman -Syu, много трафика)"
            local default_pkg="$opt_pkg_fast"
            [[ "${WITH_SYSUPGRADE:-0}" -eq 1 ]] && default_pkg="$opt_pkg_full"

            gum style --foreground 214 --margin "1 0 0 0" \
                "Внимание: полное обновление системы скачивает обновления всех системных пакетов CachyOS/Arch и может потребовать от 500 МБ до нескольких ГБ трафика."
            local chosen_pkg
            chosen_pkg="$(gum choose --header "Режим обновления системных пакетов:" --selected "$default_pkg" "$opt_pkg_fast" "$opt_pkg_full")"
            if [[ "$chosen_pkg" == "$opt_pkg_full" ]]; then
                WITH_SYSUPGRADE=1
                log "Выбрано полное обновление системы (включая pacman -Syu)."
            else
                WITH_SYSUPGRADE=0
                log "Выбрано быстрое обновление Inerium (без pacman -Syu, проверка недостающих зависимостей)."
            fi
        else
            gum style \
                --border rounded \
                --border-foreground 212 \
                --padding "1 3" \
                --margin "1 0" \
                --bold \
                "INERIUM SHELL — UPDATE" \
                "Fast update of desktop components and configurations"

            local -a config_info=(
                "Current Configuration:"
                "  GPU:             $gpu_info"
                "  Display Manager: $dm_info"
                "  noVPN:           $([[ "$WITH_NOVPN" -eq 1 ]] && echo 'Enabled' || echo 'Disabled')"
                "  Google Drive:    $([[ "$WITH_GDRIVE" -eq 1 ]] && echo 'Enabled' || echo 'Disabled')"
                "  Converter:       $([[ "$WITH_CONVERTER" -eq 1 ]] && echo 'Enabled' || echo 'Disabled')"
                "  Camera:          $([[ "$WITH_CAMERA" -eq 1 ]] && echo 'Enabled' || echo 'Disabled')"
            )
            if [[ "$is_msi" -eq 1 ]]; then
                config_info+=(
                    "  MSI EC (DKMS):   $([[ "$WITH_MSI_EC" -eq 1 ]] && echo 'Enabled' || echo 'Disabled')"
                    "  MSI Fan Reg:     $([[ "$WITH_FAN_REGULATOR" -eq 1 ]] && echo 'Enabled' || echo 'Disabled')"
                )
            fi

            gum style \
                --border normal \
                --border-foreground 99 \
                --padding "0 2" \
                --margin "0 0 1 0" \
                "${config_info[@]}"

            local modify_choice=0
            if gum confirm "Do you want to add new components or modify installed modules?" --default=false; then
                modify_choice=1
            fi

            if [[ "$modify_choice" -eq 1 ]]; then
                local opt_gdrive="Google Drive — cloud integration in Dolphin (rclone)"
                local opt_converter="Dolphin Converter — media/doc conversion & ONNX background removal"
                local opt_cam="Inerium Camera — Android phone webcam over USB/Wi-Fi"
                local opt_novpn="noVPN — run specific apps bypassing VPN tunnel"
                local opt_msi="MSI EC Driver — msi-ec DKMS (80% battery limit & EC telemetry)"
                local opt_fan="MSI Fan Regulator — custom curve daemon (Warning: default BIOS cooling is safer)"

                local -a addon_items=("$opt_gdrive" "$opt_converter" "$opt_novpn" "$opt_cam")
                if [[ "$is_msi" -eq 1 ]]; then
                    addon_items+=("$opt_msi" "$opt_fan")
                fi

                local sel=""
                [[ "$WITH_GDRIVE" -eq 1 ]] && sel="${sel:+$sel,}$opt_gdrive"
                [[ "$WITH_CONVERTER" -eq 1 ]] && sel="${sel:+$sel,}$opt_converter"
                [[ "$WITH_NOVPN" -eq 1 ]] && sel="${sel:+$sel,}$opt_novpn"
                [[ "$WITH_CAMERA" -eq 1 ]] && sel="${sel:+$sel,}$opt_cam"
                [[ "$WITH_MSI_EC" -eq 1 ]] && sel="${sel:+$sel,}$opt_msi"
                [[ "$WITH_FAN_REGULATOR" -eq 1 ]] && sel="${sel:+$sel,}$opt_fan"

                local header_addons="Select modules ('x' key to toggle, Enter to confirm):"
                local chosen_addons
                chosen_addons="$(gum_choose_multiselect "$header_addons" "$sel" "${addon_items[@]}")"

                WITH_GDRIVE=0; WITH_CONVERTER=0; WITH_CAMERA=0; WITH_MSI_EC=0; WITH_FAN_REGULATOR=0; WITH_NOVPN=0
                [[ "$chosen_addons" =~ "$opt_gdrive" ]] && WITH_GDRIVE=1
                [[ "$chosen_addons" =~ "$opt_converter" ]] && WITH_CONVERTER=1
                [[ "$chosen_addons" =~ "$opt_cam" ]] && WITH_CAMERA=1
                [[ "$chosen_addons" =~ "$opt_msi" ]] && WITH_MSI_EC=1
                [[ "$chosen_addons" =~ "$opt_fan" ]] && WITH_FAN_REGULATOR=1 && WITH_MSI_EC=1
                [[ "$chosen_addons" =~ "$opt_novpn" ]] && WITH_NOVPN=1

                if [[ "$WITH_CONVERTER" -eq 1 && ! -f "$HOME/.local/share/dolphin-converter/models/silueta.onnx" ]]; then
                    if gum confirm "Download silueta.onnx (~43 MB for Dolphin AI background removal)?" --default=true; then
                        FETCH_MODEL=1
                    fi
                fi
            else
                log "Keeping installed modules untouched. Proceeding with fast update..."
            fi

            local cfg_act_update="Update configurations (modified files backed up to .bak)"
            local cfg_act_keep="Keep existing user configurations untouched"
            local default_cfg="$cfg_act_update"
            [[ "${UPDATE_CONFIGS:-1}" -eq 0 ]] && default_cfg="$cfg_act_keep"
            local chosen_cfg
            chosen_cfg="$(gum choose --header "Configuration files (~/.config):" --selected "$default_cfg" "$cfg_act_update" "$cfg_act_keep")"
            if [[ "$chosen_cfg" == "$cfg_act_update" ]]; then
                UPDATE_CONFIGS=1
            else
                UPDATE_CONFIGS=0
            fi

            local opt_pkg_fast="Fast Inerium Update (~20 MB, bandwidth saving)"
            local opt_pkg_full="Full System Upgrade (Inerium + pacman -Syu, high bandwidth)"
            local default_pkg="$opt_pkg_fast"
            [[ "${WITH_SYSUPGRADE:-0}" -eq 1 ]] && default_pkg="$opt_pkg_full"

            gum style --foreground 214 --margin "1 0 0 0" \
                "Notice: Full system upgrade downloads updates for all CachyOS/Arch packages and may require 500 MB to several GBs of bandwidth."
            local chosen_pkg
            chosen_pkg="$(gum choose --header "System package update mode:" --selected "$default_pkg" "$opt_pkg_fast" "$opt_pkg_full")"
            if [[ "$chosen_pkg" == "$opt_pkg_full" ]]; then
                WITH_SYSUPGRADE=1
                log "Selected full system upgrade (including pacman -Syu)."
            else
                WITH_SYSUPGRADE=0
                log "Selected fast Inerium update (without pacman -Syu, resolving missing dependencies only)."
            fi
        fi
        return 0
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
            ensure_console_cyrillic_font
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
        local opt_novpn="noVPN — запуск отдельных программ в обход VPN"
        local opt_msi="Драйвер MSI EC — msi-ec DKMS (лимит батареи 80%, чтение датчиков EC)"
        local opt_fan="Регулятор кулеров MSI — msi-fan-regulator (Внимание: штатный BIOS надежнее)"

        local -a addon_items=("$opt_gdrive" "$opt_converter" "$opt_novpn" "$opt_cam")
        if [[ "$is_msi" -eq 1 ]]; then
            addon_items+=("$opt_msi" "$opt_fan")
        fi

        local header_addons="Дополнительные модули (клавиша 'x' — отметить/снять, Enter — подтвердить):"
        local chosen_addons
        chosen_addons="$(gum_choose_multiselect "$header_addons" "$opt_gdrive,$opt_novpn" "${addon_items[@]}")"
        WITH_GDRIVE=0; WITH_CONVERTER=0; WITH_CAMERA=0; WITH_MSI_EC=0; WITH_FAN_REGULATOR=0; WITH_NOVPN=0
        [[ "$chosen_addons" =~ "$opt_gdrive" ]] && WITH_GDRIVE=1
        [[ "$chosen_addons" =~ "$opt_converter" ]] && WITH_CONVERTER=1
        [[ "$chosen_addons" =~ "$opt_cam" ]] && WITH_CAMERA=1
        [[ "$chosen_addons" =~ "$opt_msi" ]] && WITH_MSI_EC=1
        [[ "$chosen_addons" =~ "$opt_fan" ]] && WITH_FAN_REGULATOR=1 && WITH_MSI_EC=1
        [[ "$chosen_addons" =~ "$opt_novpn" ]] && WITH_NOVPN=1

        if [[ "$WITH_CONVERTER" -eq 1 ]]; then
            if gum confirm "Скачать нейросеть silueta.onnx (~43 МБ для вырезания фона в Dolphin)?" \
                --default="$([[ "$FETCH_MODEL" -eq 1 ]] && echo true || echo false)"; then
                FETCH_MODEL=1
            else
                FETCH_MODEL=0
            fi
        fi

        if [[ -d "$HOME/.config/hypr" || -d "$HOME/.config/waybar" ]]; then
            local cfg_inst_fresh="Перезаписать на стандартные Inerium (существующие сохранятся в .bak)"
            local cfg_inst_keep="Сохранить существующие файлы (установить только отсутствующие)"
            local default_inst_cfg="$cfg_inst_fresh"
            [[ "$UPDATE_CONFIGS" -eq 0 ]] && default_inst_cfg="$cfg_inst_keep"
            local chosen_inst_cfg
            chosen_inst_cfg="$(gum choose --header "Обнаружены существующие конфигурации (~/.config):" --selected "$default_inst_cfg" "$cfg_inst_fresh" "$cfg_inst_keep")"
            if [[ "$chosen_inst_cfg" == "$cfg_inst_fresh" ]]; then
                UPDATE_CONFIGS=1
            else
                UPDATE_CONFIGS=0
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
        local opt_novpn="noVPN — run specific apps bypassing VPN tunnel"
        local opt_msi="MSI EC Driver — msi-ec DKMS (80% battery limit & EC telemetry)"
        local opt_fan="MSI Fan Regulator — custom curve daemon (Warning: default BIOS cooling is safer)"

        local -a addon_items=("$opt_gdrive" "$opt_converter" "$opt_novpn" "$opt_cam")
        if [[ "$is_msi" -eq 1 ]]; then
            addon_items+=("$opt_msi" "$opt_fan")
        fi

        local header_addons="Optional Add-on Modules ('x' key to toggle, Enter to confirm):"
        local chosen_addons
        chosen_addons="$(gum_choose_multiselect "$header_addons" "$opt_gdrive,$opt_novpn" "${addon_items[@]}")"
        WITH_GDRIVE=0; WITH_CONVERTER=0; WITH_CAMERA=0; WITH_MSI_EC=0; WITH_FAN_REGULATOR=0; WITH_NOVPN=0
        [[ "$chosen_addons" =~ "$opt_gdrive" ]] && WITH_GDRIVE=1
        [[ "$chosen_addons" =~ "$opt_converter" ]] && WITH_CONVERTER=1
        [[ "$chosen_addons" =~ "$opt_cam" ]] && WITH_CAMERA=1
        [[ "$chosen_addons" =~ "$opt_msi" ]] && WITH_MSI_EC=1
        [[ "$chosen_addons" =~ "$opt_fan" ]] && WITH_FAN_REGULATOR=1 && WITH_MSI_EC=1
        [[ "$chosen_addons" =~ "$opt_novpn" ]] && WITH_NOVPN=1

        if [[ "$WITH_CONVERTER" -eq 1 ]]; then
            if gum confirm "Download silueta.onnx (~43 MB for Dolphin AI background removal)?" \
                --default="$([[ "$FETCH_MODEL" -eq 1 ]] && echo true || echo false)"; then
                FETCH_MODEL=1
            else
                FETCH_MODEL=0
            fi
        fi

        if [[ -d "$HOME/.config/hypr" || -d "$HOME/.config/waybar" ]]; then
            local cfg_inst_fresh="Overwrite with standard Inerium configs (existing backed up to .bak)"
            local cfg_inst_keep="Preserve existing files (only install missing configs)"
            local default_inst_cfg="$cfg_inst_fresh"
            [[ "$UPDATE_CONFIGS" -eq 0 ]] && default_inst_cfg="$cfg_inst_keep"
            local chosen_inst_cfg
            chosen_inst_cfg="$(gum choose --header "Existing configurations detected (~/.config):" --selected "$default_inst_cfg" "$cfg_inst_fresh" "$cfg_inst_keep")"
            if [[ "$chosen_inst_cfg" == "$cfg_inst_fresh" ]]; then
                UPDATE_CONFIGS=1
            else
                UPDATE_CONFIGS=0
            fi
        fi
    fi
}
