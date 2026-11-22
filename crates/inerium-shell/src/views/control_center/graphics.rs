use std::process::Command;
use std::thread;

use inerium_i18n::{tr, tr_fmt};
use relm4::ComponentSender;

use super::{ControlCenterModel, ControlCenterMsg};
use crate::hw;

pub fn handle_graphics_msg(
    model: &mut ControlCenterModel,
    msg: &ControlCenterMsg,
    sender: &ComponentSender<ControlCenterModel>,
) -> bool {
    match msg {
        ControlCenterMsg::SwitchGpuMode(target) => {
            model.gpu_switching = true;
            model.gpu_status_text = tr_fmt!(
                "󰑐 Switching to {}: rebuilding initramfs...",
                "󰑐 Переключение на {}: пересборка initramfs...",
                target
            );
            let s = sender.clone();
            let tgt = *target;
            thread::spawn(move || {
                let ok = hw::switch_gpu_mode(tgt);
                s.input(ControlCenterMsg::GpuSwitchFinished(ok, tgt));
            });
            true
        }
        ControlCenterMsg::GpuSwitchFinished(ok, target) => {
            model.gpu_switching = false;
            if *ok {
                model.gpu_mode = target.to_string();
                model.gpu_status_ok = true;
                model.gpu_status_text = tr(
                    "󰄬 Ready. Mode will apply after reboot.",
                    "󰄬 Готово. Режим применится после перезагрузки.",
                )
                .to_string();
                model.show_reboot = true;
            } else {
                model.gpu_status_ok = false;
                model.gpu_status_text = tr(
                    "󰅙 Envycontrol switch error",
                    "󰅙 Ошибка переключения envycontrol",
                )
                .to_string();
            }
            true
        }
        ControlCenterMsg::Reboot => {
            let _ = Command::new("systemctl").arg("reboot").spawn();
            true
        }
        _ => false,
    }
}
