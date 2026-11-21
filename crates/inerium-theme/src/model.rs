//! Palette and theme configuration data models for Inerium Shell.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub theme: ThemeMeta,
    pub palette: Palette,
    #[serde(default)]
    pub typography: Option<Typography>,
    #[serde(default)]
    pub display: Option<DisplayConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Typography {
    #[serde(default)]
    pub font_ui: Option<String>,
    #[serde(default)]
    pub font_mono: Option<String>,
    #[serde(default)]
    pub font_size: Option<f32>,
}

impl Typography {
    pub fn get_ui_font(&self) -> &str {
        self.font_ui.as_deref().unwrap_or("JetBrainsMono Nerd Font")
    }

    pub fn get_mono_font(&self) -> &str {
        self.font_mono.as_deref().unwrap_or("JetBrainsMono Nerd Font")
    }

    pub fn get_size(&self) -> f32 {
        self.font_size.unwrap_or(11.0)
    }

    pub fn to_css_font_family(&self) -> String {
        let ui = self.get_ui_font();
        if ui.contains("Nerd Font") {
            format!("\"{}\", monospace", ui)
        } else {
            format!("\"{}\", \"JetBrainsMono Nerd Font\", sans-serif, monospace", ui)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DisplayConfig {
    pub temperature: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeMeta {
    pub id: String,
    pub name: String,
    pub author: String,
    pub upstream: Option<String>,
    pub variant: String, // "dark" | "light"
    pub license: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub opacity: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Palette {
    pub bg_canvas: String,
    pub bg_surface: String,
    #[serde(default)]
    pub bg_surface_alt: Option<String>,
    #[serde(default)]
    pub bg_surface_hover: Option<String>,
    #[serde(default)]
    pub bg_surface_active: Option<String>,

    pub border_subtle: String,
    pub border_active: String,
    #[serde(default)]
    pub border_active_secondary: Option<String>,

    pub text_primary: String,
    pub text_heading: String,
    #[serde(default)]
    pub text_muted: Option<String>,
    #[serde(default)]
    pub text_subtle: Option<String>,
    #[serde(default)]
    pub text_inverted: Option<String>,

    pub accent_green: String,
    #[serde(default)]
    pub accent_green_bg: Option<String>,
    #[serde(default)]
    pub accent_green_border: Option<String>,

    pub accent_blue: String,
    #[serde(default)]
    pub accent_blue_bg: Option<String>,
    #[serde(default)]
    pub accent_blue_border: Option<String>,

    pub accent_amber: String,
    #[serde(default)]
    pub accent_amber_bg: Option<String>,
    #[serde(default)]
    pub accent_amber_border: Option<String>,

    pub accent_red: String,
    #[serde(default)]
    pub accent_red_bg: Option<String>,
    #[serde(default)]
    pub accent_red_border: Option<String>,

    pub accent_aqua: String,
    #[serde(default)]
    pub accent_aqua_bg: Option<String>,
    #[serde(default)]
    pub accent_aqua_border: Option<String>,

    pub accent_purple: String,
    #[serde(default)]
    pub accent_purple_bg: Option<String>,
    #[serde(default)]
    pub accent_purple_border: Option<String>,

    #[serde(default)]
    pub accent_primary: Option<String>,
    #[serde(default)]
    pub accent_orange: Option<String>,

    #[serde(default)]
    pub slider_fill: Option<String>,
    #[serde(default)]
    pub slider_fill_secondary: Option<String>,
    #[serde(default)]
    pub slider_trough: Option<String>,
    #[serde(default)]
    pub slider_thumb: Option<String>,

    #[serde(default)]
    pub shadow: Option<String>,
    #[serde(default)]
    pub modal_backdrop: Option<String>,
}

impl Palette {
    pub fn get_primary(&self) -> &str {
        self.accent_primary.as_deref().unwrap_or(&self.accent_blue)
    }

    pub fn get_orange(&self) -> &str {
        self.accent_orange.as_deref().unwrap_or(&self.accent_amber)
    }

    pub fn get_primary_bg(&self) -> String {
        let rgba = Rgba::parse(self.get_primary());
        format!("rgba({}, {}, {}, 0.16)", rgba.0, rgba.1, rgba.2)
    }

    pub fn get_primary_border(&self) -> String {
        let rgba = Rgba::parse(self.get_primary());
        format!("rgba({}, {}, {}, 0.35)", rgba.0, rgba.1, rgba.2)
    }

    pub fn get_slider_fill(&self) -> &str {
        self.slider_fill.as_deref().unwrap_or_else(|| self.get_primary())
    }

    pub fn get_slider_fill_secondary(&self) -> &str {
        self.slider_fill_secondary.as_deref().unwrap_or(&self.accent_amber)
    }

    pub fn get_slider_trough(&self) -> String {
        self.slider_trough.clone().unwrap_or_else(|| {
            Rgba::parse(&self.text_primary).with_alpha(26).to_css_rgba()
        })
    }

    pub fn get_slider_thumb(&self) -> &str {
        self.slider_thumb.as_deref().unwrap_or(&self.text_heading)
    }

    pub fn get_border_active_secondary(&self) -> &str {
        self.border_active_secondary.as_deref().unwrap_or(&self.border_active)
    }

    pub fn get_surface_alt(&self) -> String {
        self.bg_surface_alt.clone().unwrap_or_else(|| {
            Rgba::parse(&self.bg_surface).darken(0.06).to_hex_rgb()
        })
    }

    pub fn get_surface_hover(&self) -> String {
        self.bg_surface_hover.clone().unwrap_or_else(|| {
            Rgba::parse(&self.bg_surface).lighten(0.08).to_hex_rgb()
        })
    }

    pub fn get_surface_active(&self) -> String {
        self.bg_surface_active.clone().unwrap_or_else(|| {
            Rgba::parse(&self.bg_surface).lighten(0.15).to_hex_rgb()
        })
    }

    pub fn get_text_muted(&self) -> String {
        self.text_muted.clone().unwrap_or_else(|| {
            Rgba::parse(&self.text_primary).darken(0.40).to_hex_rgb()
        })
    }

    pub fn get_text_subtle(&self) -> String {
        self.text_subtle.clone().unwrap_or_else(|| {
            Rgba::parse(&self.text_primary).darken(0.20).to_hex_rgb()
        })
    }

    pub fn get_text_inverted(&self) -> &str {
        self.text_inverted.as_deref().unwrap_or(&self.bg_canvas)
    }

    pub fn get_green_bg(&self) -> String {
        self.accent_green_bg.clone().unwrap_or_else(|| {
            let rgba = Rgba::parse(&self.accent_green);
            format!("rgba({}, {}, {}, 0.16)", rgba.0, rgba.1, rgba.2)
        })
    }

    pub fn get_green_border(&self) -> String {
        self.accent_green_border.clone().unwrap_or_else(|| {
            let rgba = Rgba::parse(&self.accent_green);
            format!("rgba({}, {}, {}, 0.35)", rgba.0, rgba.1, rgba.2)
        })
    }

    pub fn get_blue_bg(&self) -> String {
        self.accent_blue_bg.clone().unwrap_or_else(|| {
            let rgba = Rgba::parse(&self.accent_blue);
            format!("rgba({}, {}, {}, 0.16)", rgba.0, rgba.1, rgba.2)
        })
    }

    pub fn get_blue_border(&self) -> String {
        self.accent_blue_border.clone().unwrap_or_else(|| {
            let rgba = Rgba::parse(&self.accent_blue);
            format!("rgba({}, {}, {}, 0.35)", rgba.0, rgba.1, rgba.2)
        })
    }

    pub fn get_amber_bg(&self) -> String {
        self.accent_amber_bg.clone().unwrap_or_else(|| {
            let rgba = Rgba::parse(&self.accent_amber);
            format!("rgba({}, {}, {}, 0.16)", rgba.0, rgba.1, rgba.2)
        })
    }

    pub fn get_amber_border(&self) -> String {
        self.accent_amber_border.clone().unwrap_or_else(|| {
            let rgba = Rgba::parse(&self.accent_amber);
            format!("rgba({}, {}, {}, 0.35)", rgba.0, rgba.1, rgba.2)
        })
    }

    pub fn get_red_bg(&self) -> String {
        self.accent_red_bg.clone().unwrap_or_else(|| {
            let rgba = Rgba::parse(&self.accent_red);
            format!("rgba({}, {}, {}, 0.16)", rgba.0, rgba.1, rgba.2)
        })
    }

    pub fn get_red_border(&self) -> String {
        self.accent_red_border.clone().unwrap_or_else(|| {
            let rgba = Rgba::parse(&self.accent_red);
            format!("rgba({}, {}, {}, 0.35)", rgba.0, rgba.1, rgba.2)
        })
    }

    pub fn get_aqua_bg(&self) -> String {
        self.accent_aqua_bg.clone().unwrap_or_else(|| {
            let rgba = Rgba::parse(&self.accent_aqua);
            format!("rgba({}, {}, {}, 0.16)", rgba.0, rgba.1, rgba.2)
        })
    }

    pub fn get_aqua_border(&self) -> String {
        self.accent_aqua_border.clone().unwrap_or_else(|| {
            let rgba = Rgba::parse(&self.accent_aqua);
            format!("rgba({}, {}, {}, 0.35)", rgba.0, rgba.1, rgba.2)
        })
    }

    pub fn get_purple_bg(&self) -> String {
        self.accent_purple_bg.clone().unwrap_or_else(|| {
            let rgba = Rgba::parse(&self.accent_purple);
            format!("rgba({}, {}, {}, 0.16)", rgba.0, rgba.1, rgba.2)
        })
    }

    pub fn get_purple_border(&self) -> String {
        self.accent_purple_border.clone().unwrap_or_else(|| {
            let rgba = Rgba::parse(&self.accent_purple);
            format!("rgba({}, {}, {}, 0.35)", rgba.0, rgba.1, rgba.2)
        })
    }

    pub fn get_shadow(&self) -> &str {
        self.shadow.as_deref().unwrap_or("rgba(0, 0, 0, 0.85)")
    }

    pub fn get_modal_backdrop(&self) -> &str {
        self.modal_backdrop.as_deref().unwrap_or("rgba(5, 5, 7, 0.75)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

impl Rgba {
    pub fn parse(s: &str) -> Self {
        let s = s.trim();
        if s.is_empty() {
            return Rgba(0, 0, 0, 255);
        }

        // Named colors
        if s.eq_ignore_ascii_case("transparent") {
            return Rgba(0, 0, 0, 0);
        } else if s.eq_ignore_ascii_case("black") {
            return Rgba(0, 0, 0, 255);
        } else if s.eq_ignore_ascii_case("white") {
            return Rgba(255, 255, 255, 255);
        }

        // Hex formats (with or without '#')
        let hex_str = s.strip_prefix('#').unwrap_or(s);
        let is_pure_hex = hex_str.chars().all(|c| c.is_ascii_hexdigit());

        if is_pure_hex {
            match hex_str.len() {
                3 => {
                    let r = u8::from_str_radix(&hex_str[0..1], 16).unwrap_or(0);
                    let g = u8::from_str_radix(&hex_str[1..2], 16).unwrap_or(0);
                    let b = u8::from_str_radix(&hex_str[2..3], 16).unwrap_or(0);
                    return Rgba(r * 17, g * 17, b * 17, 255);
                }
                4 => {
                    let r = u8::from_str_radix(&hex_str[0..1], 16).unwrap_or(0);
                    let g = u8::from_str_radix(&hex_str[1..2], 16).unwrap_or(0);
                    let b = u8::from_str_radix(&hex_str[2..3], 16).unwrap_or(0);
                    let a = u8::from_str_radix(&hex_str[3..4], 16).unwrap_or(15);
                    return Rgba(r * 17, g * 17, b * 17, a * 17);
                }
                6 => {
                    let r = u8::from_str_radix(&hex_str[0..2], 16).unwrap_or(0);
                    let g = u8::from_str_radix(&hex_str[2..4], 16).unwrap_or(0);
                    let b = u8::from_str_radix(&hex_str[4..6], 16).unwrap_or(0);
                    return Rgba(r, g, b, 255);
                }
                8 => {
                    let r = u8::from_str_radix(&hex_str[0..2], 16).unwrap_or(0);
                    let g = u8::from_str_radix(&hex_str[2..4], 16).unwrap_or(0);
                    let b = u8::from_str_radix(&hex_str[4..6], 16).unwrap_or(0);
                    let a = u8::from_str_radix(&hex_str[6..8], 16).unwrap_or(255);
                    return Rgba(r, g, b, a);
                }
                _ => {}
            }
        }

        // Functional rgb(...) and rgba(...) handling
        if s.starts_with("rgb") {
            let inner = s
                .strip_prefix("rgba(")
                .or_else(|| s.strip_prefix("rgb("))
                .and_then(|str_val| str_val.strip_suffix(')'));

            if let Some(inner) = inner {
                let normalized = inner.replace([',', '/'], " ");
                let parts: Vec<&str> = normalized.split_whitespace().collect();

                if parts.len() >= 3 {
                    let r = parts[0].parse::<u8>().unwrap_or(0);
                    let g = parts[1].parse::<u8>().unwrap_or(0);
                    let b = parts[2].parse::<u8>().unwrap_or(0);
                    let a = if parts.len() >= 4 {
                        let a_str = parts[3].trim_end_matches('%');
                        if let Ok(alpha_float) = a_str.parse::<f32>() {
                            if parts[3].ends_with('%') {
                                (alpha_float.clamp(0.0, 100.0) / 100.0 * 255.0).round() as u8
                            } else if alpha_float <= 1.0 {
                                (alpha_float.clamp(0.0, 1.0) * 255.0).round() as u8
                            } else {
                                alpha_float.clamp(0.0, 255.0).round() as u8
                            }
                        } else {
                            parts[3].parse::<u8>().unwrap_or(255)
                        }
                    } else {
                        255
                    };
                    return Rgba(r, g, b, a);
                }
            }
        }

        Rgba(0, 0, 0, 255)
    }

    pub fn with_alpha(&self, alpha: u8) -> Self {
        Rgba(self.0, self.1, self.2, alpha)
    }

    pub fn with_alpha_f32(&self, alpha: f32) -> Self {
        Rgba(self.0, self.1, self.2, (alpha.clamp(0.0, 1.0) * 255.0).round() as u8)
    }

    pub fn to_css_rgba(&self) -> String {
        if self.3 == 255 {
            self.to_hex_rgb()
        } else {
            let a = (self.3 as f32 / 255.0 * 100.0).round() / 100.0;
            format!("rgba({}, {}, {}, {:.2})", self.0, self.1, self.2, a)
        }
    }

    pub fn to_hex_rgb(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }

    pub fn to_hex_rgba(&self) -> String {
        format!("#{:02x}{:02x}{:02x}{:02x}", self.0, self.1, self.2, self.3)
    }

    pub fn to_hypr(&self) -> String {
        format!("rgba({:02x}{:02x}{:02x}{:02x})", self.0, self.1, self.2, self.3)
    }

    pub fn to_kde_rgb(&self) -> String {
        format!("{},{},{}", self.0, self.1, self.2)
    }

    pub fn lighten(&self, factor: f32) -> Self {
        let f = factor.clamp(0.0, 1.0);
        let r = (self.0 as f32 + (255.0 - self.0 as f32) * f).round().clamp(0.0, 255.0) as u8;
        let g = (self.1 as f32 + (255.0 - self.1 as f32) * f).round().clamp(0.0, 255.0) as u8;
        let b = (self.2 as f32 + (255.0 - self.2 as f32) * f).round().clamp(0.0, 255.0) as u8;
        Rgba(r, g, b, self.3)
    }

    pub fn darken(&self, factor: f32) -> Self {
        let f = (1.0 - factor.clamp(0.0, 1.0)).clamp(0.0, 1.0);
        let r = (self.0 as f32 * f).round().clamp(0.0, 255.0) as u8;
        let g = (self.1 as f32 * f).round().clamp(0.0, 255.0) as u8;
        let b = (self.2 as f32 * f).round().clamp(0.0, 255.0) as u8;
        Rgba(r, g, b, self.3)
    }
}

