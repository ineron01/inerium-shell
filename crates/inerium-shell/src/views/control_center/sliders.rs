use super::{ControlCenterModel, ControlCenterMsg};
use crate::hw;

pub fn handle_sliders_msg(model: &mut ControlCenterModel, msg: &ControlCenterMsg) -> bool {
    match msg {
        ControlCenterMsg::ToggleVolumeMute => {
            model.volume_muted = !model.volume_muted;
            hw::toggle_volume_mute();
            true
        }
        ControlCenterMsg::SetVolume(val) => {
            if model.volume == *val {
                return true;
            }
            model.volume = *val;
            hw::set_volume(*val);
            true
        }
        ControlCenterMsg::SetLaptopBrightness(val) => {
            if model.laptop_brightness == *val {
                return true;
            }
            model.laptop_brightness = *val;
            hw::set_laptop_brightness(*val);
            true
        }
        ControlCenterMsg::SetDdcBrightness(val) => {
            if model.ddc_brightness == *val {
                return true;
            }
            model.ddc_brightness = *val;
            hw::set_ddc_brightness_async(*val);
            true
        }
        ControlCenterMsg::DdcUpdated(val) => {
            model.ddc_brightness = *val;
            true
        }
        ControlCenterMsg::SetNightLightEnabled(enabled) => {
            if model.night_light_enabled == *enabled {
                return true;
            }
            let cfg = hw::night_light::set_enabled(*enabled);
            model.night_light_enabled = cfg.enabled;
            model.night_light_temp = cfg.temperature;
            model.night_light_schedule = cfg.schedule_enabled;
            model.night_light_schedule_str = format!(
                "{:02}:{:02}–{:02}:{:02}",
                cfg.start_hour, cfg.start_minute, cfg.end_hour, cfg.end_minute
            );
            true
        }
        ControlCenterMsg::NavNightLight => {
            (model.nav_cb)("appearance:night-light");
            true
        }
        _ => false,
    }
}
