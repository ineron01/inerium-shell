# inerium-setup

GTK4 + VTE-4 onboarding and desktop customization wizard for Inerium Shell.

## Overview

`inerium-setup` is launched automatically upon first login (`inerium-setup --autostart`) or manually from the application launcher.

## Features

- **Displays & Layout:** Graphical monitor resolution, refresh rate, and positioning adjustments.
- **Visual Theming:** Wallpaper selector and dynamic Inerium Dragon theme controls.
- **Embedded Sign-in Terminals:** Integrates account sign-in steps (GitHub CLI, Google Drive rclone) directly inside embedded GTK4 terminal widgets (`vte4`) without spawning loose terminal windows.
