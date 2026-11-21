//! 16-color ANSI/VTE terminal palette computation derived from theme Palettes.

use crate::model::{Palette, Rgba};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VtePalette {
    pub foreground: Rgba,
    pub background: Rgba,
    pub cursor: Rgba,
    pub highlight_fg: Rgba,
    pub highlight_bg: Rgba,
    pub colors: [Rgba; 16],
}

impl VtePalette {
    /// Computes the 16-color terminal palette and foreground/background from a theme Palette.
    /// This precisely corresponds to the palette emitted into Kitty terminal configurations.
    pub fn from_palette(p: &Palette) -> Self {
        let bg = Rgba::parse(&p.bg_canvas);
        let fg = Rgba::parse(&p.text_primary);
        let sel_bg = Rgba::parse(&p.get_surface_hover());
        let green = Rgba::parse(&p.accent_green);
        let blue = Rgba::parse(&p.accent_blue);
        let red = Rgba::parse(&p.accent_red);
        let amber = Rgba::parse(&p.accent_amber);
        let aqua = Rgba::parse(&p.accent_aqua);
        let purple = Rgba::parse(&p.accent_purple);
        let muted = Rgba::parse(&p.get_text_muted());
        let heading = Rgba::parse(&p.text_heading);

        let bright_red = red.lighten(0.18);
        let bright_green = green.lighten(0.18);
        let bright_amber = amber.lighten(0.18);
        let bright_blue = blue.lighten(0.18);
        let bright_purple = purple.lighten(0.18);
        let bright_aqua = aqua.lighten(0.18);

        let colors = [
            bg,            // color0: Normal Black
            red,           // color1: Normal Red
            green,         // color2: Normal Green
            amber,         // color3: Normal Yellow/Amber
            blue,          // color4: Normal Blue
            purple,        // color5: Normal Magenta/Purple
            aqua,          // color6: Normal Cyan/Aqua
            fg,            // color7: Normal White/Foreground
            muted,         // color8: Bright Black (Muted)
            bright_red,    // color9: Bright Red
            bright_green,  // color10: Bright Green
            bright_amber,  // color11: Bright Yellow
            bright_blue,   // color12: Bright Blue
            bright_purple, // color13: Bright Magenta
            bright_aqua,   // color14: Bright Cyan
            heading,       // color15: Bright White (Heading)
        ];

        Self {
            foreground: fg,
            background: bg,
            cursor: fg,
            highlight_fg: fg,
            highlight_bg: sel_bg,
            colors,
        }
    }

    /// Returns (foreground, background, [colors; 16]) tuple.
    pub fn as_tuple(&self) -> (Rgba, Rgba, [Rgba; 16]) {
        (self.foreground, self.background, self.colors)
    }

    /// Returns hex strings (foreground, background, [colors; 16]).
    pub fn to_hex_tuple(&self) -> (String, String, [String; 16]) {
        let fg = self.foreground.to_hex_rgb();
        let bg = self.background.to_hex_rgb();
        let colors = std::array::from_fn(|i| self.colors[i].to_hex_rgb());
        (fg, bg, colors)
    }
}

/// Helper function to compute VTE palette directly from `Palette`.
pub fn compute_vte_palette(palette: &Palette) -> (Rgba, Rgba, [Rgba; 16]) {
    VtePalette::from_palette(palette).as_tuple()
}

/// Helper function to compute VTE hex palette directly from `Palette`.
pub fn compute_vte_palette_hex(palette: &Palette) -> (String, String, [String; 16]) {
    VtePalette::from_palette(palette).to_hex_tuple()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vte_palette_generation() {
        let p = Palette {
            bg_canvas: "#111216".to_string(),
            bg_surface: "#1e2029".to_string(),
            bg_surface_alt: None,
            bg_surface_hover: None,
            bg_surface_active: None,
            text_heading: "#e6e8ee".to_string(),
            text_primary: "#cfd2dd".to_string(),
            text_muted: None,
            text_subtle: None,
            text_inverted: None,
            border_subtle: "rgba(147, 138, 169, 0.20)".to_string(),
            border_active: "#938aa9".to_string(),
            border_active_secondary: None,
            accent_green: "#87a987".to_string(),
            accent_green_bg: None,
            accent_green_border: None,
            accent_blue: "#7fb4ca".to_string(),
            accent_blue_bg: None,
            accent_blue_border: None,
            accent_amber: "#e6c384".to_string(),
            accent_amber_bg: None,
            accent_amber_border: None,
            accent_red: "#e46876".to_string(),
            accent_red_bg: None,
            accent_red_border: None,
            accent_aqua: "#7aa89f".to_string(),
            accent_aqua_bg: None,
            accent_aqua_border: None,
            accent_purple: "#938aa9".to_string(),
            accent_purple_bg: None,
            accent_purple_border: None,
            accent_primary: None,
            accent_orange: None,
            slider_fill: None,
            slider_fill_secondary: None,
            slider_trough: None,
            slider_thumb: None,
            shadow: None,
            modal_backdrop: None,
        };

        let (fg, bg, colors) = compute_vte_palette(&p);
        assert_eq!(fg, Rgba::parse("#cfd2dd"));
        assert_eq!(bg, Rgba::parse("#111216"));
        assert_eq!(colors[0], Rgba::parse("#111216")); // color0 = bg
        assert_eq!(colors[1], Rgba::parse("#e46876")); // color1 = red
        assert_eq!(colors[2], Rgba::parse("#87a987")); // color2 = green
        assert_eq!(colors[15], Rgba::parse("#e6e8ee")); // color15 = heading
    }
}
