-- Hyprland default applications & global variables

TERMINAL     = "kitty"
FILE_MANAGER = "dolphin"
BROWSER      = "google-chrome-stable"
EDITOR       = "gnome-text-editor --new-window"
CALCULATOR   = "gnome-calculator"

-- Fallback monitor identifiers (overridden by host-specific config.monitors_local if present)
LAPTOP_MONITOR  = LAPTOP_MONITOR or "eDP-1"
MONITOR1        = MONITOR1 or LAPTOP_MONITOR
MONITOR2        = MONITOR2 or "HDMI-A-1"
MONITOR3        = MONITOR3 or "eDP-2"
PRIMARY_MONITOR = PRIMARY_MONITOR or MONITOR2

-- Default workspaces count per monitor
NUM_WPM = 3
