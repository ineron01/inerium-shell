use std::process::Command;

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, DrawingArea, Label, Orientation};
use inerium_i18n::tr;
use relm4::ComponentSender;

use super::{AppearanceModel, AppearanceMsg};

#[derive(Clone, Debug)]
pub struct ThemeItem {
    pub id: String,
    pub name: String,
    pub author: String,
    pub variant: String,
    pub description: String,
    pub is_active: bool,
    pub bg: String,
    pub surface: String,
    pub accent: String,
    pub fg: String,
}

pub fn query_themes() -> Vec<ThemeItem> {
    let mut items = Vec::new();
    let themes_map = inerium_theme::load_available_themes();
    let cur_id = inerium_theme::get_current_theme_id();
    for (id, t) in themes_map {
        let accent = t.palette.get_primary().to_string();
        items.push(ThemeItem {
            id: id.clone(),
            name: t.theme.name,
            author: t.theme.author,
            variant: t.theme.variant,
            description: t.theme.description.unwrap_or_default(),
            is_active: id == cur_id,
            bg: t.palette.bg_canvas,
            surface: t.palette.bg_surface,
            accent,
            fg: t.palette.text_primary,
        });
    }

    if items.is_empty()
        && let Ok(out) = Command::new("inerium-theme").arg("list").arg("--json").output()
        && let Ok(val) = serde_json::from_slice::<Vec<serde_json::Value>>(&out.stdout)
    {
            for v in val {
                let id = v.get("id").and_then(|x| x.as_str()).unwrap_or_default().to_string();
                let name = v.get("name").and_then(|x| x.as_str()).unwrap_or_default().to_string();
                let author = v.get("author").and_then(|x| x.as_str()).unwrap_or_default().to_string();
                let variant = v.get("variant").and_then(|x| x.as_str()).unwrap_or_default().to_string();
                let description = v.get("description").and_then(|x| x.as_str()).unwrap_or_default().to_string();
                let is_active = v.get("is_active").and_then(|x| x.as_bool()).unwrap_or(false);
                let bg = v.pointer("/colors/bg").and_then(|x| x.as_str()).unwrap_or("#221f1f").to_string();
                let surface = v.pointer("/colors/surface").and_then(|x| x.as_str()).unwrap_or("#2d2a2a").to_string();
                let accent = v.pointer("/colors/accent").and_then(|x| x.as_str()).unwrap_or("#8ba4b0").to_string();
                let fg = v.pointer("/colors/fg").and_then(|x| x.as_str()).unwrap_or("#c5c9c5").to_string();
                items.push(ThemeItem {
                    id,
                    name,
                    author,
                    variant,
                    description,
                    is_active,
                    bg,
                    surface,
                    accent,
                    fg,
                });
            }
        }
    items
}

pub fn parse_hex_color(s: &str) -> Option<(u8, u8, u8)> {
    let hex = s.trim().strip_prefix('#').unwrap_or(s.trim());
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

pub fn format_hex_color(c: (u8, u8, u8)) -> String {
    format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
}

pub fn make_color_swatch_static(r: u8, g: u8, b: u8, width: i32, height: i32, radius: f64) -> DrawingArea {
    let area = DrawingArea::new();
    area.set_content_width(width);
    area.set_content_height(height);
    area.set_draw_func(move |_area, cr, w, h| {
        let w = w as f64;
        let h = h as f64;
        let inset = 0.5;
        let w_in = (w - inset * 2.0).max(1.0);
        let h_in = (h - inset * 2.0).max(1.0);
        let rad = (radius - inset).max(0.0).min(w_in / 2.0).min(h_in / 2.0);

        if rad > 0.0 {
            cr.new_sub_path();
            cr.arc(inset + w_in - rad, inset + rad, rad, -std::f64::consts::FRAC_PI_2, 0.0);
            cr.arc(inset + w_in - rad, inset + h_in - rad, rad, 0.0, std::f64::consts::FRAC_PI_2);
            cr.arc(inset + rad, inset + h_in - rad, rad, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
            cr.arc(inset + rad, inset + rad, rad, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
            cr.close_path();
        } else {
            cr.rectangle(inset, inset, w_in, h_in);
        }

        cr.set_source_rgb(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
        let _ = cr.fill_preserve();

        // Adaptive high-contrast rim: prevents swatch from blending into any light/dark background
        let lum = 0.299 * (r as f64 / 255.0) + 0.587 * (g as f64 / 255.0) + 0.114 * (b as f64 / 255.0);
        cr.set_line_width(1.0);
        if lum < 0.45 {
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.28);
        } else {
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.24);
        }
        let _ = cr.stroke();
    });
    area
}

pub fn populate_theme_list(list_box: &GtkBox, themes: &[ThemeItem], sender: &ComponentSender<AppearanceModel>) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    for item in themes {
        let row = Button::new();
        row.add_css_class("theme-card");
        if item.is_active {
            row.add_css_class("active");
        }

        let inner = GtkBox::new(Orientation::Horizontal, 8);
        inner.set_hexpand(true);

        let ico_str = if item.variant == "light" { "󰔏" } else { "󰔎" };
        let ico = Label::new(Some(ico_str));
        ico.add_css_class("card-icon");
        if item.is_active {
            ico.add_css_class("active");
        }
        inner.append(&ico);

        let txt = GtkBox::new(Orientation::Vertical, 1);
        txt.set_hexpand(true);

        let title_box = GtkBox::new(Orientation::Horizontal, 6);
        let name_lbl = Label::new(Some(&item.name));
        name_lbl.add_css_class("card-title");
        name_lbl.set_xalign(0.0);
        title_box.append(&name_lbl);

        let variant_str = if item.variant.to_lowercase() == "light" { "LIGHT" } else { "DARK" };
        let variant_badge = Label::new(Some(variant_str));
        variant_badge.add_css_class("theme-variant-pill");
        variant_badge.set_xalign(0.0);
        title_box.append(&variant_badge);
        txt.append(&title_box);

        let sub_text = if !item.description.is_empty() {
            format!("{} • {}", item.author, item.description)
        } else {
            format!("by {}", item.author)
        };
        let desc_lbl = Label::new(Some(&sub_text));
        desc_lbl.add_css_class("card-sub");
        desc_lbl.set_xalign(0.0);
        desc_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        txt.append(&desc_lbl);
        inner.append(&txt);

        // Theme palette dots preview in high-contrast tray
        let palette_box = GtkBox::new(Orientation::Horizontal, 4);
        palette_box.add_css_class("theme-palette-tray");
        palette_box.set_valign(gtk4::Align::Center);
        let labels = [
            tr("Background", "Фон"),
            tr("Surface", "Панели"),
            tr("Accent", "Акцент"),
            tr("Foreground", "Текст"),
        ];
        for (i, col_str) in [&item.bg, &item.surface, &item.accent, &item.fg].iter().enumerate() {
            let rgba = inerium_theme::Rgba::parse(col_str);
            let dot = make_color_swatch_static(rgba.0, rgba.1, rgba.2, 12, 12, 6.0);
            dot.set_tooltip_text(Some(&format!("{}: {}", labels[i], col_str)));
            palette_box.append(&dot);
        }
        inner.append(&palette_box);

        if item.is_active {
            let badge = Label::new(Some(tr("󰄬  Active", "󰄬  Активна")));
            badge.add_css_class("card-badge");
            badge.add_css_class("active");
            badge.set_valign(gtk4::Align::Center);
            inner.append(&badge);
        }

        row.set_child(Some(&inner));

        let id = item.id.clone();
        let s = sender.clone();
        row.connect_clicked(move |_| {
            s.input(AppearanceMsg::ApplyTheme(id.clone()));
        });

        list_box.append(&row);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_color("#1f2430"), Some((0x1f, 0x24, 0x30)));
        assert_eq!(parse_hex_color("1F2430"), Some((0x1f, 0x24, 0x30)));
        assert_eq!(parse_hex_color("#fff"), None);
        assert_eq!(parse_hex_color("12345"), None);
        assert_eq!(parse_hex_color("1234567"), None);
        assert_eq!(parse_hex_color("xyz123"), None);
        assert_eq!(parse_hex_color(""), None);
    }
}
