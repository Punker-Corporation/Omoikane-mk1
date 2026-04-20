use crate::{UIBox2, Vector2i};
use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UIBox2i {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl UIBox2i {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn from_corners(top_left: Vector2i, bottom_right: Vector2i) -> Self {
        Self::new(top_left.x, top_left.y, bottom_right.x, bottom_right.y)
    }

    pub fn top_left(self) -> Vector2i {
        Vector2i::new(self.left, self.top)
    }

    pub fn bottom_right(self) -> Vector2i {
        Vector2i::new(self.right, self.bottom)
    }

    pub fn top_right(self) -> Vector2i {
        Vector2i::new(self.right, self.top)
    }

    pub fn bottom_left(self) -> Vector2i {
        Vector2i::new(self.left, self.bottom)
    }

    pub fn width(self) -> i32 {
        (self.right - self.left).abs()
    }

    pub fn height(self) -> i32 {
        (self.top - self.bottom).abs()
    }

    pub fn size(self) -> Vector2i {
        Vector2i::new(self.width(), self.height())
    }

    pub fn from_dimensions(left: i32, top: i32, width: i32, height: i32) -> Self {
        Self::new(left, top, left + width, top + height)
    }

    pub fn from_dimensions_vec(position: Vector2i, size: Vector2i) -> Self {
        Self::from_dimensions(position.x, position.y, size.x, size.y)
    }

    pub fn contains_xy(self, x: i32, y: i32) -> bool {
        self.contains(Vector2i::new(x, y), true)
    }

    pub fn contains(self, point: Vector2i, closed_region: bool) -> bool {
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

    pub fn translated(self, point: Vector2i) -> Self {
        Self::new(
            self.left + point.x,
            self.top + point.y,
            self.right + point.x,
            self.bottom + point.y,
        )
    }

    pub fn intersection(self, other: Self) -> Option<Self> {
        if !self.intersects(other) {
            return None;
        }
        Some(Self::from_corners(
            Vector2i::component_max(self.top_left(), other.top_left()),
            Vector2i::component_min(self.bottom_right(), other.bottom_right()),
        ))
    }

    pub fn intersects(self, other: Self) -> bool {
        other.bottom >= self.top
            && other.top <= self.bottom
            && other.right >= self.left
            && other.left <= self.right
    }
}

impl From<UIBox2> for UIBox2i {
    fn from(value: UIBox2) -> Self {
        Self::new(value.left as i32, value.top as i32, value.right as i32, value.bottom as i32)
    }
}

impl From<UIBox2i> for UIBox2 {
    fn from(value: UIBox2i) -> Self {
        UIBox2::new(value.left as f32, value.top as f32, value.right as f32, value.bottom as f32)
    }
}

impl core::ops::Add<(i32, i32, i32, i32)> for UIBox2i {
    type Output = Self;

    fn add(self, rhs: (i32, i32, i32, i32)) -> Self::Output {
        Self::new(self.left + rhs.0, self.top + rhs.1, self.right + rhs.2, self.bottom + rhs.3)
    }
}

impl fmt::Display for UIBox2i {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {}, {})", self.left, self.top, self.right, self.bottom)
    }
}

#[cfg(test)]
mod tests {
    use super::UIBox2i;

    #[test]
    fn ui_integer_intersection_returns_overlap_region() {
        let a = UIBox2i::new(0, 0, 10, 10);
        let b = UIBox2i::new(5, 5, 12, 12);
        assert_eq!(a.intersection(b), Some(UIBox2i::new(5, 5, 10, 10)));
    }
}
