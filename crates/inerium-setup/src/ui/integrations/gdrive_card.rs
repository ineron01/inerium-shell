use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Align, Box, Button, Entry, Label, Orientation, glib};
use vte4::prelude::*;
use vte4::{PtyFlags, Terminal};

use crate::i18n::{Lang, tr};
use crate::model::{FirstLoginStep, SetupPaths, mark_step_done};
use crate::ui::StatusCallback;
use super::RenderStepsCb;

#[allow(clippy::too_many_arguments)]
pub fn build_gdrive_card(
    step: &FirstLoginStep,
    paths: &SetupPaths,
    status_cb: &StatusCallback,
    lang: Lang,
    deferred_ids: &Rc<RefCell<HashSet<String>>>,
    render_steps_ref: &RenderStepsCb,
    active_step: &Rc<RefCell<Option<FirstLoginStep>>>,
    terminal: &Terminal,
    vte_box: &Box,
    vte_title: &Label,
) -> Box {
    // Expandable interactive wizard card for Google Drive / Cloud storage
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
        "Cloud storage · Optional (setup later by default)",
        "Облачное хранилище · Опционально (настроить позже по умолчанию)",
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

    let toggle_btn = Button::with_label(tr(
        lang,
        "Connect Cloud (Advanced) ▾",
        "Подключить облако (для продвинутых) ▾",
    ));
    toggle_btn.add_css_class("btn-secondary");
    toggle_btn.set_valign(Align::Center);
    top_row.append(&toggle_btn);

    let defer_btn = Button::with_label(tr(lang, "Later", "Отложить"));
    defer_btn.add_css_class("btn-secondary");
    defer_btn.set_valign(Align::Center);
    top_row.append(&defer_btn);
    card.append(&top_row);

    // Expandable guide & input form
    let guide_box = Box::new(Orientation::Vertical, 8);
    guide_box.set_visible(false);
    guide_box.set_margin_top(6);

    let guide_text = Label::new(Some(tr(
        lang,
        "ℹ Google Drive OAuth tokens expire after 7 days in testing mode. For a permanent token without re-login, click 'Publish App' (In Production) with scope 'drive.file' in Google Cloud Console. Alternatively, connect WebDAV (Yandex.Disk, Nextcloud) via 'rclone config'.",
        "ℹ OAuth-токен Google Drive в тестовом режиме истекает через 7 дней. Для постоянного токена без повторной авторизации переведите проект в «Publish App» (In Production) со скоупом drive.file в Google Cloud Console. (Либо настройте WebDAV / Яндекс.Диск через команду rclone config).",
    )));
    guide_text.add_css_class("card-muted");
    guide_text.set_wrap(true);
    guide_text.set_xalign(0.0);
    guide_box.append(&guide_text);

    let creds_row = Box::new(Orientation::Horizontal, 8);
    let id_entry = Entry::new();
    id_entry.add_css_class("setup-entry");
    id_entry.set_hexpand(true);
    id_entry.set_placeholder_text(Some(tr(lang, "Client ID (optional)", "Client ID (по желанию)")));
    creds_row.append(&id_entry);

    let secret_entry = Entry::new();
    secret_entry.add_css_class("setup-entry");
    secret_entry.set_hexpand(true);
    secret_entry.set_visibility(false);
    secret_entry.set_placeholder_text(Some(tr(lang, "Client Secret (optional)", "Client Secret (по желанию)")));
    creds_row.append(&secret_entry);
    guide_box.append(&creds_row);

    let action_row = Box::new(Orientation::Horizontal, 8);
    let auth_btn = Button::with_label(tr(lang, "󰌆 Authorize in Browser", "󰌆 Авторизоваться в браузере"));
    auth_btn.add_css_class("btn-primary");
    action_row.append(&auth_btn);

    let skip_btn = Button::with_label(tr(lang, "Skip (Don't ask again)", "Пропустить (больше не спрашивать)"));
    skip_btn.add_css_class("btn-secondary");
    action_row.append(&skip_btn);
    guide_box.append(&action_row);
    card.append(&guide_box);

    {
        let guide_box_c = guide_box.clone();
        let toggle_btn_c = toggle_btn.clone();
        toggle_btn.connect_clicked(move |_| {
            let now_vis = !guide_box_c.is_visible();
            guide_box_c.set_visible(now_vis);
            toggle_btn_c.set_label(if now_vis {
                tr(lang, "Hide Cloud Setup ▴", "Скрыть настройку ▴")
            } else {
                tr(lang, "Connect Cloud (Advanced) ▾", "Подключить облако (для продвинутых) ▾")
            });
        });
    }

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

    {
        let step_c = step.clone();
        let active_step = active_step.clone();
        let terminal = terminal.clone();
        let vte_box = vte_box.clone();
        let vte_title = vte_title.clone();
        let status_cb = status_cb.clone();
        let id_entry = id_entry.clone();
        let secret_entry = secret_entry.clone();

        auth_btn.connect_clicked(move |_| {
            if active_step.borrow().is_some() {
                status_cb.warn(tr(
                    lang,
                    "Wait for the current terminal step to finish",
                    "Дождитесь завершения текущего шага в терминале",
                ));
                return;
            }
            *active_step.borrow_mut() = Some(step_c.clone());
            vte_title.set_text(&match lang {
                Lang::En => format!("Running: {}", step_c.title(lang)),
                Lang::Ru => format!("Выполняется: {}", step_c.title(lang)),
            });
            vte_box.set_visible(true);
            terminal.reset(true, true);
            terminal.grab_focus();

            let step_str = step_c.path.to_string_lossy().to_string();
            let cid = id_entry.text().trim().to_string();
            let csec = secret_entry.text().trim().to_string();
            let mut env_strings: Vec<String> = std::env::vars()
                .filter(|(k, _)| k != "CLIENT_ID" && k != "CLIENT_SECRET")
                .map(|(k, v)| format!("{k}={v}"))
                .collect();
            env_strings.push(format!("CLIENT_ID={cid}"));
            env_strings.push(format!("CLIENT_SECRET={csec}"));
            let envv: Vec<&str> = env_strings.iter().map(|s| s.as_str()).collect();
            let status_local = status_cb.clone();
            let active_step_spawn = active_step.clone();

            terminal.spawn_async(
                PtyFlags::DEFAULT,
                None,
                &[&step_str],
                &envv,
                glib::SpawnFlags::SEARCH_PATH,
                || {},
                -1,
                Option::<&gio::Cancellable>::None,
                move |res| {
                    if let Err(e) = res {
                        *active_step_spawn.borrow_mut() = None;
                        status_local.error(&match lang {
                            Lang::En => format!("Failed to spawn terminal process: {e}"),
                            Lang::Ru => format!("Ошибка запуска процесса в терминале: {e}"),
                        });
                    }
                },
            );
        });
    }

    card
}
