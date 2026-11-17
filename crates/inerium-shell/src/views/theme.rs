use std::process::Command;

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, Orientation, PolicyType};
use inerium_i18n::tr;
use relm4::prelude::*;

#[derive(Clone, Debug)]
pub struct ThemeItem {
    pub id: String,
    pub name: String,
    pub author: String,
    pub variant: String,
    pub description: String,
    pub is_active: bool,
    #[allow(dead_code)]
    pub bg: String,
    #[allow(dead_code)]
    pub surface: String,
    #[allow(dead_code)]
    pub accent: String,
    #[allow(dead_code)]
    pub fg: String,
}

pub fn query_themes() -> Vec<ThemeItem> {
    let mut items = Vec::new();
    if let Ok(out) = Command::new("inerium-theme").arg("list").arg("--json").output()
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

pub struct ThemeModel {
    pub themes: Vec<ThemeItem>,
    nav_cb: std::boxed::Box<dyn Fn(&str) + 'static>,
    #[allow(dead_code)]
    close_cb: std::boxed::Box<dyn Fn() + 'static>,
}

#[derive(Debug)]
pub enum ThemeMsg {
    ApplyTheme(String),
    Back,
}

pub struct ThemeInit {
    pub nav_cb: std::boxed::Box<dyn Fn(&str) + 'static>,
    pub close_cb: std::boxed::Box<dyn Fn() + 'static>,
}

#[relm4::component(pub)]
impl SimpleComponent for ThemeModel {
    type Input = ThemeMsg;
    type Output = ();
    type Init = ThemeInit;

    view! {
        #[root]
        gtk4::Box {
            add_css_class: "popup-container",
            set_orientation: Orientation::Vertical,
            set_spacing: 10,
            set_size_request: (384, -1),

            // Header (Back, Title & Path)
            gtk4::Box {
                set_orientation: Orientation::Horizontal,
                set_spacing: 8,

                gtk4::Button {
                    add_css_class: "back-btn",
                    set_label: &tr("‹ Back", "‹ Назад"),
                    connect_clicked => ThemeMsg::Back,
                },

                gtk4::Box {
                    set_orientation: Orientation::Vertical,
                    set_spacing: 1,
                    set_hexpand: true,

                    gtk4::Label {
                        add_css_class: "cc-host",
                        set_label: &tr("Theme Gallery", "Темы оформления"),
                        set_xalign: 0.0,
                    },

                    gtk4::Label {
                        add_css_class: "cc-meta",
                        set_label: "~/.config/inerium/themes/",
                        set_xalign: 0.0,
                    },
                },
            },

            // Theme List
            gtk4::ScrolledWindow {
                add_css_class: "list-scroll",
                set_policy: (PolicyType::Never, PolicyType::Automatic),
                set_propagate_natural_height: true,
                set_min_content_height: 200,
                set_max_content_height: 380,
                set_vexpand: true,

                #[name = "list_box"]
                gtk4::Box {
                    set_orientation: Orientation::Vertical,
                    set_spacing: 6,
                },
            },
        }
    }

    fn init(init: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let themes = query_themes();
        let model = Self {
            themes,
            nav_cb: init.nav_cb,
            close_cb: init.close_cb,
        };

        let widgets = view_output!();
        populate_theme_list(&widgets.list_box, &model.themes, &sender);

        ComponentParts { model, widgets }
    }

    fn post_view() {
        populate_theme_list(list_box, &self.themes, &sender);
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            ThemeMsg::ApplyTheme(id) => {
                let _ = Command::new("inerium-theme").arg("set").arg(&id).status();
                self.themes = query_themes();
            }
            ThemeMsg::Back => {
                (self.nav_cb)("control");
            }
        }
    }
}

fn populate_theme_list(list_box: &GtkBox, themes: &[ThemeItem], sender: &ComponentSender<ThemeModel>) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    for item in themes {
        let row = Button::new();
        row.add_css_class("list-item-btn");

        let inner = GtkBox::new(Orientation::Horizontal, 8);
        inner.set_hexpand(true);

        // Icon (dark vs light)
        let ico_str = if item.variant == "light" { "󰔏" } else { "󰔎" };
        let ico = Label::new(Some(ico_str));
        ico.add_css_class("card-icon");
        inner.append(&ico);

        // Title and description
        let txt = GtkBox::new(Orientation::Vertical, 1);
        txt.set_hexpand(true);

        let title_box = GtkBox::new(Orientation::Horizontal, 6);
        let name_lbl = Label::new(Some(&item.name));
        name_lbl.add_css_class("card-title");
        name_lbl.set_xalign(0.0);
        title_box.append(&name_lbl);

        let variant_badge = Label::new(Some(&format!("[{}]", item.variant)));
        variant_badge.add_css_class("cc-meta");
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

        // Active badge
        if item.is_active {
            let badge = Label::new(Some(tr("✓ Active", "✓ Активна")));
            badge.add_css_class("card-badge");
            inner.append(&badge);
        }

        row.set_child(Some(&inner));

        let id = item.id.clone();
        let s = sender.clone();
        row.connect_clicked(move |_| {
            s.input(ThemeMsg::ApplyTheme(id.clone()));
        });

        list_box.append(&row);
    }
}

pub fn build_theme_view<FNav, FClose>(
    nav_cb: FNav,
    close_cb: FClose,
) -> (gtk4::Box, i32, i32)
where
    FNav: Fn(&str) + 'static + Clone,
    FClose: Fn() + 'static + Clone,
{
    let init = ThemeInit {
        nav_cb: std::boxed::Box::new(nav_cb),
        close_cb: std::boxed::Box::new(close_cb),
    };

    let mut controller = ThemeModel::builder().launch(init).detach();
    controller.detach_runtime();
    let root = controller.widget().clone();
    (root, 384, 380)
}
