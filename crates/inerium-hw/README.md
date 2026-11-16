# inerium-hw

Universal hardware discovery, sysfs abstractions, and power telemetry primitives for Inerium Shell.

## Capabilities

- **Battery & AC Detection:** Discovers `/sys/class/power_supply/` nodes dynamically without hardcoded IDs (`BAT0`, `BAT1`, `ADP1`). Supports dual-battery setups, Type-C Power Delivery, and vendor charge thresholds (`charge_control_end_threshold`, `charge_stop_threshold`).
- **Cooling & Thermal Telemetry:** Abstracted temperature sensor readers prioritizing CPU package dies over peripheral NVMe/GPU sensors.
- **Graceful Desktop Fallback:** Dynamically detects missing battery/backlight controllers and returns graceful fallback states instead of failing.
