# Inerium Shell — Optional Modules & Add-ons

Inerium Shell includes optional system modules that can be selected during installation via the interactive TUI or CLI flags (`--with-camera`, `--with-msi-ec`, `--with-novpn`, `--all-addons`).

---

## 1. Inerium Camera (`--with-camera`)

- **Description:** Native Rust/GTK4 virtual webcam manager and streamer. Turns an Android smartphone into a high-definition (1080p @ 30fps) webcam for Linux via USB ADB or wireless Wi-Fi/Hotspot.
- **Components:**
  - `crates/inerium-camera` -> `~/.local/bin/inerium-camera` (alias `phone-webcam`)
  - `~/.config/systemd/user/inerium-camera-bridge.service`
  - `/etc/udev/rules.d/99-phone-webcam.rules`
  - `/etc/modprobe.d/v4l2loopback.conf`
- **Key Features:**
  - Exposes a persistent `/dev/video0` node (`PhoneWebcam`) that remains active across stream stops (streaming studio black), preventing meeting apps (Discord, Telegram, Chrome, OBS) from crashing.
  - Hot-swapping between phone back camera, front camera, and laptop webcam during active calls.
  - Automatic 16:9 landscape and 9:16 portrait modes with true pillarboxed 1:1 aspect ratio.
- **Setup Guide:** See [Android Phone Setup Guide](camera-setup.md).

---

## 2. MSI Cooling & Fan Regulator (`--with-msi-ec`)

- **Description:** Upstream `msi-ec` DKMS kernel driver (`msi-ec-dkms-git` package from AUR) and background software fan regulator daemon (`msi-fan-regulator`).
- **Hardware Compatibility & Notice:**
  - Starting with version 0.18 (PR #773), `msi-ec` natively supports the Bravo 15 C7VE / C7V / C7UCX family out of the box via EC firmware prefix matching (`158NIMS1.1`).
  - The fan regulator daemon monitors CPU/GPU temperatures and dynamically regulates fan speed curves via `msi-ec` sysfs registers.
  - Designed for MSI laptops where standard Linux fan curves either fail to cool or let fans run at excessive noise levels.
- **Components:**
  - `msi-ec-dkms-git` package (AUR)
  - `msi-fan-regulator` binary (built from `crates/inerium-tools` with `--features msi-ec`)
  - `/etc/systemd/system/msi-fan-regulator.service`
  - `/etc/udev/rules.d/99-msi-ec.rules`
  - `/etc/modules-load.d/msi-ec.conf`

---

## 3. noVPN Split-Tunneling (`--with-novpn`)

- **Description:** Lightweight network-namespace-based split-tunneling utility to route specific applications directly through the physical ISP gateway, bypassing any active global VPN (AmneziaVPN, WireGuard, OpenVPN).
- **Components:**
  - `/usr/local/bin/novpn`
  - `/usr/local/bin/novpn-exec`
  - `/usr/local/bin/novpn-up`
  - `/usr/local/bin/novpn-down`
  - `/etc/systemd/system/novpn.service`
- **Usage:**
  - `novpn curl https://ifconfig.me` — runs curl showing physical IP address.
  - `novpn google-chrome-stable` — runs browser without VPN.
  - `novpn status` — displays connectivity and public IP for both VPN host and direct network namespace.
