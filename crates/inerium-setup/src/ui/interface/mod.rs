pub mod input_overrides;
pub mod theme_card;
pub mod wallpaper_card;

use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Box, Orientation, PolicyType, ScrolledWindow};


use crate::i18n::Lang;
use crate::model::{SetupPaths, ensure_default_wallpapers};
use crate::ui::StatusCallback;

pub fn build_interface_tab(
    paths: SetupPaths,
    status_cb: StatusCallback,
    lang: Lang,
) -> (Box, Rc<dyn Fn() -> Result<(), String>>) {
    ensure_default_wallpapers(&paths);
    let outer = Box::new(Orientation::Vertical, 10);

    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .build();

    let content = Box::new(Orientation::Vertical, 12);
    content.set_margin_end(4);

    let theme_card = theme_card::build_theme_card(&status_cb, lang);
    content.append(&theme_card);

    let wallpaper_card = wallpaper_card::build_wallpaper_card(&paths, &status_cb, lang);
    content.append(&wallpaper_card);

    let (input_card, save_fn) = input_overrides::build_input_overrides_card(&paths, &status_cb, lang);
    content.append(&input_card);

    scroll.set_child(Some(&content));
    outer.append(&scroll);
    (outer, save_fn)
}
