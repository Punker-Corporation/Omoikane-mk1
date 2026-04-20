use crate::Vector2;
use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct UIBox2 {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl UIBox2 {
    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn from_corners(top_left: Vector2, bottom_right: Vector2) -> Self {
        Self::new(top_left.x, top_left.y, bottom_right.x, bottom_right.y)
    }

    pub fn top_left(self) -> Vector2 {
        Vector2::new(self.left, self.top)
    }

    pub fn bottom_right(self) -> Vector2 {
        Vector2::new(self.right, self.bottom)
    }

    pub fn top_right(self) -> Vector2 {
        Vector2::new(self.right, self.top)
    }

    pub fn bottom_left(self) -> Vector2 {
        Vector2::new(self.left, self.bottom)
    }

    pub fn width(self) -> f32 {
        (self.right - self.left).abs()
    }

    pub fn height(self) -> f32 {
        (self.top - self.bottom).abs()
    }

    pub fn size(self) -> Vector2 {
        Vector2::new(self.width(), self.height())
    }

    pub fn center(self) -> Vector2 {
        self.top_left() + self.size() / 2.0
    }

    pub fn from_dimensions(left: f32, top: f32, width: f32, height: f32) -> Self {
        Self::new(left, top, left + width, top + height)
    }

    pub fn from_dimensions_vec(left_top_position: Vector2, size: Vector2) -> Self {
        Self::from_dimensions(left_top_position.x, left_top_position.y, size.x, size.y)
    }

    pub fn intersects(self, other: Self) -> bool {
        other.bottom >= self.top
            && other.top <= self.bottom
            && other.right >= self.left
            && other.left <= self.right
    }

    pub fn is_empty(self) -> bool {
        crate::MathHelper::close_to_percent(self.width(), 0.0, 0.00001)
            && crate::MathHelper::close_to_percent(self.height(), 0.0, 0.00001)
    }

    pub fn encloses(self, inner: Self) -> bool {
        self.left < inner.left
            && self.bottom > inner.bottom
            && self.right > inner.right
            && self.top < inner.top
    }

    pub fn contains_xy(self, x: f32, y: f32) -> bool {
        self.contains(Vector2::new(x, y), true)
    }

    pub fn contains(self, point: Vector2, closed_region: bool) -> bool {
        let x_ok = if closed_region {
            (point.x >= self.left) ^ (point.x > self.right)
        } else {
            (point.x > self.left) ^ (point.x >= self.right)
        };
        let y_ok = if closed_region {
            (point.y >= self.top) ^ (point.y > self.bottom)
        } else {
            (point.y > self.top) ^ (point.y >= self.bottom)
        };
        x_ok && y_ok
    }

    pub fn scale(self, scalar: f32) -> Self {
        assert!(scalar >= 0.0, "scalar cannot be negative");
        let center = self.center();
        let half_size = self.size() / 2.0 * scalar;
        Self::from_corners(center - half_size, center + half_size)
    }

    pub fn translated(self, point: Vector2) -> Self {
        Self::new(
            self.left + point.x,
            self.top + point.y,
            self.right + point.x,
            self.bottom + point.y,
        )
    }
}

impl core::ops::Add<(f32, f32, f32, f32)> for UIBox2 {
    type Output = Self;

    fn add(self, rhs: (f32, f32, f32, f32)) -> Self::Output {
        Self::new(self.left + rhs.0, self.top + rhs.1, self.right + rhs.2, self.bottom + rhs.3)
    }
}

impl fmt::Display for UIBox2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {}, {})", self.left, self.top, self.right, self.bottom)
    }
}

#[cfg(test)]
mod tests {
    use super::UIBox2;
    use crate::Vector2;

    #[test]
    fn ui_box_uses_screen_coordinate_intersection() {
        let a = UIBox2::new(0.0, 0.0, 10.0, 10.0);
        let b = UIBox2::new(5.0, 5.0, 15.0, 15.0);
        assert!(a.intersects(b));
        assert!(a.contains(Vector2::new(2.0, 2.0), true));
    }
}
