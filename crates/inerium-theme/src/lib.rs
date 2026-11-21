//! Shared theme types, palette models, detection, and default tokens for Inerium Shell.

pub mod detect;
pub mod model;
pub mod tokens;

pub use detect::{detect_theme, is_dark_theme, ThemeMode};
pub use model::{DisplayConfig, Palette, Rgba, ThemeConfig, ThemeMeta, Typography};
pub use tokens::{DRAGON_VARS, LOTUS_VARS, THEME_ALIASES};
