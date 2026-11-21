use std::fs;
use std::path::{Path, PathBuf};
use image::{ImageBuffer, Rgb};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BgMode {
    Solid,
    Gradient,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GradientDirection {
    Vertical,   // Top to bottom
    Horizontal, // Left to right
    Diagonal,   // Top-left to bottom-right
    Radial,     // Center to edges
}

impl GradientDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Vertical => "v",
            Self::Horizontal => "h",
            Self::Diagonal => "d",
            Self::Radial => "r",
        }
    }
}

#[derive(Clone, Debug)]
pub struct BackgroundSpec {
    pub mode: BgMode,
    pub color_start: (u8, u8, u8),
    pub color_end: (u8, u8, u8),
    pub direction: GradientDirection,
}

const BAYER4: [[f32; 4]; 4] = [
    [0.0 / 16.0, 8.0 / 16.0, 2.0 / 16.0, 10.0 / 16.0],
    [12.0 / 16.0, 4.0 / 16.0, 14.0 / 16.0, 6.0 / 16.0],
    [3.0 / 16.0, 11.0 / 16.0, 1.0 / 16.0, 9.0 / 16.0],
    [15.0 / 16.0, 7.0 / 16.0, 13.0 / 16.0, 5.0 / 16.0],
];

impl BackgroundSpec {
    pub fn filename(&self) -> String {
        match self.mode {
            BgMode::Solid => {
                format!(
                    "solid_{:02x}{:02x}{:02x}.png",
                    self.color_start.0, self.color_start.1, self.color_start.2
                )
            }
            BgMode::Gradient => {
                format!(
                    "gradient_{}_{:02x}{:02x}{:02x}_{:02x}{:02x}{:02x}.png",
                    self.direction.as_str(),
                    self.color_start.0,
                    self.color_start.1,
                    self.color_start.2,
                    self.color_end.0,
                    self.color_end.1,
                    self.color_end.2
                )
            }
        }
    }

    pub fn generate_and_save(&self, target_dir: &Path) -> Result<PathBuf, String> {
        let _ = fs::create_dir_all(target_dir);
        let filename = self.filename();
        let target_path = target_dir.join(&filename);

        // If file already exists, return early
        if target_path.exists() {
            return Ok(target_path);
        }

        let width = 1920u32;
        let height = 1080u32;

        let mut img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(width, height);

        match self.mode {
            BgMode::Solid => {
                let pixel = Rgb([self.color_start.0, self.color_start.1, self.color_start.2]);
                for p in img.pixels_mut() {
                    *p = pixel;
                }
            }
            BgMode::Gradient => {
                let c1 = (
                    self.color_start.0 as f32,
                    self.color_start.1 as f32,
                    self.color_start.2 as f32,
                );
                let c2 = (
                    self.color_end.0 as f32,
                    self.color_end.1 as f32,
                    self.color_end.2 as f32,
                );

                let cx = (width as f32) / 2.0;
                let cy = (height as f32) / 2.0;
                let max_r = (cx * cx + cy * cy).sqrt();

                for y in 0..height {
                    let y_f = y as f32;
                    let y_norm = y_f / ((height - 1) as f32);

                    for x in 0..width {
                        let x_f = x as f32;
                        let x_norm = x_f / ((width - 1) as f32);

                        let t = match self.direction {
                            GradientDirection::Vertical => y_norm,
                            GradientDirection::Horizontal => x_norm,
                            GradientDirection::Diagonal => (x_norm + y_norm) * 0.5,
                            GradientDirection::Radial => {
                                let dx = x_f - cx;
                                let dy = y_f - cy;
                                ((dx * dx + dy * dy).sqrt() / max_r).clamp(0.0, 1.0)
                            }
                        };

                        let dither = (BAYER4[(y % 4) as usize][(x % 4) as usize] - 0.5) * 1.5;

                        let r = (c1.0 + (c2.0 - c1.0) * t + dither).clamp(0.0, 255.0) as u8;
                        let g = (c1.1 + (c2.1 - c1.1) * t + dither).clamp(0.0, 255.0) as u8;
                        let b = (c1.2 + (c2.2 - c1.2) * t + dither).clamp(0.0, 255.0) as u8;

                        img.put_pixel(x, y, Rgb([r, g, b]));
                    }
                }
            }
        }

        img.save(&target_path)
            .map_err(|e| format!("Failed to save background image: {e}"))?;

        Ok(target_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solid_generation() {
        let temp_dir = std::env::temp_dir().join("inerium_bg_test_solid");
        let spec = BackgroundSpec {
            mode: BgMode::Solid,
            color_start: (0x18, 0x16, 0x16),
            color_end: (0, 0, 0),
            direction: GradientDirection::Vertical,
        };
        let res = spec.generate_and_save(&temp_dir);
        assert!(res.is_ok());
        let path = res.unwrap();
        assert!(path.exists());
        assert_eq!(spec.filename(), "solid_181616.png");
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_gradient_generation() {
        let temp_dir = std::env::temp_dir().join("inerium_bg_test_grad");
        let spec = BackgroundSpec {
            mode: BgMode::Gradient,
            color_start: (0x10, 0x10, 0x12),
            color_end: (0x28, 0x28, 0x30),
            direction: GradientDirection::Diagonal,
        };
        let res = spec.generate_and_save(&temp_dir);
        assert!(res.is_ok());
        let path = res.unwrap();
        assert!(path.exists());
        assert_eq!(spec.filename(), "gradient_d_101012_282830.png");
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
