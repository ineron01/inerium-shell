#!/usr/bin/env bash
# NetworkManager Dispatcher: Synchronize noVPN table 200 routing on interface state changes
# Installed to /etc/NetworkManager/dispatcher.d/98-novpn-sync.sh

INTERFACE="$1"
ACTION="$2"

# Ignore virtual/tunnel interfaces
if [[ "$INTERFACE" =~ ^(amn|tun|tap|wg|awg|tailscale|virbr|waydroid|docker|veth|br-|lo) ]]; then
    exit 0
fi

case "$ACTION" in
    up|dhcp4-change|connectivity-change)
        if systemctl is-active --quiet novpn.service; then
            /usr/local/bin/novpn-up >/dev/null 2>&1 || true
        elif ip -4 route show default 2>/dev/null | grep -q 'via '; then
            systemctl start --no-block novpn.service >/dev/null 2>&1 || true
        fi
        ;;
    down)
        if systemctl is-active --quiet novpn.service; then
            /usr/local/bin/novpn-up >/dev/null 2>&1 || true
        fi
        ;;
esac
