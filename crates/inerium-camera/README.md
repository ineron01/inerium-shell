# inerium-camera

High-performance virtual webcam manager and streamer for Inerium Shell.

## Overview

Routes Android back/front camera streams (via USB ADB or Wi-Fi) and internal laptop webcams directly into a standard `/dev/video0` virtual camera device (`v4l2loopback`).

## Key Capabilities

- **GTK4 Stream Controller:** Simple GUI for camera preview, video source switching (Back, Front, Laptop), and aspect ratio rotation.
- **Low-Latency Pipeline:** GStreamer-accelerated pipeline with hardware H.264 decoding and scrcpy integration.
- **Persistent Virtual Device:** Automatically configures `/dev/video0` through system udev rules and systemd user services.
