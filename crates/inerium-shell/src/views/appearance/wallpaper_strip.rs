use gtk4::gio;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, GestureClick, Label, Orientation, Picture};
use inerium_i18n::tr;
use relm4::ComponentSender;

use super::{AppearanceModel, AppearanceMsg};
use crate::hw;

pub fn populate_monitor_pills(
    box_widget: &GtkBox,
    model: &AppearanceModel,
    sender: &ComponentSender<AppearanceModel>,
) {
    while let Some(child) = box_widget.first_child() {
        box_widget.remove(&child);
    }

    if !model.has_multiple_monitors {
        return;
    }

    let all_btn = Button::with_label(tr("All Monitors", "Все экраны"));
    all_btn.add_css_class("filter-pill");
    if model.selected_monitor == "all" {
        all_btn.add_css_class("active");
    }
    let s = sender.clone();
    all_btn.connect_clicked(move |_| {
        s.input(AppearanceMsg::SelectMonitor("all".to_string()));
    });
    box_widget.append(&all_btn);

    for m in &model.connected_monitors {
        let name = m.name.clone();
        let btn = Button::with_label(&m.label);
        btn.add_css_class("filter-pill");
        if model.selected_monitor == name {
            btn.add_css_class("active");
        }
        let s2 = sender.clone();
        let n_clone = name.clone();
        btn.connect_clicked(move |_| {
            s2.input(AppearanceMsg::SelectMonitor(n_clone.clone()));
        });
        box_widget.append(&btn);
    }
}

pub fn update_bottom_row(
    bottom_row: &GtkBox,
    item: &hw::WallpaperItem,
    is_confirming: bool,
    inerium_dir_str: &str,
    sender: &ComponentSender<AppearanceModel>,
) {
    while let Some(child) = bottom_row.first_child() {
        bottom_row.remove(&child);
    }

    if is_confirming {
        let btn_yes = Button::with_label(tr("󰆴 Delete", "󰆴 Удалить"));
        btn_yes.add_css_class("wall-del-confirm");
        btn_yes.set_hexpand(true);
        let s_del = sender.clone();
        let path_del = item.path.clone();
        btn_yes.connect_clicked(move |_| {
            s_del.input(AppearanceMsg::ConfirmDeleteWallpaper(path_del.clone()));
        });
        bottom_row.append(&btn_yes);

        let btn_no = Button::with_label(tr("󰅖 Cancel", "󰅖 Отмена"));
        btn_no.add_css_class("wall-del-cancel");
        btn_no.set_hexpand(true);
        let s_cancel = sender.clone();
        btn_no.connect_clicked(move |_| {
            s_cancel.input(AppearanceMsg::CancelDeleteWallpaper);
        });
        bottom_row.append(&btn_no);
    } else {
        let name_lbl = Label::new(Some(&item.name));
        name_lbl.add_css_class("wall-name");
        name_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        name_lbl.set_hexpand(true);
        name_lbl.set_xalign(0.0);

        let click_label = GestureClick::new();
        let s_lbl = sender.clone();
        let path_lbl = item.path.clone();
        click_label.connect_released(move |_, _, _, _| {
            s_lbl.input(AppearanceMsg::ApplyWallpaper(path_lbl.clone()));
        });
        name_lbl.add_controller(click_label);
        bottom_row.append(&name_lbl);

        let is_user_deletable = !inerium_dir_str.is_empty() && item.path.starts_with(inerium_dir_str);
        if is_user_deletable {
            let del_btn = Button::with_label("󰆴");
            del_btn.add_css_class("wall-del-btn");
            del_btn.set_tooltip_text(Some(tr("Delete from folder", "Удалить из папки")));
            let s_req = sender.clone();
            let path_req = item.path.clone();
            del_btn.connect_clicked(move |_| {
                s_req.input(AppearanceMsg::RequestDeleteWallpaper(path_req.clone()));
            });
            bottom_row.append(&del_btn);
        }
    }
}

pub fn populate_wallpapers_strip(
    scroller: &gtk4::ScrolledWindow,
    strip: &GtkBox,
    model: &AppearanceModel,
    sender: &ComponentSender<AppearanceModel>,
) {
    if model.loading_wallpapers {
        while let Some(child) = strip.first_child() {
            strip.remove(&child);
        }
        let loading = Label::new(Some(tr(" Loading wallpapers...", " Загрузка обоев...")));
        loading.add_css_class("card-sub");
        strip.append(&loading);
        return;
    }

    if model.wallpapers.is_empty() {
        while let Some(child) = strip.first_child() {
            strip.remove(&child);
        }
        let empty = Label::new(Some(tr("No wallpapers in folder", "Нет обоев в папке")));
        empty.add_css_class("card-sub");
        strip.append(&empty);
        return;
    }

    let active_wall = if model.selected_monitor == "all" {
        model.connected_monitors.first().and_then(|m| model.active_wallpapers.get(&m.name))
    } else {
        model.active_wallpapers.get(&model.selected_monitor)
    };

    let inerium_dir = hw::inerium_wallpapers_dir();
    let inerium_dir_str = inerium_dir.to_str().unwrap_or("").to_string();

    // Collect existing card widgets in strip
    let mut existing_cards: Vec<gtk4::Widget> = Vec::new();
    let mut next_child = strip.first_child();
    while let Some(c) = next_child {
        next_child = c.next_sibling();
        existing_cards.push(c);
    }

    // Fast in-place path: if card count matches model.wallpapers, update in-place without touching strip or scroll position!
    let can_update_in_place = existing_cards.len() == model.wallpapers.len()
        && existing_cards.iter().zip(&model.wallpapers).all(|(card_w, item)| {
            card_w.widget_name().as_str() == item.path.as_str()
        });

    if can_update_in_place {
        for (card_w, item) in existing_cards.iter().zip(&model.wallpapers) {
            let is_active = active_wall.map(|w| w == &item.path).unwrap_or(false);
            if is_active {
                card_w.add_css_class("active");
            } else {
                card_w.remove_css_class("active");
            }

            // The last child of card_w is bottom_row (GtkBox)
            if let Some(card_box) = card_w.downcast_ref::<GtkBox>()
                && let Some(bottom_row_w) = card_box.last_child()
                && let Some(bottom_row) = bottom_row_w.downcast_ref::<GtkBox>()
            {
                let is_confirming = model.delete_confirm_path.as_deref() == Some(&item.path);
                update_bottom_row(bottom_row, item, is_confirming, &inerium_dir_str, sender);
            }
        }
        return;
    }

    // Fallback: wallpaper list changed (added or deleted). Save and restore horizontal scroll position!
    let hadj = scroller.hadjustment();
    let prev_scroll = hadj.value();

    while let Some(child) = strip.first_child() {
        strip.remove(&child);
    }

    for item in &model.wallpapers {
        let card = GtkBox::new(Orientation::Vertical, 4);
        card.add_css_class("wall-card");
        card.set_size_request(140, 114);
        card.set_valign(gtk4::Align::Center);
        card.set_widget_name(&item.path);

        let is_active = active_wall.map(|w| w == &item.path).unwrap_or(false);
        if is_active {
            card.add_css_class("active");
        }

        // Thumbnail Picture wrapped in clickable button
        let pic = if let Some(thumb) = &item.thumb_path {
            Picture::for_filename(thumb)
        } else {
            let file = gio::File::for_path(&item.path);
            if let Ok(tex) = gdk4::Texture::from_file(&file) {
                Picture::for_paintable(&tex)
            } else {
                Picture::for_filename(&item.path)
            }
        };
        pic.set_content_fit(gtk4::ContentFit::Cover);
        pic.set_size_request(128, 72);
        pic.set_can_shrink(true);
        pic.add_css_class("wall-thumb");

        let top_btn = Button::new();
        top_btn.add_css_class("wall-btn");
        top_btn.set_size_request(128, 72);
        top_btn.set_vexpand(false);
        top_btn.set_child(Some(&pic));

        let s_apply = sender.clone();
        let path_apply = item.path.clone();
        top_btn.connect_clicked(move |_| {
            s_apply.input(AppearanceMsg::ApplyWallpaper(path_apply.clone()));
        });
        card.append(&top_btn);

        // Bottom label and clean delete action with confirmation
        let bottom_row = GtkBox::new(Orientation::Horizontal, 4);
        bottom_row.set_size_request(-1, 24);
        bottom_row.set_valign(gtk4::Align::Center);
        let is_confirming = model.delete_confirm_path.as_deref() == Some(&item.path);
        update_bottom_row(&bottom_row, item, is_confirming, &inerium_dir_str, sender);
        card.append(&bottom_row);

        strip.append(&card);
    }

    if prev_scroll > 0.0 {
        glib::idle_add_local_once(move || {
            hadj.set_value(prev_scroll);
        });
    }
}
