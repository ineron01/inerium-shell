use std::cell::RefCell;
use std::collections::HashSet;
use std::process::Command;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Align, Box, Button, Label, Orientation, glib};
use vte4::prelude::*;
use vte4::{PtyFlags, Terminal};

use crate::i18n::{Lang, tr};
use crate::model::{FirstLoginStep, SetupPaths, StepMode, StepStatus, mark_step_done};
use crate::system::run_step_check;
use crate::ui::StatusCallback;
use super::RenderStepsCb;

#[allow(clippy::too_many_arguments)]
pub fn build_generic_step_card(
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
    let card = Box::new(Orientation::Horizontal, 12);
    card.add_css_class("setup-subcard");

    let info = Box::new(Orientation::Vertical, 2);
    info.set_hexpand(true);
    let title = Label::new(Some(step.title(lang)));
    title.add_css_class("card-title");
    title.set_xalign(0.0);
    title.set_wrap(true);
    info.append(&title);

    let meta = Label::new(Some(&step.id));
    meta.add_css_class("card-muted");
    meta.set_xalign(0.0);
    info.append(&meta);
    card.append(&info);

    let pill = match step.status {
        StepStatus::Configured => {
            let p = Label::new(Some(tr(lang, "Configured", "Настроено")));
            p.add_css_class("pill");
            p.add_css_class("pill-ok");
            p
        }
        StepStatus::Deferred => {
            let p = Label::new(Some(tr(lang, "Deferred", "Отложено")));
            p.add_css_class("pill");
            p.add_css_class("pill-deferred");
            p
        }
        StepStatus::Pending => {
            let p = Label::new(Some(tr(lang, "Pending", "Ожидает")));
            p.add_css_class("pill");
            p.add_css_class("pill-pending");
            p
        }
    };
    pill.set_valign(Align::Center);
    card.append(&pill);

    let run_lbl = if step.status == StepStatus::Configured {
        tr(lang, "Re-run", "Повторить")
    } else {
        tr(lang, "Run", "Запустить")
    };
    let run_btn = Button::with_label(run_lbl);
    run_btn.add_css_class("btn-primary");
    run_btn.set_valign(Align::Center);
    card.append(&run_btn);

    if step.status == StepStatus::Pending {
        let defer_btn = Button::with_label(tr(lang, "Later", "Отложить"));
        defer_btn.add_css_class("btn-secondary");
        defer_btn.set_valign(Align::Center);
        defer_btn.set_tooltip_text(Some(tr(
            lang,
            "Postpone for this session",
            "Напомнить при следующем входе",
        )));
        card.append(&defer_btn);

        let skip_btn = Button::with_label(tr(lang, "Skip", "Пропустить"));
        skip_btn.add_css_class("btn-secondary");
        skip_btn.set_valign(Align::Center);
        skip_btn.set_tooltip_text(Some(tr(
            lang,
            "Don't ask again (mark as done)",
            "Больше не спрашивать (отметить выполненным)",
        )));
        card.append(&skip_btn);

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

    // Wire Run button
    {
        let step_c = step.clone();
        let active_step = active_step.clone();
        let terminal = terminal.clone();
        let vte_box = vte_box.clone();
        let vte_title = vte_title.clone();
        let status_cb = status_cb.clone();
        let paths = paths.clone();
        let render_steps_c = render_steps_ref.clone();

        run_btn.connect_clicked(move |_| {
            if active_step.borrow().is_some() {
                status_cb.warn(tr(
                    lang,
                    "Wait for the current terminal step to finish",
                    "Дождитесь завершения текущего шага в терминале",
                ));
                return;
            }
            match step_c.mode {
                StepMode::Gui => {
                    let path = step_c.path.clone();
                    let step_id = step_c.id.clone();
                    let title = step_c.title(lang).to_string();
                    let state_dir = paths.first_login_state_dir();
                    let status_cb = status_cb.clone();
                    let render_steps_c = render_steps_c.clone();
                    status_cb.warn(&match lang {
                        Lang::En => format!("Running '{title}'..."),
                        Lang::Ru => format!("Запуск «{title}»..."),
                    });
                    glib::MainContext::default().spawn_local(async move {
                        let path_bg = path.clone();
                        let ok = gio::spawn_blocking(move || {
                            Command::new(&path_bg)
                                .status()
                                .map(|s| s.success())
                                .unwrap_or(false)
                                || run_step_check(&path_bg)
                        })
                        .await
                        .unwrap_or(false);
                        if ok {
                            let _ = mark_step_done(&state_dir, &step_id);
                            status_cb.ok(&match lang {
                                Lang::En => format!("Step '{title}' completed"),
                                Lang::Ru => format!("Шаг «{title}» завершён"),
                            });
                        } else {
                            status_cb.warn(&match lang {
                                Lang::En => format!("Step '{title}' is not configured yet"),
                                Lang::Ru => format!("Шаг «{title}» ещё не настроен"),
                            });
                        }
                        if let Some(ref f) = *render_steps_c.borrow() {
                            f(true);
                        }
                    });
                }
                StepMode::Terminal => {
                    *active_step.borrow_mut() = Some(step_c.clone());
                    vte_title.set_text(&match lang {
                        Lang::En => format!("Running: {}", step_c.title(lang)),
                        Lang::Ru => format!("Выполняется: {}", step_c.title(lang)),
                    });
                    vte_box.set_visible(true);
                    terminal.reset(true, true);
                    terminal.grab_focus();

                    let step_str = step_c.path.to_string_lossy().to_string();
                    let active_step_err = active_step.clone();
                    let status_cb_err = status_cb.clone();
                    let step_title_err = step_c.title(lang).to_string();
                    let step_path_fallback = step_c.path.clone();

                    terminal.spawn_async(
                        PtyFlags::DEFAULT,
                        None,
                        &[&step_str],
                        &[],
                        glib::SpawnFlags::DEFAULT,
                        || {},
                        -1,
                        gio::Cancellable::NONE,
                        move |res| {
                            if let Err(e) = res {
                                *active_step_err.borrow_mut() = None;
                                status_cb_err.warn(&match lang {
                                    Lang::En => format!("VTE error ({e}), opening kitty..."),
                                    Lang::Ru => format!("VTE ошибка ({e}), открываю kitty..."),
                                });
                                let _ = Command::new("kitty")
                                    .args([
                                        "--class",
                                        "inerium-first-login",
                                        "--title",
                                        &step_title_err,
                                    ])
                                    .arg(&step_path_fallback)
                                    .spawn();
                            }
                        },
                    );
                }
            }
        });
    }

    card
}
