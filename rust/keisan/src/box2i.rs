use crate::{Box2, Vector2i};
use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Box2i {
    pub left: i32,
    pub bottom: i32,
    pub right: i32,
    pub top: i32,
}

impl Box2i {
    pub const fn new(left: i32, bottom: i32, right: i32, top: i32) -> Self {
        Self {
            left,
            bottom,
            right,
            top,
        }
    }

    pub fn from_corners(bottom_left: Vector2i, top_right: Vector2i) -> Self {
        Self::new(bottom_left.x, bottom_left.y, top_right.x, top_right.y)
    }

    pub fn bottom_left(self) -> Vector2i {
        Vector2i::new(self.left, self.bottom)
    }

    pub fn top_right(self) -> Vector2i {
        Vector2i::new(self.right, self.top)
    }

    pub fn bottom_right(self) -> Vector2i {
        Vector2i::new(self.right, self.bottom)
    }

    pub fn top_left(self) -> Vector2i {
        Vector2i::new(self.left, self.top)
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

    pub fn area(self) -> i32 {
        self.width() * self.height()
    }

    pub fn from_dimensions(left: i32, bottom: i32, width: i32, height: i32) -> Self {
        Self::new(left, bottom, left + width, bottom + height)
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
            (point.y >= self.bottom) ^ (point.y > self.top)
        } else {
            (point.y > self.bottom) ^ (point.y >= self.top)
        };
        x_ok && y_ok
    }

    pub fn is_empty(self) -> bool {
        self.bottom == self.top || self.left == self.right
    }

    pub fn translated(self, point: Vector2i) -> Self {
        Self::new(
            self.left + point.x,
            self.bottom + point.y,
            self.right + point.x,
            self.top + point.y,
        )
    }

    pub fn union(self, other: Self) -> Self {
        let left = self.left.min(other.left);
        let right = self.right.max(other.right);
        let bottom = self.bottom.min(other.bottom);
        let top = self.top.max(other.top);
        if left <= right && bottom <= top {
            Self::new(left, bottom, right, top)
        } else {
            Self::default()
        }
    }

    pub fn scale(self, scalar: i32) -> Self {
        Self::new(
            self.left * scalar,
            self.bottom * scalar,
            self.right * scalar,
            self.top * scalar,
        )
    }
}

impl From<Box2> for Box2i {
    fn from(value: Box2) -> Self {
        Self::new(value.left as i32, value.bottom as i32, value.right as i32, value.top as i32)
    }
}

impl From<Box2i> for Box2 {
    fn from(value: Box2i) -> Self {
        Box2::new(value.left as f32, value.bottom as f32, value.right as f32, value.top as f32)
    }
}

impl fmt::Display for Box2i {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {}, {})", self.left, self.bottom, self.right, self.top)
    }
}

#[cfg(test)]
mod tests {
    use super::Box2i;
    use crate::Vector2i;

    #[test]
    fn integer_box_contains_and_union_work() {
        let a = Box2i::new(0, 0, 5, 5);
        assert!(a.contains(Vector2i::new(1, 1), true));
        assert_eq!(a.union(Box2i::new(3, -2, 8, 4)), Box2i::new(0, -2, 8, 5));
    }
}
