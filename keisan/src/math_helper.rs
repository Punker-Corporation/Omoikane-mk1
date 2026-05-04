pub struct MathHelper;

impl MathHelper {
    pub const PI: f32 = core::f32::consts::PI;
    pub const PI_OVER_2: f32 = Self::PI / 2.0;
    pub const PI_OVER_3: f32 = Self::PI / 3.0;
    pub const PI_OVER_4: f32 = Self::PI / 4.0;
    pub const PI_OVER_6: f32 = Self::PI / 6.0;
    pub const TWO_PI: f32 = 2.0 * Self::PI;
    pub const THREE_PI_OVER_2: f32 = 3.0 * Self::PI / 2.0;
    pub const E: f32 = core::f32::consts::E;
    pub const LOG10_E: f32 = core::f32::consts::LOG10_E;
    pub const LOG2_E: f32 = core::f32::consts::LOG2_E;

    pub fn clamp<T>(value: T, min: T, max: T) -> T
    where
        T: PartialOrd + Copy,
    {
        if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        }
    }

    pub fn clamp01(value: f32) -> f32 {
        Self::clamp(value, 0.0, 1.0)
    }

    pub fn close_to(a: f32, b: f32, tolerance: f32) -> bool {
        (a - b).abs() <= tolerance
    }

    pub fn close_to_f64(a: f64, b: f64, tolerance: f64) -> bool {
        (a - b).abs() <= tolerance
    }

    pub fn close_to_percent(a: f32, b: f32, percentage: f64) -> bool {
        let af = a as f64;
        let bf = b as f64;
        let epsilon = (af.abs().max(bf.abs()) * percentage).max(percentage);
        (af - bf).abs() <= epsilon
    }

    pub fn close_to_percent_f64(a: f64, b: f64, percentage: f64) -> bool {
        let epsilon = (a.abs().max(b.abs()) * percentage).max(percentage);
        (a - b).abs() <= epsilon
    }

    pub fn lerp(a: f32, b: f32, blend: f32) -> f32 {
        a + (b - a) * blend
    }

    pub fn lerp_f64(a: f64, b: f64, blend: f64) -> f64 {
        a + (b - a) * blend
    }

    pub fn interpolate_cubic(pre_a: f32, a: f32, b: f32, post_b: f32, t: f32) -> f32 {
        a + 0.5
            * t
            * (b - pre_a
                + t * (2.0 * pre_a - 5.0 * a + 4.0 * b - post_b
                    + t * (3.0 * (a - b) + post_b - pre_a)))
    }

    pub fn degrees_to_radians(value: f32) -> f32 {
        value * (Self::PI / 180.0)
    }

    pub fn degrees_to_radians_f64(value: f64) -> f64 {
        value * (core::f64::consts::PI / 180.0)
    }

    pub fn radians_to_degrees(value: f32) -> f32 {
        value * (180.0 / Self::PI)
    }

    pub fn radians_to_degrees_f64(value: f64) -> f64 {
        value * (180.0 / core::f64::consts::PI)
    }

    pub fn median(a: f32, b: f32, c: f32) -> f32 {
        a.min(b).max(a.max(b).min(c))
    }
}
