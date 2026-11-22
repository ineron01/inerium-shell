pub mod gdrive_card;
pub mod novpn_card;
pub mod step_card;

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    Box, Button, Label, Orientation, PolicyType, ScrolledWindow, pango,
};
use vte4::prelude::*;
use vte4::Terminal;


use crate::i18n::{Lang, tr};
use crate::model::{
    FirstLoginStep, SetupPaths, StepStatus, mark_step_done,
};
use crate::system::{evaluate_steps, run_step_check};
use crate::ui::StatusCallback;

pub type RenderStepsCb = Rc<RefCell<Option<Rc<dyn Fn(bool)>>>>;

pub fn build_integrations_tab<FPending>(
    paths: SetupPaths,
    initial_steps: Vec<FirstLoginStep>,
    status_cb: StatusCallback,
    lang: Lang,
    on_pending_changed: FPending,
) -> Box
where
    FPending: Fn(usize) + 'static + Clone,
{
    let root = Box::new(Orientation::Vertical, 10);

    // Header card
    let head_card = Box::new(Orientation::Horizontal, 10);
    head_card.add_css_class("setup-card");

    let head_info = Box::new(Orientation::Vertical, 2);
    head_info.set_hexpand(true);
    let head_title = Label::new(Some(tr(
        lang,
        "Accounts & external integrations",
        "Аккаунты и внешние интеграции",
    )));
    head_title.add_css_class("card-title");
    head_title.set_xalign(0.0);
    head_info.append(&head_title);

    let head_sub = Label::new(Some(tr(
        lang,
        "Executable steps from ~/.config/inerium/first-login.d/ (checked via --check)",
        "Исполняемые шаги из ~/.config/inerium/first-login.d/ (проверка через --check)",
    )));
    head_sub.add_css_class("card-muted");
    head_sub.set_xalign(0.0);
    head_info.append(&head_sub);
    head_card.append(&head_info);

    let recheck_btn = Button::with_label(tr(lang, "󰑐 Recheck", "󰑐 Проверить снова"));
    recheck_btn.add_css_class("btn-secondary");
    head_card.append(&recheck_btn);
    root.append(&head_card);

    // Steps list scroll
    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .build();
    let list_box = Box::new(Orientation::Vertical, 8);
    scroll.set_child(Some(&list_box));
    root.append(&scroll);

    // Collapsible Embedded VTE-4 Terminal Drawer
    let vte_box = Box::new(Orientation::Vertical, 6);
    vte_box.add_css_class("vte-frame");
    vte_box.set_visible(false);

    let vte_head = Box::new(Orientation::Horizontal, 8);
    let vte_title = Label::new(Some(tr(lang, "Embedded terminal", "Встроенный терминал")));
    vte_title.add_css_class("card-meta");
    vte_title.set_hexpand(true);
    vte_title.set_xalign(0.0);
    vte_head.append(&vte_title);

    let vte_close_btn = Button::with_label(tr(lang, "Hide", "Скрыть"));
    vte_close_btn.add_css_class("btn-secondary");
    vte_head.append(&vte_close_btn);
    vte_box.append(&vte_head);

    let terminal = Terminal::new();
    terminal.set_hexpand(true);
    terminal.set_vexpand(false);
    terminal.set_size_request(-1, 220);
    terminal.set_scrollback_lines(2000);
    let font_desc = pango::FontDescription::from_string("JetBrainsMono Nerd Font 11");
    terminal.set_font(Some(&font_desc));
    crate::theme::update_terminal_palette(&terminal);

    {
        let terminal_weak = terminal.downgrade();
        crate::theme::add_theme_listener(move || {
            if let Some(term) = terminal_weak.upgrade() {
                crate::theme::update_terminal_palette(&term);
                true
            } else {
                false
            }
        });
    }

    let vte_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .build();
    vte_scroll.set_size_request(-1, 220);
    vte_scroll.set_child(Some(&terminal));
    vte_box.append(&vte_scroll);
    root.append(&vte_box);

    {
        let vte_box_c = vte_box.clone();
        vte_close_btn.connect_clicked(move |_| vte_box_c.set_visible(false));
    }

    let deferred_ids: Rc<RefCell<HashSet<String>>> = Rc::new(RefCell::new(HashSet::new()));
    let active_step: Rc<RefCell<Option<FirstLoginStep>>> = Rc::new(RefCell::new(None));
    let cached_steps: Rc<RefCell<Vec<FirstLoginStep>>> = Rc::new(RefCell::new(initial_steps));

    let render_steps: RenderStepsCb = Rc::new(RefCell::new(None));

    // Handle VTE child exit once
    {
        let active_step = active_step.clone();
        let paths = paths.clone();
        let status_cb = status_cb.clone();
        let render_steps = render_steps.clone();
        let vte_title = vte_title.clone();

        terminal.connect_child_exited(move |_, wait_status| {
            let Some(step) = active_step.borrow_mut().take() else {
                return;
            };
            let exited_zero = wait_status == 0;
            let exit_code = if wait_status >= 0 && (wait_status & 0x7f) == 0 {
                (wait_status >> 8) & 0xff
            } else {
                wait_status
            };
            let checked_ok = run_step_check(&step.path);
            let s_title = step.title(lang);
            if exited_zero || checked_ok {
                let _ = mark_step_done(&paths.first_login_state_dir(), &step.id);
                vte_title.set_text(&match lang {
                    Lang::En => format!("Completed: {}", s_title),
                    Lang::Ru => format!("Завершено: {}", s_title),
                });
                status_cb.ok(&match lang {
                    Lang::En => format!("Step '{}' completed", s_title),
                    Lang::Ru => format!("Шаг «{}» завершён", s_title),
                });
            } else {
                vte_title.set_text(&match lang {
                    Lang::En => format!("Incomplete: {}", s_title),
                    Lang::Ru => format!("Не завершено: {}", s_title),
                });
                status_cb.warn(&match lang {
                    Lang::En => format!(
                        "Step '{}' did not finish (code {}) — you can retry",
                        s_title, exit_code
                    ),
                    Lang::Ru => format!(
                        "Шаг «{}» не завершён (код {}) — можно повторить",
                        s_title, exit_code
                    ),
                });
            }
            if let Some(ref f) = *render_steps.borrow() {
                f(true);
            }
        });
    }

    let render_fn: Rc<dyn Fn(bool)> = {
        let list_box = list_box.clone();
        let cached_steps = cached_steps.clone();
        let deferred_ids = deferred_ids.clone();
        let active_step = active_step.clone();
        let paths = paths.clone();
        let on_pending_changed = on_pending_changed.clone();
        let terminal = terminal.clone();
        let vte_box = vte_box.clone();
        let vte_title = vte_title.clone();
        let render_steps_ref = render_steps.clone();
        let status_cb = status_cb.clone();

        Rc::new(move |rescan: bool| {
            if rescan {
                *cached_steps.borrow_mut() = evaluate_steps(&paths);
            }
            while let Some(child) = list_box.first_child() {
                list_box.remove(&child);
            }

            let mut steps = cached_steps.borrow().clone();
            let deferred = deferred_ids.borrow();
            let mut pending_count = 0usize;

            for step in &mut steps {
                if step.status == StepStatus::Pending && deferred.contains(&step.id) {
                    step.status = StepStatus::Deferred;
                }
                let is_optional = step.id.contains("gdrive") || step.id.contains("rclone") || step.id.contains("novpn");
                // Cloud storage and noVPN are optional by default; don't count towards blocking pending
                if step.status == StepStatus::Pending && !is_optional {
                    pending_count += 1;
                }
            }
            on_pending_changed(pending_count);

            if steps.is_empty() {
                let empty_card = Box::new(Orientation::Vertical, 6);
                empty_card.add_css_class("setup-subcard");
                let lbl = Label::new(Some(tr(
                    lang,
                    "No extra integration steps",
                    "Дополнительных шагов интеграции нет",
                )));
                lbl.add_css_class("card-title");
                lbl.set_xalign(0.0);
                empty_card.append(&lbl);

                let sub = Label::new(Some(tr(
                    lang,
                    "No executable scripts found in ~/.config/inerium/first-login.d/.",
                    "В ~/.config/inerium/first-login.d/ не найдено исполняемых скриптов.",
                )));
                sub.add_css_class("card-muted");
                sub.set_xalign(0.0);
                empty_card.append(&sub);
                list_box.append(&empty_card);
                return;
            }

            for step in steps {
                let is_gdrive = step.id.contains("gdrive") || step.id.contains("rclone");
                if is_gdrive && step.status != StepStatus::Configured {
                    let card = gdrive_card::build_gdrive_card(
                        &step,
                        &paths,
                        &status_cb,
                        lang,
                        &deferred_ids,
                        &render_steps_ref,
                        &active_step,
                        &terminal,
                        &vte_box,
                        &vte_title,
                    );
                    list_box.append(&card);
                    continue;
                }

                let is_novpn = step.id.contains("novpn");
                if is_novpn && step.status != StepStatus::Configured {
                    let card = novpn_card::build_novpn_card(
                        &step,
                        &paths,
                        &status_cb,
                        lang,
                        &deferred_ids,
                        &render_steps_ref,
                    );
                    list_box.append(&card);
                    continue;
                }

                let card = step_card::build_generic_step_card(
                    &step,
                    &paths,
                    &status_cb,
                    lang,
                    &deferred_ids,
                    &render_steps_ref,
                    &active_step,
                    &terminal,
                    &vte_box,
                    &vte_title,
                );
                list_box.append(&card);
            }
        })
    };

    *render_steps.borrow_mut() = Some(render_fn.clone());
    render_fn(false);

    {
        let render_fn = render_fn.clone();
        let status_cb = status_cb.clone();
        recheck_btn.connect_clicked(move |_| {
            render_fn(true);
            status_cb.ok(tr(lang, "Step status refreshed", "Статус шагов обновлён"));
        });
    }

    root
}
