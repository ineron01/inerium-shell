//! Shared theme types, palette models, detection, loaders, emitters, application, and ANSI/VTE tokens for Inerium Shell.

pub mod apply;
pub mod detect;
pub mod emit;
pub mod loader;
pub mod model;
pub mod tokens;
pub mod vte;

pub use apply::apply_theme;
pub use detect::{detect_theme, is_dark_theme, ThemeMode};
pub use emit::*;
pub use loader::*;
pub use model::{DisplayConfig, Palette, Rgba, ThemeConfig, ThemeMeta, Typography};
pub use tokens::{DRAGON_VARS, LOTUS_VARS, THEME_ALIASES};
pub use vte::{compute_vte_palette, compute_vte_palette_hex, VtePalette};
