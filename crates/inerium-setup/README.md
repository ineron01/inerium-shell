# inerium-setup

GTK4 + VTE-4 onboarding and desktop customization wizard for Inerium Shell.

## Overview

`inerium-setup` is launched automatically upon first login (`inerium-setup --autostart`) or manually from the application launcher.

## Features

- **Displays & Layout:** Graphical monitor resolution, refresh rate, and positioning adjustments with direct «📝 Open config» editor button for `monitors.lua`.
- **Keybindings Quick-Access:** Direct «📝 Keybinds» editor shortcut for `keybinds.lua`.
- **Visual Theming:** Wallpaper selector and dynamic Theme Gallery with FlowBox preview cards for base themes and the `themes/extra/` pool (promoting extra palettes on selection).
- **Embedded Sign-in Terminals:** Integrates account sign-in steps (GitHub CLI, Google Drive rclone) directly inside embedded GTK4 terminal widgets (`vte4`) without spawning loose terminal windows.
