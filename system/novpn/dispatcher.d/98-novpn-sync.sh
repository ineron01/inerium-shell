#!/usr/bin/env bash
# NetworkManager Dispatcher: Synchronize noVPN table 200 routing on interface state changes
# Installed to /etc/NetworkManager/dispatcher.d/98-novpn-sync.sh

INTERFACE="$1"
ACTION="$2"

# Only act if novpn namespace is initialized
if ! ip netns list 2>/dev/null | grep -qw "novpn"; then
    exit 0
fi

# Ignore virtual/tunnel interfaces
if [[ "$INTERFACE" =~ ^(amn|tun|tap|wg|awg|tailscale|virbr|waydroid|docker|veth|br-|lo) ]]; then
    exit 0
fi

case "$ACTION" in
    up|down|dhcp4-change|connectivity-change)
        /usr/local/bin/novpn-up >/dev/null 2>&1 || true
        ;;
esac
