use std::cell::RefCell;
use std::collections::HashSet;
use std::process::Command;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Align, Box, Button, DropDown, Label, Orientation, StringList, glib};

use crate::i18n::{Lang, tr};
use crate::model::{FirstLoginStep, SetupPaths, mark_step_done};
use crate::ui::StatusCallback;
use super::RenderStepsCb;

#[derive(serde::Deserialize, Clone)]
pub struct BrowserItem {
    pub id: String,
    pub name: String,
}

pub fn detect_installed_browsers() -> Vec<BrowserItem> {
    for helper in ["novpn-desktop", "/usr/local/bin/novpn-desktop"] {
        if let Ok(out) = Command::new(helper).arg("--json").output()
            && out.status.success()
            && let Ok(list) = serde_json::from_slice::<Vec<BrowserItem>>(&out.stdout)
            && !list.is_empty()
        {
            return list;
        }
    }
    Vec::new()
}

pub fn build_novpn_card(
    step: &FirstLoginStep,
    paths: &SetupPaths,
    status_cb: &StatusCallback,
    lang: Lang,
    deferred_ids: &Rc<RefCell<HashSet<String>>>,
    render_steps_ref: &RenderStepsCb,
) -> Box {
    // Interactive card for noVPN browser profile generator
    let card = Box::new(Orientation::Vertical, 8);
    card.add_css_class("setup-subcard");

    let top_row = Box::new(Orientation::Horizontal, 10);
    let info = Box::new(Orientation::Vertical, 2);
    info.set_hexpand(true);
    let title = Label::new(Some(step.title(lang)));
    title.add_css_class("card-title");
    title.set_xalign(0.0);
    title.set_wrap(true);
    info.append(&title);

    let meta = Label::new(Some(tr(
        lang,
        "Isolated browser profile bypassing VPN for direct ISP traffic · Optional",
        "Изолированный профиль браузера для доступа к сайтам РФ/банков в обход VPN · Опционально",
    )));
    meta.add_css_class("card-muted");
    meta.set_xalign(0.0);
    info.append(&meta);
    top_row.append(&info);

    let pill = Label::new(Some(tr(lang, "Optional", "Опционально")));
    pill.add_css_class("pill");
    pill.add_css_class("pill-deferred");
    pill.set_valign(Align::Center);
    top_row.append(&pill);

    let defer_btn = Button::with_label(tr(lang, "Later", "Отложить"));
    defer_btn.add_css_class("btn-secondary");
    defer_btn.set_valign(Align::Center);
    top_row.append(&defer_btn);
    card.append(&top_row);

    let form_box = Box::new(Orientation::Vertical, 8);
    form_box.set_margin_top(4);

    let detected = detect_installed_browsers();
    if detected.is_empty() {
        let warn_lbl = Label::new(Some(tr(
            lang,
            "No web browsers detected in PATH. Install a browser (Chrome, Firefox, Brave, etc.) first, then create the direct profile.",
            "В системе не обнаружен ни один браузер. Сначала установите браузер (Chrome, Firefox, Brave и др.), затем создайте профиль.",
        )));
        warn_lbl.add_css_class("card-muted");
        warn_lbl.set_wrap(true);
        warn_lbl.set_xalign(0.0);
        form_box.append(&warn_lbl);
    } else {
        let action_row = Box::new(Orientation::Horizontal, 8);
        action_row.set_valign(Align::Center);

        let lbl = Label::new(Some(tr(lang, "Browser:", "Браузер:")));
        lbl.add_css_class("card-muted");
        action_row.append(&lbl);

        let dd_labels: Vec<String> = detected.iter().map(|b| format!("{} ({})", b.name, b.id)).collect();
        let dd_items: Vec<&str> = dd_labels.iter().map(|s| s.as_str()).collect();
        let browser_dd = DropDown::new(Some(StringList::new(&dd_items)), gtk4::Expression::NONE);
        browser_dd.set_hexpand(true);
        action_row.append(&browser_dd);

        let create_btn = Button::with_label(tr(lang, "󰌷 Create Shortcut", "󰌷 Создать ярлык «Без VPN»"));
        create_btn.add_css_class("btn-primary");
        action_row.append(&create_btn);

        let detected_c = detected.clone();
        let status_cb_c = status_cb.clone();
        let render_steps_c = render_steps_ref.clone();
        create_btn.connect_clicked(move |_| {
            let idx = browser_dd.selected() as usize;
            if let Some(target_b) = detected_c.get(idx) {
                let target = target_b.id.clone();
                let status_cb = status_cb_c.clone();
                let render_steps = render_steps_c.clone();
                glib::MainContext::default().spawn_local(async move {
                    let res = gio::spawn_blocking(move || {
                        Command::new("novpn-desktop")
                            .arg(&target)
                            .status()
                    }).await;
                    match res {
                        Ok(Ok(st)) if st.success() => {
                            status_cb.ok(tr(
                                lang,
                                "Direct shortcut created successfully in application menu!",
                                "Ярлык прямого доступа успешно создан в меню приложений!",
                            ));
                            if let Some(ref f) = *render_steps.borrow() {
                                f(true);
                            }
                        }
                        _ => {
                            status_cb.error(tr(
                                lang,
                                "Failed to generate shortcut. Try 'novpn make-desktop' in terminal.",
                                "Ошибка при создании ярлыка. Попробуйте 'novpn make-desktop' в терминале.",
                            ));
                        }
                    }
                });
            }
        });

        form_box.append(&action_row);
    }

    let skip_row = Box::new(Orientation::Horizontal, 8);
    let skip_spacer = Box::new(Orientation::Horizontal, 0);
    skip_spacer.set_hexpand(true);
    skip_row.append(&skip_spacer);

    let skip_btn = Button::with_label(tr(lang, "Skip (Don't ask again)", "Пропустить (больше не спрашивать)"));
    skip_btn.add_css_class("btn-secondary");
    skip_row.append(&skip_btn);
    form_box.append(&skip_row);
    card.append(&form_box);

    {
        let deferred_ids_c = deferred_ids.clone();
        let render_steps_c = render_steps_ref.clone();
        let status_cb_c = status_cb.clone();
        let step_id = step.id.clone();
        let step_title = step.title(lang).to_string();
        defer_btn.connect_clicked(move |_| {
            deferred_ids_c.borrow_mut().insert(step_id.clone());
            status_cb_c.warn(&match lang {
                Lang::En => format!("Step '{step_title}' deferred"),
                Lang::Ru => format!("Шаг «{step_title}» отложен до следующего входа"),
            });
            if let Some(ref f) = *render_steps_c.borrow() {
                f(false);
            }
        });
    }

    {
        let paths_c = paths.clone();
        let render_steps_c = render_steps_ref.clone();
        let status_cb_c = status_cb.clone();
        let step_id = step.id.clone();
        let step_title = step.title(lang).to_string();
        skip_btn.connect_clicked(move |_| {
            if let Err(e) = mark_step_done(&paths_c.first_login_state_dir(), &step_id) {
                status_cb_c.error(&e);
                return;
            }
            status_cb_c.ok(&match lang {
                Lang::En => format!("Step '{step_title}' skipped"),
                Lang::Ru => format!("Шаг «{step_title}» пропущен"),
            });
            if let Some(ref f) = *render_steps_c.borrow() {
                f(true);
            }
        });
    }

    card
}
