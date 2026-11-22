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
        _ => false,
    }
}
