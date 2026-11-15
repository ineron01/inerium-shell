#!/usr/bin/env bash
# ============================================================================
# scripts/install/10-detect.sh — Hardware, Display Manager & Session Detection
# ============================================================================

detect_gpu_label() {
    local has_nv=0 has_amd=0 has_intel=0 v vf
    for vf in /sys/class/drm/card*/device/vendor; do
        [[ -r "$vf" ]] || continue
        v="$(cat "$vf" 2>/dev/null || true)"
        case "$v" in
            0x10de) has_nv=1 ;;
            0x1002) has_amd=1 ;;
            0x8086) has_intel=1 ;;
        esac
    done
    if [[ "$has_amd" -eq 1 && "$has_nv" -eq 1 ]]; then
        echo "Hybrid (AMD + NVIDIA)"
    elif [[ "$has_intel" -eq 1 && "$has_nv" -eq 1 ]]; then
        echo "Hybrid (Intel + NVIDIA)"
    elif [[ "$has_nv" -eq 1 ]]; then
        echo "NVIDIA"
    elif [[ "$has_amd" -eq 1 ]]; then
        echo "AMD"
    elif [[ "$has_intel" -eq 1 ]]; then
        echo "Intel"
    else
        echo "Unknown"
    fi
}

detect_chassis_label() {
    if compgen -G "/sys/class/power_supply/BAT*" >/dev/null 2>&1; then
        echo "Laptop (battery detected)"
    else
        echo "Desktop"
    fi
}

detect_current_dm() {
    local dm
    for dm in sddm gdm lightdm greetd; do
        if systemctl is-enabled "${dm}.service" >/dev/null 2>&1 || systemctl is-active "${dm}.service" >/dev/null 2>&1; then
            echo "$dm"
            return 0
        fi
    done
    echo "none"
}

detect_current_session() {
    local de="${XDG_CURRENT_DESKTOP:-${DESKTOP_SESSION:-}}"
    if [[ -n "$de" ]]; then
        if [[ -n "${WAYLAND_DISPLAY:-}" ]]; then
            echo "$de (Wayland)"
        elif [[ -n "${DISPLAY:-}" ]]; then
            echo "$de (X11)"
        else
            echo "$de"
        fi
    elif [[ -n "${WAYLAND_DISPLAY:-}" ]]; then
        echo "Wayland"
    elif [[ -n "${DISPLAY:-}" ]]; then
        echo "X11"
    else
        echo "TTY"
    fi
}

detect_is_msi_hardware() {
    local sys_vendor=""
    [[ -r /sys/class/dmi/id/sys_vendor ]] && sys_vendor="$(cat /sys/class/dmi/id/sys_vendor 2>/dev/null || true)"
    if [[ "$sys_vendor" =~ [Mm][Ss][Ii] ]] || [[ "$sys_vendor" =~ Micro-Star ]]; then
        return 0
    fi
    return 1
}
