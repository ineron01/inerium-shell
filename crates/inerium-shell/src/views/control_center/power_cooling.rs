use inerium_i18n::tr;

use super::{ControlCenterModel, ControlCenterMsg};
use crate::hw;

pub fn compute_fan_string(is_cb: bool) -> String {
    let (cpu_rpm, gpu_rpm, cpu_pct, gpu_pct) = hw::get_fan_telemetry();
    let c_pct = cpu_pct.unwrap_or(0);
    let g_pct = gpu_pct.unwrap_or(0);

    if is_cb {
        match (cpu_rpm, gpu_rpm) {
            (Some(cr), Some(gr)) if cr > 0 && gr > 0 => {
                format!("󰈐 Cooler Boost: CPU {} RPM  ·  GPU {} RPM", cr, gr)
            }
            _ => tr(
                "󰈐 Cooler Boost: maximum speed (100%)",
                "󰈐 Cooler Boost: максимальные обороты (100%)",
            )
            .to_string(),
        }
    } else {
        let mut parts = Vec::new();
        if let Some(cr) = cpu_rpm {
            if cr > 0 {
                if cpu_pct.is_some() {
                    parts.push(format!("CPU {}% ({} RPM)", c_pct, cr));
                } else {
                    parts.push(format!("Fan 1: {} RPM", cr));
                }
            } else if cpu_pct.is_some() {
                parts.push(format!("CPU {}%", c_pct));
            }
        } else if cpu_pct.is_some() {
            parts.push(format!("CPU {}%", c_pct));
        }

        if let Some(gr) = gpu_rpm {
            if gr > 0 {
                if gpu_pct.is_some() {
                    parts.push(format!("GPU {}% ({} RPM)", g_pct, gr));
                } else {
                    parts.push(format!("Fan 2: {} RPM", gr));
                }
            } else if gpu_pct.is_some() {
                parts.push(format!("GPU {}%", g_pct));
            }
        } else if gpu_pct.is_some() {
            parts.push(format!("GPU {}%", g_pct));
        }

        if parts.is_empty() {
            tr("󰈨  Fans: active", "󰈨  Кулеры: активный режим").to_string()
        } else {
            format!("󰈨  {}: {}", tr("Fans", "Кулеры"), parts.join(" · "))
        }
    }
}

pub fn handle_power_cooling_msg(model: &mut ControlCenterModel, msg: &ControlCenterMsg) -> bool {
    match msg {
        ControlCenterMsg::SetProfile(profile) => {
            model.current_profile = profile.clone();
            hw::set_power_profile(profile);
            true
        }
        ControlCenterMsg::RefreshFanTelemetry => {
            if model.has_fan_telem {
                model.fan_str = compute_fan_string(model.cooler_boost);
            }
            true
        }
        ControlCenterMsg::ToggleCoolerBoost => {
            model.cooler_boost = !model.cooler_boost;
            hw::set_cooler_boost(model.cooler_boost);
            if model.has_fan_telem {
                model.fan_str = compute_fan_string(model.cooler_boost);
            }
            true
        }
        ControlCenterMsg::ToggleBatteryThreshold => {
            let next = if model.battery_threshold < 100 { 100 } else { 80 };
            model.battery_threshold = next;
            hw::set_battery_threshold(next);
            true
        }
        _ => false,
    }
}
