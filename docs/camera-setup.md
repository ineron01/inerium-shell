# Inerium Camera — Android Phone Setup Guide

`inerium-camera` routes video streams from Android smartphone cameras (back/front, 16:9 or 9:16 portrait) or the built-in laptop camera directly into a persistent Linux virtual webcam node (`/dev/video0`, named `PhoneWebcam`).

It utilizes standard Android Debug Bridge (`adb`) and `scrcpy` over USB or Wi-Fi. **No third-party Android APKs or proprietary apps are required.**

---

## 1. Phone Preparation (One-Time Setup)

1. Open **Settings** on your Android smartphone.
2. Go to **About phone** (or **Software information**).
3. Tap **Build number** 7 times rapidly until you see *"You are now a developer!"*.
4. Return to **Settings** -> **System** (or **Additional settings**) -> **Developer options**.
5. Enable **USB debugging**.
   *(Note for Xiaomi / MIUI / HyperOS users: also enable "Install via USB" and "USB debugging (Security settings)" if prompted).*

---

## 2. First Connection (USB)

1. Connect your smartphone to the PC with a USB cable.
2. A prompt will appear on the phone: **"Allow USB debugging?"**.
3. Check **"Always allow from this computer"** and tap **Allow**.
4. Run `inerium-camera` (or click the desktop launcher **Inerium Camera**):
   - Your device will be detected in the source list.
   - Click **Start** to begin streaming.

---

## 3. Wireless Mode (Wi-Fi / Hotspot)

You do not need to keep the phone tethered with a cable:

1. Ensure the phone and PC are connected to the same Wi-Fi network (or connect the phone to the laptop's hotspot, e.g., `inerium-hotspot`).
2. With the phone plugged in via USB once, click **"Enable Wi-Fi Mode"** in the `inerium-camera` GUI (or run `adb tcpip 5555` in terminal).
3. Unplug the USB cable.
4. `inerium-camera` will scan the local subnet / hotspot ARP table, detect the phone wirelessly, and connect automatically.

---

## 4. Usage in Applications

Select **PhoneWebcam** (`/dev/video0`) in:
- **Discord** -> Voice & Video -> Camera
- **Telegram** -> Settings -> Calls -> Camera
- **Web Browsers** (Chrome, Firefox, Chromium) for Google Meet, Zoom Web, Telemost
- **OBS Studio** -> Video Capture Device (V4L2) -> `/dev/video0`

### Hot-Swapping & Controls
- You can switch between **Back camera**, **Front camera**, and **Laptop webcam** on the fly without closing or restarting the active call.
- Switch between **16:9 landscape** and **9:16 portrait** (with automatic studio black sidebars).
- Turn the phone's **Flashlight/Torch** on and off from the GUI.
- If the stream is stopped or disconnected, `/dev/video0` holds a clean studio-black image so the meeting application does not crash or display static noise.
