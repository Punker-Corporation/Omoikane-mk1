use crate::{UIBox2, Vector2};
use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Thickness {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Thickness {
    pub const fn uniform(uniform: f32) -> Self {
        Self {
            left: uniform,
            top: uniform,
            right: uniform,
            bottom: uniform,
        }
    }

    pub const fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self {
            left: horizontal,
            right: horizontal,
            top: vertical,
            bottom: vertical,
        }
    }

    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn sum_horizontal(self) -> f32 {
        self.left + self.right
    }

    pub fn sum_vertical(self) -> f32 {
        self.top + self.bottom
    }

    pub fn inflate_box(self, box2: UIBox2) -> UIBox2 {
        UIBox2::new(
            box2.left - self.left,
            box2.top - self.top,
            box2.right + self.right,
            box2.bottom + self.bottom,
        )
    }

    pub fn inflate_size(self, size: Vector2) -> Vector2 {
        Vector2::new(size.x + self.sum_horizontal(), size.y + self.sum_vertical())
    }

    pub fn deflate_box(self, box2: UIBox2) -> UIBox2 {
        let left = box2.left + self.left;
        let top = box2.top + self.top;
        UIBox2::new(
            left,
            top,
            left.max(box2.right - self.right),
            top.max(box2.bottom - self.bottom),
        )
    }

    pub fn deflate_size(self, size: Vector2) -> Vector2 {
        Vector2::component_max(
            Vector2::ZERO,
            Vector2::new(size.x - self.sum_horizontal(), size.y - self.sum_vertical()),
        )
    }
}

impl fmt::Display for Thickness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{},{},{},{}",
            self.left, self.top, self.right, self.bottom
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Thickness;
    use crate::{UIBox2, Vector2};

    #[test]
    fn thickness_inflates_and_deflates_consistently() {
        let t = Thickness::new(1.0, 2.0, 3.0, 4.0);
        let box2 = UIBox2::new(10.0, 10.0, 20.0, 20.0);
        assert_eq!(t.inflate_box(box2), UIBox2::new(9.0, 8.0, 23.0, 24.0));
        assert_eq!(
            t.deflate_size(Vector2::new(10.0, 10.0)),
            Vector2::new(6.0, 4.0)
        );
    }
}
