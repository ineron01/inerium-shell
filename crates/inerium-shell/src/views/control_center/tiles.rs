use inerium_i18n::tr;

use super::{ControlCenterModel, ControlCenterMsg};
use crate::hw;

pub fn get_wifi_sub(wifi_on: bool) -> String {
    if wifi_on {
        if hw::is_hotspot_active() {
            tr("Hotspot", "Точка доступа").to_string()
        } else if let Some(ref ssid) = hw::get_active_wifi_ssid() {
            ssid.clone()
        } else {
            tr("Enabled", "Включен").to_string()
        }
    } else {
        tr("Disabled", "Выключен").to_string()
    }
}

pub fn handle_tiles_msg(model: &mut ControlCenterModel, msg: &ControlCenterMsg) -> bool {
    match msg {
        ControlCenterMsg::ToggleWifi => {
            model.is_wifi = !model.is_wifi;
            model.wifi_sub = get_wifi_sub(model.is_wifi);
            hw::toggle_wifi(model.is_wifi);
            true
        }
        ControlCenterMsg::ToggleBt => {
            model.is_bt = !model.is_bt;
            hw::toggle_bluetooth(model.is_bt);
            true
        }
        ControlCenterMsg::ToggleDnd => {
            model.is_dnd = !model.is_dnd;
            hw::toggle_dnd();
            true
        }
        ControlCenterMsg::ToggleMic => {
            model.is_mic_muted = !model.is_mic_muted;
            hw::toggle_mic_mute();
            true
        }
        ControlCenterMsg::NavNetwork => {
            (model.nav_cb)("network");
            true
        }
        ControlCenterMsg::NavBluetooth => {
            (model.nav_cb)("bluetooth");
            true
        }
        ControlCenterMsg::NavAudio => {
            (model.nav_cb)("audio");
            true
        }
        _ => false,
    }
}
