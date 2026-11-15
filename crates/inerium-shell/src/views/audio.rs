use std::rc::Rc;
use gtk4::prelude::*;
use gtk4::{Box, Button, Label, Orientation, Scale, ScrolledWindow};
use inerium_i18n::tr;

use crate::hw;

pub fn build_audio_view<FNav, FClose>(
    nav_cb: FNav,
    close_cb: FClose,
) -> (Box, i32, i32)
where
    FNav: Fn(&str) + 'static + Clone,
    FClose: Fn() + 'static + Clone,
{
    let root = Box::new(Orientation::Vertical, 10);
    root.add_css_class("popup-container");
    root.set_size_request(380, -1);

    // Header
    let head = Box::new(Orientation::Horizontal, 8);
    let back_btn = Button::with_label(tr("‹ Back", "‹ Назад"));
    back_btn.add_css_class("back-btn");
    let n_cb = nav_cb.clone();
    back_btn.connect_clicked(move |_| n_cb("control"));
    head.append(&back_btn);

    let title = Label::new(Some(tr("Audio Devices", "Аудиоустройства")));
    title.add_css_class("cc-host");
    title.set_hexpand(true);
    title.set_xalign(0.0);
    head.append(&title);

    let pavu_btn = Button::with_label(tr("󰕾 Mixer", "󰕾 Микшер"));
    pavu_btn.add_css_class("cc-chip");
    let c_cb = close_cb.clone();
    pavu_btn.connect_clicked(move |_| {
        c_cb();
        let _ = std::process::Command::new("uwsm")
            .args(["app", "--", "pavucontrol"])
            .spawn()
            .or_else(|_| std::process::Command::new("pavucontrol").spawn());
    });
    head.append(&pavu_btn);
    root.append(&head);

    // Volume Sliders Card (Output & Input)
    let sliders_card = Box::new(Orientation::Vertical, 8);
    sliders_card.add_css_class("cc-card");

    // Row 1: Output Volume
    let out_row = Box::new(Orientation::Horizontal, 8);
    let out_muted = hw::is_volume_muted();
    let out_ico = Button::with_label(if out_muted { "󰝟" } else { "󰕾" });
    out_ico.add_css_class("cc-slider-ico");
    if out_muted {
        out_ico.add_css_class("muted");
    }
    out_ico.set_tooltip_text(Some(tr("Mute Output", "Заглушить вывод")));
    let o_ico_c = out_ico.clone();
    out_ico.connect_clicked(move |_| {
        hw::toggle_volume_mute();
        let muted = hw::is_volume_muted();
        o_ico_c.set_label(if muted { "󰝟" } else { "󰕾" });
        if muted {
            o_ico_c.add_css_class("muted");
        } else {
            o_ico_c.remove_css_class("muted");
        }
    });
    out_row.append(&out_ico);

    let cur_vol = hw::get_volume();
    let vol_sc = Scale::with_range(Orientation::Horizontal, 0.0, 150.0, 1.0);
    vol_sc.add_css_class("cc-scale");
    vol_sc.set_hexpand(true);
    vol_sc.set_value(cur_vol as f64);
    out_row.append(&vol_sc);

    let vol_pct = Label::new(Some(&format!("{}%", cur_vol)));
    vol_pct.add_css_class("cc-pct");
    out_row.append(&vol_pct);

    let v_pct_c = vol_pct.clone();
    vol_sc.connect_value_changed(move |sc| {
        let val = sc.value() as u32;
        v_pct_c.set_label(&format!("{}%", val));
        hw::set_volume(val);
    });
    sliders_card.append(&out_row);

    // Row 2: Input / Mic Volume
    let in_row = Box::new(Orientation::Horizontal, 8);
    let mic_muted = hw::is_mic_muted();
    let mic_ico = Button::with_label(if mic_muted { "󰍭" } else { "󰍬" });
    mic_ico.add_css_class("cc-slider-ico");
    if mic_muted {
        mic_ico.add_css_class("muted");
    }
    mic_ico.set_tooltip_text(Some(tr("Mute Microphone", "Заглушить микрофон")));
    let m_ico_c = mic_ico.clone();
    mic_ico.connect_clicked(move |_| {
        hw::toggle_mic_mute();
        let muted = hw::is_mic_muted();
        m_ico_c.set_label(if muted { "󰍭" } else { "󰍬" });
        if muted {
            m_ico_c.add_css_class("muted");
        } else {
            m_ico_c.remove_css_class("muted");
        }
    });
    in_row.append(&mic_ico);

    let cur_mic_vol = hw::get_mic_volume();
    let mic_sc = Scale::with_range(Orientation::Horizontal, 0.0, 150.0, 1.0);
    mic_sc.add_css_class("cc-scale");
    mic_sc.set_hexpand(true);
    mic_sc.set_value(cur_mic_vol as f64);
    in_row.append(&mic_sc);

    let mic_pct = Label::new(Some(&format!("{}%", cur_mic_vol)));
    mic_pct.add_css_class("cc-pct");
    in_row.append(&mic_pct);

    let m_pct_c = mic_pct.clone();
    mic_sc.connect_value_changed(move |sc| {
        let val = sc.value() as u32;
        m_pct_c.set_label(&format!("{}%", val));
        hw::set_mic_volume(val);
    });
    sliders_card.append(&in_row);

    root.append(&sliders_card);

    // Devices List
    let scroll = ScrolledWindow::new();
    scroll.add_css_class("list-scroll");
    scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    scroll.set_propagate_natural_height(true);
    scroll.set_min_content_height(140);
    scroll.set_max_content_height(260);
    scroll.set_vexpand(true);

    let list_box = Box::new(Orientation::Vertical, 4);

    // Section: Output Devices
    let out_title = Label::new(Some(tr("OUTPUT DEVICES", "УСТРОЙСТВА ВЫВОДА")));
    out_title.add_css_class("cc-label");
    out_title.set_xalign(0.0);
    list_box.append(&out_title);

    let sinks = hw::list_audio_sinks();
    let sink_buttons = Rc::new(std::cell::RefCell::new(Vec::<(Button, Label)>::new()));

    for sink in sinks {
        let row = Button::new();
        row.add_css_class("card-item");
        if sink.is_default {
            row.add_css_class("active");
        }

        let inner = Box::new(Orientation::Horizontal, 8);
        let ico = Label::new(Some(if sink.name.to_lowercase().contains("headphone") || sink.name.to_lowercase().contains("buds") { "󰋋" } else { "󰕾" }));
        ico.add_css_class("card-icon");
        inner.append(&ico);

        let txt = Box::new(Orientation::Vertical, 1);
        txt.set_hexpand(true);
        let name_lbl = Label::new(Some(&sink.name));
        name_lbl.add_css_class("card-title");
        name_lbl.set_xalign(0.0);
        name_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        txt.append(&name_lbl);
        inner.append(&txt);

        let badge = Label::new(Some(if sink.is_default { tr("󰄬 Active", "󰄬 Активен") } else { "" }));
        badge.add_css_class("card-badge");
        inner.append(&badge);

        row.set_child(Some(&inner));

        let s_id = sink.id;
        let s_btns = sink_buttons.clone();
        let r_clone = row.clone();
        let b_clone = badge.clone();
        row.connect_clicked(move |_| {
            hw::set_default_sink(s_id);
            for (btn, bg) in s_btns.borrow().iter() {
                btn.remove_css_class("active");
                bg.set_label("");
            }
            r_clone.add_css_class("active");
            b_clone.set_label("󰄬 Активен");
        });

        sink_buttons.borrow_mut().push((row.clone(), badge));
        list_box.append(&row);
    }

    // Section: Input Devices
    let in_title = Label::new(Some(tr("INPUT DEVICES", "УСТРОЙСТВА ВВОДА")));
    in_title.add_css_class("cc-label");
    in_title.set_xalign(0.0);
    in_title.set_margin_top(8);
    list_box.append(&in_title);

    let sources = hw::list_audio_sources();
    let source_buttons = Rc::new(std::cell::RefCell::new(Vec::<(Button, Label)>::new()));

    for source in sources {
        let row = Button::new();
        row.add_css_class("card-item");
        if source.is_default {
            row.add_css_class("active");
        }

        let inner = Box::new(Orientation::Horizontal, 8);
        let ico = Label::new(Some("󰍬"));
        ico.add_css_class("card-icon");
        inner.append(&ico);

        let txt = Box::new(Orientation::Vertical, 1);
        txt.set_hexpand(true);
        let name_lbl = Label::new(Some(&source.name));
        name_lbl.add_css_class("card-title");
        name_lbl.set_xalign(0.0);
        name_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        txt.append(&name_lbl);
        inner.append(&txt);

        let badge = Label::new(Some(if source.is_default { tr("󰄬 Active", "󰄬 Активен") } else { "" }));
        badge.add_css_class("card-badge");
        inner.append(&badge);

        row.set_child(Some(&inner));

        let src_id = source.id;
        let src_btns = source_buttons.clone();
        let r_clone = row.clone();
        let b_clone = badge.clone();
        row.connect_clicked(move |_| {
            hw::set_default_source(src_id);
            for (btn, bg) in src_btns.borrow().iter() {
                btn.remove_css_class("active");
                bg.set_label("");
            }
            r_clone.add_css_class("active");
            b_clone.set_label("󰄬 Активен");
        });

        source_buttons.borrow_mut().push((row.clone(), badge));
        list_box.append(&row);
    }

    scroll.set_child(Some(&list_box));
    root.append(&scroll);

    (root, 380, 420)
}
