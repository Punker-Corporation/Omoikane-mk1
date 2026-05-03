use crate::Vector4;
use core::fmt;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Clone, Copy, Default, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const TRANSPARENT: Self = Self::from_bytes(255, 255, 255, 0);
    pub const BLACK: Self = Self::from_bytes(0, 0, 0, 255);
    pub const WHITE: Self = Self::from_bytes(255, 255, 255, 255);
    pub const RED: Self = Self::from_bytes(255, 0, 0, 255);
    pub const GREEN: Self = Self::from_bytes(0, 128, 0, 255);
    pub const BLUE: Self = Self::from_bytes(0, 0, 255, 255);
    pub const YELLOW: Self = Self::from_bytes(255, 255, 0, 255);
    pub const CYAN: Self = Self::from_bytes(0, 255, 255, 255);
    pub const MAGENTA: Self = Self::from_bytes(255, 0, 255, 255);
    pub const GRAY: Self = Self::from_bytes(128, 128, 128, 255);
    pub const ORANGE: Self = Self::from_bytes(255, 165, 0, 255);
    pub const PURPLE: Self = Self::from_bytes(128, 0, 128, 255);

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }

    pub const fn from_bytes(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::new(
            r as f32 / u8::MAX as f32,
            g as f32 / u8::MAX as f32,
            b as f32 / u8::MAX as f32,
            a as f32 / u8::MAX as f32,
        )
    }

    pub fn r_byte(self) -> u8 {
        (self.r * u8::MAX as f32) as u8
    }

    pub fn g_byte(self) -> u8 {
        (self.g * u8::MAX as f32) as u8
    }

    pub fn b_byte(self) -> u8 {
        (self.b * u8::MAX as f32) as u8
    }

    pub fn a_byte(self) -> u8 {
        (self.a * u8::MAX as f32) as u8
    }

    pub fn to_argb(self) -> i32 {
        let value = ((self.a * u8::MAX as f32) as u32) << 24
            | ((self.r * u8::MAX as f32) as u32) << 16
            | ((self.g * u8::MAX as f32) as u32) << 8
            | (self.b * u8::MAX as f32) as u32;
        value as i32
    }

    pub fn with_red(self, new_r: f32) -> Self {
        Self::new(new_r, self.g, self.b, self.a)
    }

    pub fn with_green(self, new_g: f32) -> Self {
        Self::new(self.r, new_g, self.b, self.a)
    }

    pub fn with_blue(self, new_b: f32) -> Self {
        Self::new(self.r, self.g, new_b, self.a)
    }

    pub fn with_alpha(self, new_a: f32) -> Self {
        Self::new(self.r, self.g, self.b, new_a)
    }

    pub fn from_srgb(srgb: Self) -> Self {
        fn channel(v: f32) -> f32 {
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        }

        Self::new(channel(srgb.r), channel(srgb.g), channel(srgb.b), srgb.a)
    }

    pub fn to_srgb(rgb: Self) -> Self {
        fn channel(v: f32) -> f32 {
            if v <= 0.0031308 {
                12.92 * v
            } else {
                1.055 * v.powf(1.0 / 2.4) - 0.055
            }
        }

        Self::new(channel(rgb.r), channel(rgb.g), channel(rgb.b), rgb.a)
    }

    pub fn from_hsl(hsl: Vector4) -> Self {
        let hue = hsl.x * 360.0;
        let saturation = hsl.y;
        let lightness = hsl.z;
        let c = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
        let h = hue / 60.0;
        let x = c * (1.0 - (h % 2.0 - 1.0).abs());

        let (r, g, b) = if (0.0..1.0).contains(&h) {
            (c, x, 0.0)
        } else if (1.0..2.0).contains(&h) {
            (x, c, 0.0)
        } else if (2.0..3.0).contains(&h) {
            (0.0, c, x)
        } else if (3.0..4.0).contains(&h) {
            (0.0, x, c)
        } else if (4.0..5.0).contains(&h) {
            (x, 0.0, c)
        } else if (5.0..6.0).contains(&h) {
            (c, 0.0, x)
        } else {
            (0.0, 0.0, 0.0)
        };

        let m = lightness - c / 2.0;
        Self::new(r + m, g + m, b + m, hsl.w)
    }

    pub fn to_hsl(rgb: Self) -> Vector4 {
        let max = rgb.r.max(rgb.g.max(rgb.b));
        let min = rgb.r.min(rgb.g.min(rgb.b));
        let c = max - min;

        let mut h = 0.0;
        if c != 0.0 {
            if max == rgb.r {
                h = (rgb.g - rgb.b) / c;
            } else if max == rgb.g {
                h = (rgb.b - rgb.r) / c + 2.0;
            } else {
                h = (rgb.r - rgb.g) / c + 4.0;
            }
        }

        let mut hue = h / 6.0;
        if hue < 0.0 {
            hue += 1.0;
        }

        let lightness = (max + min) / 2.0;
        let saturation = if lightness != 0.0 && lightness != 1.0 {
            c / (1.0 - (2.0 * lightness - 1.0).abs())
        } else {
            0.0
        };

        Vector4::new(hue, saturation, lightness, rgb.a)
    }

    pub fn from_hsv(hsv: Vector4) -> Self {
        let hue = hsv.x * 360.0;
        let saturation = hsv.y;
        let value = hsv.z;
        let c = value * saturation;
        let h = hue / 60.0;
        let x = c * (1.0 - (h % 2.0 - 1.0).abs());

        let (r, g, b) = if (0.0..1.0).contains(&h) {
            (c, x, 0.0)
        } else if (1.0..2.0).contains(&h) {
            (x, c, 0.0)
        } else if (2.0..3.0).contains(&h) {
            (0.0, c, x)
        } else if (3.0..4.0).contains(&h) {
            (0.0, x, c)
        } else if (4.0..5.0).contains(&h) {
            (x, 0.0, c)
        } else if (5.0..6.0).contains(&h) {
            (c, 0.0, x)
        } else {
            (0.0, 0.0, 0.0)
        };

        let m = value - c;
        Self::new(r + m, g + m, b + m, hsv.w)
    }

    pub fn to_hsv(rgb: Self) -> Vector4 {
        let max = rgb.r.max(rgb.g.max(rgb.b));
        let min = rgb.r.min(rgb.g.min(rgb.b));
        let c = max - min;

        let mut h = 0.0;
        if c != 0.0 {
            if max == rgb.r {
                h = (rgb.g - rgb.b) / c % 6.0;
            } else if max == rgb.g {
                h = (rgb.b - rgb.r) / c + 2.0;
            } else {
                h = (rgb.r - rgb.g) / c + 4.0;
            }
        }

        let hue = h * 60.0 / 360.0;
        let saturation = if max == 0.0 { 0.0 } else { c / max };
        Vector4::new(hue, saturation, max, rgb.a)
    }

    pub fn from_name(color_name: &str) -> Option<Self> {
        default_colors()
            .get(&color_name.to_ascii_lowercase())
            .copied()
    }

    pub fn name(self) -> Option<&'static str> {
        inverted_default_colors().get(&self.to_argb()).copied()
    }
}

impl fmt::Debug for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{(R, G, B, A) = ({}, {}, {}, {})}}",
            self.r, self.g, self.b, self.a
        )
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl Eq for Color {}

impl std::hash::Hash for Color {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.to_argb().hash(state);
    }
}

impl From<(f32, f32, f32, f32)> for Color {
    fn from(value: (f32, f32, f32, f32)) -> Self {
        Self::new(value.0, value.1, value.2, value.3)
    }
}

impl From<(f32, f32, f32)> for Color {
    fn from(value: (f32, f32, f32)) -> Self {
        Self::rgb(value.0, value.1, value.2)
    }
}

fn default_colors() -> &'static HashMap<String, Color> {
    static COLORS: OnceLock<HashMap<String, Color>> = OnceLock::new();
    COLORS.get_or_init(|| {
        HashMap::from([
            ("transparent".to_string(), Color::TRANSPARENT),
            ("black".to_string(), Color::BLACK),
            ("white".to_string(), Color::WHITE),
            ("red".to_string(), Color::RED),
            ("green".to_string(), Color::GREEN),
            ("blue".to_string(), Color::BLUE),
            ("yellow".to_string(), Color::YELLOW),
            ("cyan".to_string(), Color::CYAN),
            ("magenta".to_string(), Color::MAGENTA),
            ("gray".to_string(), Color::GRAY),
            ("orange".to_string(), Color::ORANGE),
            ("purple".to_string(), Color::PURPLE),
        ])
    })
}

fn inverted_default_colors() -> &'static HashMap<i32, &'static str> {
    static INVERTED: OnceLock<HashMap<i32, &'static str>> = OnceLock::new();
    INVERTED.get_or_init(|| {
        HashMap::from([
            (Color::TRANSPARENT.to_argb(), "transparent"),
            (Color::BLACK.to_argb(), "black"),
            (Color::WHITE.to_argb(), "white"),
            (Color::RED.to_argb(), "red"),
            (Color::GREEN.to_argb(), "green"),
            (Color::BLUE.to_argb(), "blue"),
            (Color::YELLOW.to_argb(), "yellow"),
            (Color::CYAN.to_argb(), "cyan"),
            (Color::MAGENTA.to_argb(), "magenta"),
            (Color::GRAY.to_argb(), "gray"),
            (Color::ORANGE.to_argb(), "orange"),
            (Color::PURPLE.to_argb(), "purple"),
        ])
    })
}

#[cfg(test)]
mod tests {
    use super::Color;
    use crate::{MathHelper, Vector4};

    #[test]
    fn srgb_roundtrip_stays_close() {
        let srgb = Color::new(0.4, 0.2, 0.8, 0.7);
        let linear = Color::from_srgb(srgb);
        let roundtrip = Color::to_srgb(linear);
        assert!(MathHelper::close_to(roundtrip.r, srgb.r, 0.0001));
        assert!(MathHelper::close_to(roundtrip.g, srgb.g, 0.0001));
        assert!(MathHelper::close_to(roundtrip.b, srgb.b, 0.0001));
        assert_eq!(roundtrip.a, srgb.a);
    }

    #[test]
    fn hsl_and_hsv_roundtrip_primary_red() {
        let red = Color::RED;
        let hsl = Color::to_hsl(red);
        let hsv = Color::to_hsv(red);
        assert_eq!(Color::from_hsl(hsl), red);
        assert_eq!(Color::from_hsv(hsv), red);
        assert_eq!(Color::from_hsl(Vector4::new(0.0, 1.0, 0.5, 1.0)), red);
        assert_eq!(Color::from_hsv(Vector4::new(0.0, 1.0, 1.0, 1.0)), red);
    }

    #[test]
    fn named_colors_are_discoverable() {
        assert_eq!(Color::from_name("Blue"), Some(Color::BLUE));
        assert_eq!(Color::BLUE.name(), Some("blue"));
    }
}
