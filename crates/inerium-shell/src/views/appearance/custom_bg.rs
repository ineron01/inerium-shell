use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, DrawingArea, Entry, Label, Orientation, Scale};
use inerium_i18n::tr;
use relm4::ComponentSender;

use super::theme_picker::{format_hex_color, make_color_swatch_static, parse_hex_color};
use super::{AppearanceModel, AppearanceMsg};
use crate::bg_generator::{BgMode, GradientDirection};

#[derive(Clone, Debug)]
pub struct CustomBgState {
    pub mode: BgMode,
    pub color_start: (u8, u8, u8),
    pub color_end: (u8, u8, u8),
    pub direction: GradientDirection,
}

type ColorRgb = (u8, u8, u8);
type GradPresetItem = ((ColorRgb, ColorRgb), gtk4::glib::WeakRef<Button>);
type SolidPresetItem = (ColorRgb, gtk4::glib::WeakRef<Button>);
type DirButtonItem = (GradientDirection, gtk4::glib::WeakRef<Button>);

#[derive(Default)]
pub struct CustomBgWidgets {
    pub bg_controls_mode: Option<BgMode>,
    pub preview_area: gtk4::glib::WeakRef<DrawingArea>,
    pub swatch_start: gtk4::glib::WeakRef<DrawingArea>,
    pub swatch_end: gtk4::glib::WeakRef<DrawingArea>,
    pub entry_start: gtk4::glib::WeakRef<Entry>,
    pub entry_end: gtk4::glib::WeakRef<Entry>,
    pub r_scale: gtk4::glib::WeakRef<Scale>,
    pub g_scale: gtk4::glib::WeakRef<Scale>,
    pub b_scale: gtk4::glib::WeakRef<Scale>,
    pub solid_presets: Vec<SolidPresetItem>,
    pub grad_presets: Vec<GradPresetItem>,
    pub dir_buttons: Vec<DirButtonItem>,
}

impl CustomBgWidgets {
    pub fn clear(&mut self) {
        self.bg_controls_mode = None;
        self.preview_area = gtk4::glib::WeakRef::new();
        self.swatch_start = gtk4::glib::WeakRef::new();
        self.swatch_end = gtk4::glib::WeakRef::new();
        self.entry_start = gtk4::glib::WeakRef::new();
        self.entry_end = gtk4::glib::WeakRef::new();
        self.r_scale = gtk4::glib::WeakRef::new();
        self.g_scale = gtk4::glib::WeakRef::new();
        self.b_scale = gtk4::glib::WeakRef::new();
        self.solid_presets.clear();
        self.grad_presets.clear();
        self.dir_buttons.clear();
    }

    pub fn update_active_presets_and_dirs(
        &self,
        mode: BgMode,
        c_start: (u8, u8, u8),
        c_end: (u8, u8, u8),
        dir: GradientDirection,
    ) {
        match mode {
            BgMode::Solid => {
                for (col, btn_weak) in &self.solid_presets {
                    if let Some(btn) = btn_weak.upgrade() {
                        if *col == c_start {
                            btn.add_css_class("active");
                        } else {
                            btn.remove_css_class("active");
                        }
                    }
                }
            }
            BgMode::Gradient => {
                for ((start, end), btn_weak) in &self.grad_presets {
                    if let Some(btn) = btn_weak.upgrade() {
                        if *start == c_start && *end == c_end {
                            btn.add_css_class("active");
                        } else {
                            btn.remove_css_class("active");
                        }
                    }
                }
                for (d, btn_weak) in &self.dir_buttons {
                    if let Some(btn) = btn_weak.upgrade() {
                        if *d == dir {
                            btn.add_css_class("active");
                        } else {
                            btn.remove_css_class("active");
                        }
                    }
                }
            }
        }
    }
}

pub fn populate_custom_bg_controls(
    container: &GtkBox,
    model: &AppearanceModel,
    sender: &ComponentSender<AppearanceModel>,
) {
    // Only rebuild if the mode changed or if first run
    {
        let w = model.bg_widgets.borrow();
        if w.bg_controls_mode == Some(model.custom_bg_mode) && container.first_child().is_some() {
            return;
        }
    }
    model.bg_widgets.borrow_mut().clear();
    model.bg_widgets.borrow_mut().bg_controls_mode = Some(model.custom_bg_mode);

    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    // --- Header Row ---
    let header_row = GtkBox::new(Orientation::Horizontal, 6);
    let title_lbl = Label::new(Some(tr("󰸌  Background Studio", "󰸌  Свой фон (генератор)")));
    title_lbl.add_css_class("cc-label");
    title_lbl.set_hexpand(true);
    title_lbl.set_xalign(0.0);
    header_row.append(&title_lbl);

    let btn_close = Button::with_label("󰅖");
    btn_close.add_css_class("wall-del-btn");
    btn_close.set_tooltip_text(Some(tr("Close", "Закрыть")));
    let s = sender.clone();
    btn_close.connect_clicked(move |_| {
        s.input(AppearanceMsg::ToggleCustomBg);
    });
    header_row.append(&btn_close);
    container.append(&header_row);

    // --- Live Cairo Background Preview Area ---
    let preview = DrawingArea::new();
    preview.set_content_height(75);
    preview.set_hexpand(true);
    preview.add_css_class("bg-preview-area");

    let st_preview = model.bg_state.clone();
    preview.set_draw_func(move |_area, cr, w, h| {
        let w = w as f64;
        let h = h as f64;
        let r = 8.0;

        cr.new_sub_path();
        cr.arc(w - r, r, r, -std::f64::consts::FRAC_PI_2, 0.0);
        cr.arc(w - r, h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
        cr.arc(r, h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
        cr.arc(r, r, r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
        cr.close_path();
        cr.clip();

        let st = st_preview.borrow();
        match st.mode {
            BgMode::Solid => {
                let c = st.color_start;
                cr.set_source_rgb(
                    c.0 as f64 / 255.0,
                    c.1 as f64 / 255.0,
                    c.2 as f64 / 255.0,
                );
                let _ = cr.paint();
            }
            BgMode::Gradient => {
                let c_start = st.color_start;
                let c_end = st.color_end;
                match st.direction {
                    GradientDirection::Vertical | GradientDirection::Horizontal | GradientDirection::Diagonal => {
                        let (x0, y0, x1, y1) = match st.direction {
                            GradientDirection::Vertical => (0.0, 0.0, 0.0, h),
                            GradientDirection::Horizontal => (0.0, 0.0, w, 0.0),
                            _ => (0.0, 0.0, w, h),
                        };
                        let pat = gtk4::cairo::LinearGradient::new(x0, y0, x1, y1);
                        pat.add_color_stop_rgb(
                            0.0,
                            c_start.0 as f64 / 255.0,
                            c_start.1 as f64 / 255.0,
                            c_start.2 as f64 / 255.0,
                        );
                        pat.add_color_stop_rgb(
                            1.0,
                            c_end.0 as f64 / 255.0,
                            c_end.1 as f64 / 255.0,
                            c_end.2 as f64 / 255.0,
                        );
                        let _ = cr.set_source(&pat);
                        let _ = cr.paint();
                    }
                    GradientDirection::Radial => {
                        let rad = (w * w + h * h).sqrt() / 2.0;
                        let pat = gtk4::cairo::RadialGradient::new(w / 2.0, h / 2.0, 0.0, w / 2.0, h / 2.0, rad);
                        pat.add_color_stop_rgb(
                            0.0,
                            c_start.0 as f64 / 255.0,
                            c_start.1 as f64 / 255.0,
                            c_start.2 as f64 / 255.0,
                        );
                        pat.add_color_stop_rgb(
                            1.0,
                            c_end.0 as f64 / 255.0,
                            c_end.1 as f64 / 255.0,
                            c_end.2 as f64 / 255.0,
                        );
                        let _ = cr.set_source(&pat);
                        let _ = cr.paint();
                    }
                }
            }
        }
    });
    container.append(&preview);
    model.bg_widgets.borrow_mut().preview_area = preview.downgrade();

    // --- Mode Selector Row ---
    let mode_seg = GtkBox::new(Orientation::Horizontal, 4);
    mode_seg.add_css_class("cc-seg");

    let btn_solid = Button::with_label(tr("󰔏  Solid Color", "󰔏  Сплошной"));
    btn_solid.set_hexpand(true);
    if model.custom_bg_mode == BgMode::Solid {
        btn_solid.add_css_class("active");
    }
    let s = sender.clone();
    btn_solid.connect_clicked(move |_| {
        s.input(AppearanceMsg::SetBgMode(BgMode::Solid));
    });
    mode_seg.append(&btn_solid);

    let btn_gradient = Button::with_label(tr("󰔎  Gradient", "󰔎  Градиент"));
    btn_gradient.set_hexpand(true);
    if model.custom_bg_mode == BgMode::Gradient {
        btn_gradient.add_css_class("active");
    }
    let s = sender.clone();
    btn_gradient.connect_clicked(move |_| {
        s.input(AppearanceMsg::SetBgMode(BgMode::Gradient));
    });
    mode_seg.append(&btn_gradient);
    container.append(&mode_seg);

    // --- Inline Color Controls ---
    match model.custom_bg_mode {
        BgMode::Solid => {
            // Presets palette
            let presets_box = GtkBox::new(Orientation::Vertical, 4);
            let lbl_p = Label::new(Some(tr("Presets:", "Пресеты:")));
            lbl_p.add_css_class("cc-label");
            lbl_p.set_xalign(0.0);
            presets_box.append(&lbl_p);

            let dots_row = GtkBox::new(Orientation::Horizontal, 6);
            dots_row.set_halign(gtk4::Align::Center);
            let presets = [
                ("Sumi", (0x18, 0x16, 0x16)),
                ("Obsidian", (0x0d, 0x0e, 0x11)),
                ("Slate", (0x1e, 0x20, 0x24)),
                ("Navy", (0x0f, 0x17, 0x2a)),
                ("Forest", (0x0d, 0x22, 0x18)),
                ("Wine", (0x23, 0x12, 0x26)),
                ("Earth", (0x26, 0x1d, 0x1a)),
                ("Lotus", (0xf2, 0xec, 0xbc)),
                ("White", (0xff, 0xff, 0xff)),
                ("Black", (0x00, 0x00, 0x00)),
            ];

            for (tip, col) in presets {
                let dot_btn = Button::new();
                dot_btn.add_css_class("color-preset-btn");
                if model.custom_color_start == col {
                    dot_btn.add_css_class("active");
                }
                dot_btn.set_tooltip_text(Some(tip));
                let swatch = make_color_swatch_static(col.0, col.1, col.2, 22, 22, 11.0);
                dot_btn.set_child(Some(&swatch));
                let s = sender.clone();
                dot_btn.connect_clicked(move |_| {
                    s.input(AppearanceMsg::SetBgColorStart(col));
                });
                model.bg_widgets.borrow_mut().solid_presets.push((col, dot_btn.downgrade()));
                dots_row.append(&dot_btn);
            }
            presets_box.append(&dots_row);
            container.append(&presets_box);

            // Hex Input and Direct Swatch Row
            let hex_row = GtkBox::new(Orientation::Horizontal, 8);
            let lbl_hex = Label::new(Some(tr("Custom Color:", "Свой цвет:")));
            lbl_hex.add_css_class("cc-label");
            hex_row.append(&lbl_hex);

            let cur_col = model.custom_color_start;
            let swatch_start = DrawingArea::new();
            swatch_start.set_content_width(28);
            swatch_start.set_content_height(28);
            swatch_start.add_css_class("color-swatch-box");
            let st_swatch = model.bg_state.clone();
            swatch_start.set_draw_func(move |_area, cr, w, h| {
                let c = st_swatch.borrow().color_start;
                let w = w as f64;
                let h = h as f64;
                let r = 6.0;
                cr.new_sub_path();
                cr.arc(w - r, r, r, -std::f64::consts::FRAC_PI_2, 0.0);
                cr.arc(w - r, h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
                cr.arc(r, h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
                cr.arc(r, r, r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
                cr.close_path();
                cr.clip();
                cr.set_source_rgb(c.0 as f64 / 255.0, c.1 as f64 / 255.0, c.2 as f64 / 255.0);
                let _ = cr.paint();
            });
            hex_row.append(&swatch_start);
            model.bg_widgets.borrow_mut().swatch_start = swatch_start.downgrade();

            let entry = Entry::new();
            entry.add_css_class("hex-input");
            entry.set_text(&format_hex_color(cur_col));
            entry.set_max_length(7);
            entry.set_width_chars(8);
            let s = sender.clone();
            let sync = model.syncing_bg.clone();
            entry.connect_changed(move |e| {
                if sync.get() {
                    return;
                }
                let text = e.text();
                if let Some(c) = parse_hex_color(&text) {
                    s.input(AppearanceMsg::SetBgColorStart(c));
                }
            });
            hex_row.append(&entry);
            model.bg_widgets.borrow_mut().entry_start = entry.downgrade();

            // RGB adjustment sliders
            let rgb_box = GtkBox::new(Orientation::Vertical, 2);
            rgb_box.set_hexpand(true);

            let r_scale = Scale::with_range(Orientation::Horizontal, 0.0, 255.0, 1.0);
            r_scale.add_css_class("cc-scale");
            r_scale.set_value(cur_col.0 as f64);
            r_scale.set_hexpand(true);
            r_scale.set_draw_value(false);
            let s_r = sender.clone();
            let st_r = model.bg_state.clone();
            let sync_r = model.syncing_bg.clone();
            r_scale.connect_value_changed(move |sc| {
                if sync_r.get() {
                    return;
                }
                let v = sc.value().round() as u8;
                let cur = st_r.borrow().color_start;
                if cur.0 != v {
                    s_r.input(AppearanceMsg::SetBgColorStart((v, cur.1, cur.2)));
                }
            });
            let r_row = GtkBox::new(Orientation::Horizontal, 4);
            let r_lbl = Label::new(Some("R"));
            r_lbl.add_css_class("cc-label");
            r_lbl.set_width_chars(2);
            r_row.append(&r_lbl);
            r_row.append(&r_scale);
            rgb_box.append(&r_row);
            model.bg_widgets.borrow_mut().r_scale = r_scale.downgrade();

            let g_scale = Scale::with_range(Orientation::Horizontal, 0.0, 255.0, 1.0);
            g_scale.add_css_class("cc-scale");
            g_scale.set_value(cur_col.1 as f64);
            g_scale.set_hexpand(true);
            g_scale.set_draw_value(false);
            let s_g = sender.clone();
            let st_g = model.bg_state.clone();
            let sync_g = model.syncing_bg.clone();
            g_scale.connect_value_changed(move |sc| {
                if sync_g.get() {
                    return;
                }
                let v = sc.value().round() as u8;
                let cur = st_g.borrow().color_start;
                if cur.1 != v {
                    s_g.input(AppearanceMsg::SetBgColorStart((cur.0, v, cur.2)));
                }
            });
            let g_row = GtkBox::new(Orientation::Horizontal, 4);
            let g_lbl = Label::new(Some("G"));
            g_lbl.add_css_class("cc-label");
            g_lbl.set_width_chars(2);
            g_row.append(&g_lbl);
            g_row.append(&g_scale);
            rgb_box.append(&g_row);
            model.bg_widgets.borrow_mut().g_scale = g_scale.downgrade();

            let b_scale = Scale::with_range(Orientation::Horizontal, 0.0, 255.0, 1.0);
            b_scale.add_css_class("cc-scale");
            b_scale.set_value(cur_col.2 as f64);
            b_scale.set_hexpand(true);
            b_scale.set_draw_value(false);
            let s_b = sender.clone();
            let st_b = model.bg_state.clone();
            let sync_b = model.syncing_bg.clone();
            b_scale.connect_value_changed(move |sc| {
                if sync_b.get() {
                    return;
                }
                let v = sc.value().round() as u8;
                let cur = st_b.borrow().color_start;
                if cur.2 != v {
                    s_b.input(AppearanceMsg::SetBgColorStart((cur.0, cur.1, v)));
                }
            });
            let b_row = GtkBox::new(Orientation::Horizontal, 4);
            let b_lbl = Label::new(Some("B"));
            b_lbl.add_css_class("cc-label");
            b_lbl.set_width_chars(2);
            b_row.append(&b_lbl);
            b_row.append(&b_scale);
            rgb_box.append(&b_row);
            model.bg_widgets.borrow_mut().b_scale = b_scale.downgrade();

            hex_row.append(&rgb_box);
            container.append(&hex_row);
        }
        BgMode::Gradient => {
            let colors_row = GtkBox::new(Orientation::Horizontal, 8);
            colors_row.set_halign(gtk4::Align::Center);

            // Start Color Box
            let start_box = GtkBox::new(Orientation::Horizontal, 4);
            let lbl_from = Label::new(Some(tr("From:", "От:")));
            lbl_from.add_css_class("cc-label");
            start_box.append(&lbl_from);

            let c1 = model.custom_color_start;
            let swatch1 = DrawingArea::new();
            swatch1.set_content_width(24);
            swatch1.set_content_height(24);
            swatch1.add_css_class("color-swatch-box");
            let st_sw1 = model.bg_state.clone();
            swatch1.set_draw_func(move |_area, cr, w, h| {
                let c = st_sw1.borrow().color_start;
                let w = w as f64;
                let h = h as f64;
                let r = 6.0;
                cr.new_sub_path();
                cr.arc(w - r, r, r, -std::f64::consts::FRAC_PI_2, 0.0);
                cr.arc(w - r, h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
                cr.arc(r, h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
                cr.arc(r, r, r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
                cr.close_path();
                cr.clip();
                cr.set_source_rgb(c.0 as f64 / 255.0, c.1 as f64 / 255.0, c.2 as f64 / 255.0);
                let _ = cr.paint();
            });
            start_box.append(&swatch1);
            model.bg_widgets.borrow_mut().swatch_start = swatch1.downgrade();

            let entry1 = Entry::new();
            entry1.add_css_class("hex-input");
            entry1.set_text(&format_hex_color(c1));
            entry1.set_max_length(7);
            entry1.set_width_chars(8);
            let s1 = sender.clone();
            let sync1 = model.syncing_bg.clone();
            entry1.connect_changed(move |e| {
                if sync1.get() {
                    return;
                }
                let text = e.text();
                if let Some(c) = parse_hex_color(&text) {
                    s1.input(AppearanceMsg::SetBgColorStart(c));
                }
            });
            start_box.append(&entry1);
            model.bg_widgets.borrow_mut().entry_start = entry1.downgrade();
            colors_row.append(&start_box);

            // Swap Colors Button
            let btn_swap = Button::with_label("⇄");
            btn_swap.add_css_class("filter-pill");
            btn_swap.set_tooltip_text(Some(tr("Swap Colors", "Поменять цвета местами")));
            let s_swap = sender.clone();
            btn_swap.connect_clicked(move |_| {
                s_swap.input(AppearanceMsg::SwapBgColors);
            });
            colors_row.append(&btn_swap);

            // End Color Box
            let end_box = GtkBox::new(Orientation::Horizontal, 4);
            let lbl_to = Label::new(Some(tr("To:", "До:")));
            lbl_to.add_css_class("cc-label");
            end_box.append(&lbl_to);

            let c2 = model.custom_color_end;
            let swatch2 = DrawingArea::new();
            swatch2.set_content_width(24);
            swatch2.set_content_height(24);
            swatch2.add_css_class("color-swatch-box");
            let st_sw2 = model.bg_state.clone();
            swatch2.set_draw_func(move |_area, cr, w, h| {
                let c = st_sw2.borrow().color_end;
                let w = w as f64;
                let h = h as f64;
                let r = 6.0;
                cr.new_sub_path();
                cr.arc(w - r, r, r, -std::f64::consts::FRAC_PI_2, 0.0);
                cr.arc(w - r, h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
                cr.arc(r, h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
                cr.arc(r, r, r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
                cr.close_path();
                cr.clip();
                cr.set_source_rgb(c.0 as f64 / 255.0, c.1 as f64 / 255.0, c.2 as f64 / 255.0);
                let _ = cr.paint();
            });
            end_box.append(&swatch2);
            model.bg_widgets.borrow_mut().swatch_end = swatch2.downgrade();

            let entry2 = Entry::new();
            entry2.add_css_class("hex-input");
            entry2.set_text(&format_hex_color(c2));
            entry2.set_max_length(7);
            entry2.set_width_chars(8);
            let s2 = sender.clone();
            let sync2 = model.syncing_bg.clone();
            entry2.connect_changed(move |e| {
                if sync2.get() {
                    return;
                }
                let text = e.text();
                if let Some(c) = parse_hex_color(&text) {
                    s2.input(AppearanceMsg::SetBgColorEnd(c));
                }
            });
            end_box.append(&entry2);
            model.bg_widgets.borrow_mut().entry_end = entry2.downgrade();
            colors_row.append(&end_box);

            container.append(&colors_row);

            // Direction row
            let dir_row = GtkBox::new(Orientation::Horizontal, 6);
            dir_row.set_halign(gtk4::Align::Center);
            let lbl_dir = Label::new(Some(tr("Direction:", "Направление:")));
            lbl_dir.add_css_class("cc-label");
            dir_row.append(&lbl_dir);

            let dirs = [
                (GradientDirection::Vertical, "↓", tr("Vertical (down)", "Сверху вниз")),
                (GradientDirection::Horizontal, "→", tr("Horizontal (right)", "Слева направо")),
                (GradientDirection::Diagonal, "↘", tr("Diagonal", "По диагонали")),
                (GradientDirection::Radial, "◉", tr("Radial", "Радиальный")),
            ];
            for (dir_spec, symbol, tip) in dirs {
                let btn_d = Button::with_label(symbol);
                btn_d.add_css_class("cc-chip");
                btn_d.set_tooltip_text(Some(tip));
                if model.custom_direction == dir_spec {
                    btn_d.add_css_class("active");
                }
                let s = sender.clone();
                btn_d.connect_clicked(move |_| {
                    s.input(AppearanceMsg::SetBgDirection(dir_spec));
                });
                model.bg_widgets.borrow_mut().dir_buttons.push((dir_spec, btn_d.downgrade()));
                dir_row.append(&btn_d);
            }
            container.append(&dir_row);

            // Gradient Presets
            let presets_box = GtkBox::new(Orientation::Vertical, 4);
            let lbl_presets = Label::new(Some(tr("Presets:", "Пресеты:")));
            lbl_presets.add_css_class("cc-label");
            lbl_presets.set_xalign(0.0);
            presets_box.append(&lbl_presets);

            let row1 = GtkBox::new(Orientation::Horizontal, 5);
            row1.set_halign(gtk4::Align::Center);
            let row2 = GtkBox::new(Orientation::Horizontal, 5);
            row2.set_halign(gtk4::Align::Center);

            let grad_presets = [
                ("Kanagawa", (0x18, 0x16, 0x16), (0x28, 0x27, 0x28)),
                ("Obsidian", (0x0a, 0x0a, 0x0c), (0x1a, 0x1a, 0x24)),
                ("Nordic", (0x1e, 0x23, 0x2a), (0x38, 0x43, 0x4f)),
                ("Sunset", (0x1c, 0x10, 0x24), (0x3d, 0x1a, 0x33)),
                ("Cyber", (0x0a, 0x19, 0x2f), (0x17, 0x2a, 0x45)),
                ("Emerald", (0x0a, 0x1c, 0x14), (0x16, 0x38, 0x28)),
                ("Lotus", (0xf2, 0xec, 0xbc), (0xde, 0xd7, 0xbc)),
            ];
            for (idx, (name, c_start, c_end)) in grad_presets.into_iter().enumerate() {
                let chip = Button::with_label(name);
                chip.add_css_class("cc-chip");
                if model.custom_color_start == c_start && model.custom_color_end == c_end {
                    chip.add_css_class("active");
                }
                let s = sender.clone();
                chip.connect_clicked(move |_| {
                    s.input(AppearanceMsg::SetBgColorStart(c_start));
                    s.input(AppearanceMsg::SetBgColorEnd(c_end));
                });
                model.bg_widgets.borrow_mut().grad_presets.push(((c_start, c_end), chip.downgrade()));
                if idx < 4 {
                    row1.append(&chip);
                } else {
                    row2.append(&chip);
                }
            }
            presets_box.append(&row1);
            presets_box.append(&row2);
            container.append(&presets_box);
        }
    }

    // --- Action row: Apply & Save ---
    let action_row = GtkBox::new(Orientation::Horizontal, 8);

    let btn_apply = Button::with_label(tr("󰄬  Apply Background", "󰄬  Применить фон"));
    btn_apply.add_css_class("btn-primary-action");
    btn_apply.set_hexpand(true);
    let s = sender.clone();
    btn_apply.connect_clicked(move |_| {
        s.input(AppearanceMsg::ApplyCustomBg);
    });
    action_row.append(&btn_apply);

    let btn_save = Button::with_label(tr("󰆓  Save to Wallpapers", "󰆓  Сохранить в обои"));
    btn_save.add_css_class("cc-chip");
    btn_save.set_hexpand(true);
    let s = sender.clone();
    btn_save.connect_clicked(move |_| {
        s.input(AppearanceMsg::SaveCustomBg);
    });
    action_row.append(&btn_save);

    container.append(&action_row);
}
