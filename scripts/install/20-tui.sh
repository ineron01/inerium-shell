#!/usr/bin/env bash
# ============================================================================
# scripts/install/20-tui.sh — Interactive Bilingual TUI (gum) & prompts
# ============================================================================
# shellcheck disable=SC2034 # State variables consumed across installer stages

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
            # shellcheck disable=SC2024
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
        if [[ "${IS_UPDATE:-0}" -eq 1 ]]; then
            if [[ -f "$HOME/.config/locale.conf" ]] && grep -q "ru_RU" "$HOME/.config/locale.conf"; then
                INSTALL_LANG="ru"
            elif [[ "${LANG:-}" =~ ^ru || "${LC_ALL:-}" =~ ^ru ]]; then
                INSTALL_LANG="ru"
            else
                INSTALL_LANG="en"
            fi
        else
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
    fi

    # Define localized addon label strings (CRITICAL: zero commas in any string to prevent gum --selected splitting bugs)
    local opt_gdrive opt_converter opt_cam opt_novpn opt_github opt_greeter opt_msi opt_fan
    if [[ "$INSTALL_LANG" == "ru" ]]; then
        opt_gdrive="Google Drive — интеграция облака в Dolphin (rclone)"
        opt_converter="Dolphin Converter — конвертация файлов (LibreOffice / Pandoc / ONNX)"
        opt_cam="Inerium Camera — веб-камера из Android (USB или Wi-Fi)"
        opt_novpn="noVPN — запуск отдельных программ в обход VPN"
        opt_github="GitHub CLI — вход git push по HTTPS и подпись коммитов"
        opt_greeter="Экран входа — inerium-greeter дисплейный менеджер greetd"
        opt_msi="Драйвер MSI EC — msi-ec DKMS (лимит батареи 80% и датчики EC)"
        opt_fan="Регулятор кулеров MSI — msi-fan-regulator (только для EC 158NIMS1.10F)"
    else
        opt_gdrive="Google Drive — cloud integration in Dolphin (rclone)"
        opt_converter="Dolphin Converter — media/doc conversion & ONNX background removal"
        opt_cam="Inerium Camera — Android phone webcam over USB/Wi-Fi"
        opt_novpn="noVPN — run specific apps bypassing VPN tunnel"
        opt_github="GitHub CLI — login git push over HTTPS and commit signing"
        opt_greeter="Login Screen — inerium-greeter greetd display manager"
        opt_msi="MSI EC Driver — msi-ec DKMS (80% battery limit & EC telemetry)"
        opt_fan="MSI Fan Regulator — custom curve daemon (fan curve control)"
    fi

    # Helper: Build addons item array and selected comma-separated string
    local -a addon_items=()
    local sel=""
    build_addons_list_and_sel() {
        addon_items=("$opt_gdrive" "$opt_converter" "$opt_novpn" "$opt_cam" "$opt_github")
        if [[ "${IS_UPDATE:-0}" -eq 1 ]]; then
            addon_items+=("$opt_greeter")
        fi
        if [[ "$is_msi" -eq 1 ]]; then
            addon_items+=("$opt_msi" "$opt_fan")
        fi

        local -a sel_array=()
        [[ "${WITH_GDRIVE:-0}" -eq 1 ]] && sel_array+=("$opt_gdrive")
        [[ "${WITH_CONVERTER:-0}" -eq 1 ]] && sel_array+=("$opt_converter")
        [[ "${WITH_NOVPN:-0}" -eq 1 ]] && sel_array+=("$opt_novpn")
        [[ "${WITH_CAMERA:-0}" -eq 1 ]] && sel_array+=("$opt_cam")
        [[ "${WITH_GITHUB:-0}" -eq 1 ]] && sel_array+=("$opt_github")
        if [[ "${IS_UPDATE:-0}" -eq 1 && "${WITH_GREETER:-0}" -eq 1 ]]; then
            sel_array+=("$opt_greeter")
        fi
        if [[ "$is_msi" -eq 1 ]]; then
            [[ "${WITH_MSI_EC:-0}" -eq 1 ]] && sel_array+=("$opt_msi")
            [[ "${WITH_FAN_REGULATOR:-0}" -eq 1 ]] && sel_array+=("$opt_fan")
        fi

        local IFS=','
        sel="${sel_array[*]}"
    }

    # Helper: Apply selected items from gum multiselect output
    apply_chosen_addons() {
        local chosen="$1"
        WITH_GDRIVE=0; WITH_CONVERTER=0; WITH_CAMERA=0; WITH_NOVPN=0; WITH_GITHUB=0
        WITH_MSI_EC=0; WITH_FAN_REGULATOR=0
        if [[ "${IS_UPDATE:-0}" -eq 1 ]]; then
            WITH_GREETER=0
        fi

        if grep -Fqx "$opt_gdrive" <<< "$chosen"; then WITH_GDRIVE=1; fi
        if grep -Fqx "$opt_converter" <<< "$chosen"; then WITH_CONVERTER=1; FETCH_MODEL=1; fi
        if grep -Fqx "$opt_cam" <<< "$chosen"; then WITH_CAMERA=1; fi
        if grep -Fqx "$opt_novpn" <<< "$chosen"; then WITH_NOVPN=1; fi
        if grep -Fqx "$opt_github" <<< "$chosen"; then WITH_GITHUB=1; fi
        if [[ "${IS_UPDATE:-0}" -eq 1 ]]; then
            if grep -Fqx "$opt_greeter" <<< "$chosen"; then WITH_GREETER=1; fi
        fi
        if [[ "$is_msi" -eq 1 ]]; then
            if grep -Fqx "$opt_msi" <<< "$chosen"; then WITH_MSI_EC=1; fi
            if grep -Fqx "$opt_fan" <<< "$chosen"; then WITH_FAN_REGULATOR=1; WITH_MSI_EC=1; fi
        fi
    }

    # ========================================================================
    # Update Mode Workflow
    # ========================================================================
    if [[ "${IS_UPDATE:-0}" -eq 1 ]]; then
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
                "  inerium-greeter:     $([[ "$WITH_GREETER" -eq 1 ]] && echo 'Включен' || echo 'Выключен')"
                "  GitHub CLI:          $([[ "${WITH_GITHUB:-0}" -eq 1 ]] && echo 'Включен' || echo 'Выключен')"
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
                build_addons_list_and_sel
                local header_addons="Выберите состав модулей (клавиша 'x' — отметить/снять, Enter — подтвердить):"
                local chosen_addons
                chosen_addons="$(gum_choose_multiselect "$header_addons" "$sel" "${addon_items[@]}")"
                apply_chosen_addons "$chosen_addons"
            else
                log "Сохранение текущего набора модулей. Запуск быстрого обновления..."
            fi

            local cfg_act_keep="Не трогать конфиги (оставить текущие без изменений)"
            local cfg_act_ui="Обновить только оформление и оболочку (Waybar, Rofi, темы, анимации — бинды и правила не трогать)"
            local cfg_act_all="Обновить все конфигурации (существующие сохранятся в .bak)"
            local default_cfg="$cfg_act_keep"
            if [[ "${UPDATE_CONFIGS_MODE:-none}" == "ui" ]]; then
                default_cfg="$cfg_act_ui"
            elif [[ "${UPDATE_CONFIGS_MODE:-none}" == "all" || "${UPDATE_CONFIGS:-0}" -eq 1 ]]; then
                default_cfg="$cfg_act_all"
            fi
            local chosen_cfg
            chosen_cfg="$(gum choose --header "Конфигурационные файлы (~/.config):" --selected "$default_cfg" "$cfg_act_keep" "$cfg_act_ui" "$cfg_act_all")"
            if [[ "$chosen_cfg" == "$cfg_act_ui" ]]; then
                UPDATE_CONFIGS_MODE="ui"
                UPDATE_CONFIGS=1
            elif [[ "$chosen_cfg" == "$cfg_act_all" ]]; then
                UPDATE_CONFIGS_MODE="all"
                UPDATE_CONFIGS=1
            else
                UPDATE_CONFIGS_MODE="none"
                UPDATE_CONFIGS=0
            fi

            local opt_assets_keep="Сохранить текущие (не восстанавливать удалённые обои и темы)"
            local opt_assets_restore="Восстановить стандартные обои и темы оформления"
            local default_assets="$opt_assets_keep"
            [[ "${RESTORE_ASSETS:-0}" -eq 1 ]] && default_assets="$opt_assets_restore"
            local chosen_assets
            chosen_assets="$(gum choose --header "Стандартные обои и темы оформления:" --selected "$default_assets" "$opt_assets_keep" "$opt_assets_restore")"
            if [[ "$chosen_assets" == "$opt_assets_restore" ]]; then
                RESTORE_ASSETS=1
                log "Выбрано восстановление стандартных обоев и тем."
            else
                RESTORE_ASSETS=0
                log "Стандартные обои и темы не будут восстановлены (пользовательские изменения сохранены)."
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
                "  inerium-greeter: $([[ "$WITH_GREETER" -eq 1 ]] && echo 'Enabled' || echo 'Disabled')"
                "  GitHub CLI:      $([[ "${WITH_GITHUB:-0}" -eq 1 ]] && echo 'Enabled' || echo 'Disabled')"
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
                build_addons_list_and_sel
                local header_addons="Select modules ('x' key to toggle, Enter to confirm):"
                local chosen_addons
                chosen_addons="$(gum_choose_multiselect "$header_addons" "$sel" "${addon_items[@]}")"
                apply_chosen_addons "$chosen_addons"
            else
                log "Keeping installed modules untouched. Proceeding with fast update..."
            fi

            local cfg_act_keep="Do not touch configs (keep current untouched)"
            local cfg_act_ui="Update UI and shell only (Waybar, Rofi, themes, animations; keep keybinds & rules)"
            local cfg_act_all="Update all configurations (existing preserved in .bak)"
            local default_cfg="$cfg_act_keep"
            if [[ "${UPDATE_CONFIGS_MODE:-none}" == "ui" ]]; then
                default_cfg="$cfg_act_ui"
            elif [[ "${UPDATE_CONFIGS_MODE:-none}" == "all" || "${UPDATE_CONFIGS:-0}" -eq 1 ]]; then
                default_cfg="$cfg_act_all"
            fi
            local chosen_cfg
            chosen_cfg="$(gum choose --header "Configuration files (~/.config):" --selected "$default_cfg" "$cfg_act_keep" "$cfg_act_ui" "$cfg_act_all")"
            if [[ "$chosen_cfg" == "$cfg_act_ui" ]]; then
                UPDATE_CONFIGS_MODE="ui"
                UPDATE_CONFIGS=1
            elif [[ "$chosen_cfg" == "$cfg_act_all" ]]; then
                UPDATE_CONFIGS_MODE="all"
                UPDATE_CONFIGS=1
            else
                UPDATE_CONFIGS_MODE="none"
                UPDATE_CONFIGS=0
            fi

            local opt_assets_keep="Keep current (do not restore deleted wallpapers and themes)"
            local opt_assets_restore="Restore missing standard wallpapers and themes"
            local default_assets="$opt_assets_keep"
            [[ "${RESTORE_ASSETS:-0}" -eq 1 ]] && default_assets="$opt_assets_restore"
            local chosen_assets
            chosen_assets="$(gum choose --header "Standard wallpapers and themes:" --selected "$default_assets" "$opt_assets_keep" "$opt_assets_restore")"
            if [[ "$chosen_assets" == "$opt_assets_restore" ]]; then
                RESTORE_ASSETS=1
                log "Selected restoring standard wallpapers and themes."
            else
                RESTORE_ASSETS=0
                log "Standard wallpapers and themes will not be restored (user modifications preserved)."
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

    # ========================================================================
    # Fresh Install Workflow
    # ========================================================================
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

        build_addons_list_and_sel
        local header_addons="Дополнительные модули (клавиша 'x' — отметить/снять, Enter — подтвердить):"
        local chosen_addons
        chosen_addons="$(gum_choose_multiselect "$header_addons" "$sel" "${addon_items[@]}")"
        apply_chosen_addons "$chosen_addons"

        if [[ -d "$HOME/.config/hypr" || -d "$HOME/.config/waybar" ]]; then
            local cfg_inst_keep="Сохранить существующие файлы (не трогать текущие настройки)"
            local cfg_inst_ui="Обновить только оформление и оболочку (Waybar, Rofi, темы — правила и бинды оставить)"
            local cfg_inst_fresh="Перезаписать все конфигурации на стандартные (существующие сохранятся в .bak)"
            local default_inst_cfg="$cfg_inst_fresh"
            if [[ "${UPDATE_CONFIGS_MODE:-none}" == "none" || "$UPDATE_CONFIGS" -eq 0 ]]; then
                default_inst_cfg="$cfg_inst_keep"
            elif [[ "${UPDATE_CONFIGS_MODE:-none}" == "ui" ]]; then
                default_inst_cfg="$cfg_inst_ui"
            fi
            local chosen_inst_cfg
            chosen_inst_cfg="$(gum choose --header "Обнаружены существующие конфигурации (~/.config):" --selected "$default_inst_cfg" "$cfg_inst_keep" "$cfg_inst_ui" "$cfg_inst_fresh")"
            if [[ "$chosen_inst_cfg" == "$cfg_inst_ui" ]]; then
                UPDATE_CONFIGS_MODE="ui"
                UPDATE_CONFIGS=1
            elif [[ "$chosen_inst_cfg" == "$cfg_inst_fresh" ]]; then
                UPDATE_CONFIGS_MODE="all"
                UPDATE_CONFIGS=1
            else
                UPDATE_CONFIGS_MODE="none"
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

        build_addons_list_and_sel
        local header_addons="Optional Add-on Modules ('x' key to toggle, Enter to confirm):"
        local chosen_addons
        chosen_addons="$(gum_choose_multiselect "$header_addons" "$sel" "${addon_items[@]}")"
        apply_chosen_addons "$chosen_addons"

        if [[ -d "$HOME/.config/hypr" || -d "$HOME/.config/waybar" ]]; then
            local cfg_inst_keep="Preserve existing files (do not touch current user configs)"
            local cfg_inst_ui="Update UI and shell only (Waybar, Rofi, themes, animations; keep keybinds & rules)"
            local cfg_inst_fresh="Overwrite all configurations with standard Inerium (existing backed up to .bak)"
            local default_inst_cfg="$cfg_inst_fresh"
            if [[ "${UPDATE_CONFIGS_MODE:-none}" == "none" || "$UPDATE_CONFIGS" -eq 0 ]]; then
                default_inst_cfg="$cfg_inst_keep"
            elif [[ "${UPDATE_CONFIGS_MODE:-none}" == "ui" ]]; then
                default_inst_cfg="$cfg_inst_ui"
            fi
            local chosen_inst_cfg
            chosen_inst_cfg="$(gum choose --header "Existing configurations detected (~/.config):" --selected "$default_inst_cfg" "$cfg_inst_keep" "$cfg_inst_ui" "$cfg_inst_fresh")"
            if [[ "$chosen_inst_cfg" == "$cfg_inst_ui" ]]; then
                UPDATE_CONFIGS_MODE="ui"
                UPDATE_CONFIGS=1
            elif [[ "$chosen_inst_cfg" == "$cfg_inst_fresh" ]]; then
                UPDATE_CONFIGS_MODE="all"
                UPDATE_CONFIGS=1
            else
                UPDATE_CONFIGS_MODE="none"
                UPDATE_CONFIGS=0
            fi
        fi
    fi
}
