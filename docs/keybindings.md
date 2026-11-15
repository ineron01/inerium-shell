# 🚀 Inerium Desktop — Hotkeys, Gestures & Waybar Tutorial

[ English | [Русский](keybindings.ru.md) ]

Welcome to **Inerium Desktop**! This desktop environment is built around the **Hyprland** Wayland compositor (Lua API), the **Waybar** status bar, and the native **Inerium** Rust utility suite.

The core design principle is **keyboard ergonomics and automatic tiling**: windows tile cleanly without clutter, and all primary actions are triggered instantly via shortcuts. The system also offers full mouse control and smooth multi-touch gestures for laptop touchpads.

> [!TIP]
> The primary modifier key (**`Super`**) is the **`Windows`** key (or **`Cmd`** on Apple keyboards).

---

## 📑 Quick Cheat Sheet

| Action | Shortcut | Description |
| :--- | :--- | :--- |
| **Application Launcher** | `Super + Space` | Fast application search and launcher (Rofi) |
| **Terminal** | `Super + Enter` | Kitty terminal emulator |
| **File Manager** | `Super + E` | Dolphin file manager |
| **Web Browser** | `Super + W` | Default web browser |
| **Text Editor** | `Super + T` | Default GUI text editor |
| **Calculator** | `Super + C` or `XF86Calculator` | Fast calculator with history |
| **Control Center** | `Super + A` | Inerium quick settings panel (Wi-Fi, Bluetooth, power profiles) |
| **Clipboard History** | `Super + V` | Copied text and image history (`inerium-clipboard`) |
| **Window Switcher** | `Super + Tab` | Search and switch between active windows (Rofi) |
| **System Monitor** | `Ctrl + Shift + Esc` | Process, CPU, GPU, and RAM monitor (`btop`) |
| **Area Screenshot** | `Print` | Interactive area capture with annotation editor (`satty`) |
| **Fullscreen Screenshot** | `Shift + Print` | Instant full-screen screenshot copied to clipboard & disk |
| **Color Picker** | `Super + P` | Grab color from screen with automatic HEX copy (`hyprpicker`) |
| **Lock Screen** | `Super + L` | Screen lock via Hyprlock |
| **Power Menu** | `Super + Alt + C` or `Power` | Power off, reboot, suspend, lock, or log out |
| **Switch Keyboard Layout** | `Alt + Shift` | Toggle layout (English / Russian) |

---

## 🪟 1. Window Management & Tiling

Windows are automatically organized into an ergonomic tiling layout. You can adjust geometry, toggle floating mode, or expand windows to fullscreen.

| Shortcut | Action |
| :--- | :--- |
| `Super + Q` | **Close** active window |
| `Super + Shift + Q` | **Force kill** (SIGKILL) unresponsive window process |
| `Super + Escape` | **Window Killer mode** — cursor turns into a crosshair; click any window to kill it |
| `Super + F` | **Exclusive Fullscreen** (hides Waybar panel) |
| `Super + D` | **Maximize** (fullscreen while keeping Waybar panel visible) |
| `Super + Alt + Space` | **Toggle Floating mode** — detach window from tiling grid |
| `Super + J` | **Toggle Split direction** — switch between horizontal and vertical split |
| `Super + ← / → / ↑ / ↓` | **Change focus** — move focus to adjacent window in given direction |
| `Alt + Tab` | **Cycle focus** — cycle focus through windows in current workspace |
| `Super + Shift + ← / → / ↑ / ↓` | **Move window** — swap window position in tiling grid |
| `Super + Shift + MouseWheel Up / Down` | **Move to monitor** — send active window to previous / next monitor |
| `Super + Ctrl + Shift + ← / →` | **Move to monitor workspace** — send window to next / previous workspace on current monitor |
| `Super + Ctrl + Shift + MouseWheel Up / Down` | **Move to monitor workspace** — send window to next / previous workspace via mouse wheel |

---

## 🖥️ 2. Workspaces & Multi-Monitor Navigation

Workspaces are virtual desktops. When an external monitor is connected, workspaces are cleanly separated across outputs (e.g. 1–3 on laptop display, 4–6 on external monitor).

| Shortcut | Action |
| :--- | :--- |
| `Super + 1..6` | **Switch** to workspace 1..6 |
| `Super + Shift + 1..6` | **Move window** to workspace 1..6 and follow focus |
| `Super + Alt + 1..6` | **Silent move** — send window to workspace 1..6 without switching active workspace |
| `Super + Ctrl + ← / →` | Cycle workspace on current monitor (previous / next) |
| `Super + Ctrl + ↓` | Jump to **first empty** workspace on current monitor |
| `Super + Ctrl + 1..3` | Focus physical monitor 1, 2, or 3 |
| `Super + S` | **Scratchpad** — toggle special floating drawer workspace |
| `Super + Shift + S` | **Send to Scratchpad** — move active window to special background drawer |

---

## 🖱️ 3. Mouse Controls & Screen Magnifier (Zoom)

| Shortcut / Action | Result |
| :--- | :--- |
| `Super + Hold Left Click` | **Drag & move window** (works seamlessly on tiled windows too) |
| `Super + Hold Right Click` | **Resize window** interactively |
| `Super + Mouse Wheel` | Fast cycle through workspaces on current monitor |
| `Super + Ctrl + Mouse Wheel` | Cycle workspaces on current monitor |
| `Super + Plus (+)` / `Minus (-)` | **Screen Magnifier (Zoom)** — smooth hardware zoom from 1.0x to 3.0x (0.3 step) |
| `Super + Keypad [+]` / `[-]` | Screen magnifier zoom via numeric keypad |

---

## 🖐️ 4. Touchpad Gestures

For laptop users, native smooth multi-touch gestures are configured out of the box:

- **4-Finger Swipe Left / Right** — fast switching between workspaces.
- **3-Finger Swipe Down** — close active window (`close`).
- **3-Finger Swipe Up** — toggle fullscreen (`fullscreen`).
- **3-Finger Swipe Left** — toggle window floating / tiling state (`float`).

---

## 🎵 5. Multimedia & Hardware Keys

Hardware volume and brightness changes trigger smooth, non-intrusive on-screen displays (**SwayOSD**):

- **Volume:** `Fn + Volume Up / Down` (or dedicated multimedia keys).
- **Audio Mute:** `Mute` key (or `Fn + Mute`).
- **Mic Mute:** `Mic Mute` key (keyboard indicator LED stays in sync).
- **Brightness:** `Fn + Brightness Up / Down`.
- **Media Playback:** `Play / Pause`, `Next Track`, `Previous Track` (controls Spotify, browser players, MPV, etc.).

---

## 💡 6. Waybar Interactive Features & Secrets

The top Inerium Waybar status panel is fully interactive. Most modules respond to clicks, right-clicks, and wheel scrolling:

- **OS Icon (Left):**
  - *Left Click:* Open application launcher (Rofi).
  - *Right Click:* Open clipboard history manager (`inerium-clipboard`).
- **Workspaces Widget:**
  - *Left Click:* Switch to clicked workspace.
  - *Scroll Wheel:* Cycle next / previous workspace on current monitor.
- **Hardware Monitor (CPU / RAM / Temperature):**
  - *Left Click:* Toggle display mode (CPU ↔ RAM usage).
  - *Right Click:* Launch detailed interactive system monitor (`btop`).
- **Media Player Widget (MPRIS track info):**
  - *Left Click:* Open interactive track card with playback controls (`inerium-popup media`).
  - *Right Click:* Play / Pause current track.
  - *Scroll Wheel:* Skip to next / previous track.
- **Empty Panel Space (Left & Right of Clock):**
  - *Left / Right Click:* Instantly open **Inerium Control Center** (`inerium-popup control`).
- **Clock & Calendar (Center):**
  - *Left Click:* Toggle between compact (`HH:MM`) and full (`Day, DD Mon HH:MM`) date format.
  - *Right Click:* Reset calendar view to current month.
  - *Scroll Wheel:* Browse calendar months forward and backward.
- **Keyboard Layout Indicator:**
  - *Left Click:* Toggle keyboard layout (US ↔ RU).
- **Audio Module:**
  - *Left Click:* Open audio output / input device selector popup (`inerium-popup audio`).
  - *Right Click:* Mute / Unmute audio output.
  - *Middle Click:* Open advanced PulseAudio / PipeWire mixer (`pavucontrol`).
  - *Scroll Wheel:* Fine-tune volume in 2% steps with SwayOSD feedback.
- **Network / Wi-Fi Module:**
  - *Left Click:* Open Wi-Fi networks and connection menu (`inerium-popup network`).
  - *Right Click:* Fast toggle Wi-Fi radio on / off (`inerium-toggle-network.sh`).
  - *Middle Click:* Open Network Connections editor (`nm-connection-editor`).
- **Wi-Fi Hotspot Module:**
  - *Left Click:* Fast toggle Wi-Fi hotspot on / off (`inerium-hotspot toggle`).
- **Bluetooth Module:**
  - *Left Click:* Open list of paired Bluetooth devices for 1-click connection (`inerium-popup bluetooth`).
  - *Right Click:* Fast toggle Bluetooth radio on / off (`inerium-toggle-bluetooth.sh`).
  - *Middle Click:* Open Bluetooth device manager (`blueman-manager`).
- **Battery Module:**
  - *Left Click:* Toggle display between charge percentage (%) and estimated remaining battery runtime.
- **Do Not Disturb (DND Bell):**
  - *Left / Right Click:* Toggle Do Not Disturb mode (mutes on-screen notification popups).
- **Control Center Button ():**
  - *Left / Right Click:* Open Inerium Control Center (`inerium-popup control`).
- **Power Button (⏻):**
  - *Left Click:* Open Power menu popup (`inerium-popup power` — Shutdown, Reboot, Suspend, Lock, Logout).
