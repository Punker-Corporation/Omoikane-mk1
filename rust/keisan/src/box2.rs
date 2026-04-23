use crate::{ApproxEq, MathHelper, Vector2};
use core::fmt;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Box2 {
    pub left: f32,
    pub bottom: f32,
    pub right: f32,
    pub top: f32,
}

impl Box2 {
    pub const UNIT_CENTERED: Self = Self::new(-0.5, -0.5, 0.5, 0.5);

    pub const fn new(left: f32, bottom: f32, right: f32, top: f32) -> Self {
        Self {
            left,
            bottom,
            right,
            top,
        }
    }

    pub fn from_corners(bottom_left: Vector2, top_right: Vector2) -> Self {
        Self::new(bottom_left.x, bottom_left.y, top_right.x, top_right.y)
    }

    pub fn bottom_left(self) -> Vector2 {
        Vector2::new(self.left, self.bottom)
    }

    pub fn top_right(self) -> Vector2 {
        Vector2::new(self.right, self.top)
    }

    pub fn bottom_right(self) -> Vector2 {
        Vector2::new(self.right, self.bottom)
    }

    pub fn top_left(self) -> Vector2 {
        Vector2::new(self.left, self.top)
    }

    pub fn width(self) -> f32 {
        (self.right - self.left).abs()
    }

    pub fn height(self) -> f32 {
        (self.bottom - self.top).abs()
    }

    pub fn size(self) -> Vector2 {
        Vector2::new(self.width(), self.height())
    }

    pub fn center(self) -> Vector2 {
        self.bottom_left() + self.size() * 0.5
    }

    pub fn extents(self) -> Vector2 {
        (self.top_right() - self.bottom_left()) * 0.5
    }

    pub fn from_dimensions(left: f32, bottom: f32, width: f32, height: f32) -> Self {
        Self::new(left, bottom, left + width, bottom + height)
    }

    pub fn from_dimensions_vec(bottom_left: Vector2, size: Vector2) -> Self {
        Self::from_dimensions(bottom_left.x, bottom_left.y, size.x, size.y)
    }

    pub fn centered_around(center: Vector2, size: Vector2) -> Self {
        Self::from_dimensions_vec(center - size / 2.0, size)
    }

    pub fn centred_around_zero(size: Vector2) -> Self {
        Self::from_dimensions_vec(-size / 2.0, size)
    }

    pub fn intersects(self, other: Self) -> bool {
        other.bottom <= self.top
            && other.top >= self.bottom
            && other.right >= self.left
            && other.left <= self.right
    }

    pub fn enlarged(self, size: f32) -> Self {
        Self::new(self.left - size, self.bottom - size, self.right + size, self.top + size)
    }

    pub fn intersect(self, other: Self) -> Self {
        let max = Vector2::component_max(self.bottom_left(), other.bottom_left());
        let min = Vector2::component_min(self.top_right(), other.top_right());
        if max.x <= min.x && max.y <= min.y {
            Self::new(max.x, max.y, min.x, min.y)
        } else {
            Self::default()
        }
    }

    pub fn intersect_percentage(self, other: Self) -> f32 {
        let surface_intersect = Self::area(self.intersect(other));
        surface_intersect / (Self::area(self) + Self::area(other) - surface_intersect)
    }

    pub fn union(self, other: Self) -> Self {
        let left_bottom = Vector2::component_min(self.bottom_left(), other.bottom_left());
        let right_top = Vector2::component_max(self.top_right(), other.top_right());
        if left_bottom.x <= right_top.x && left_bottom.y <= right_top.y {
            Self::new(left_bottom.x, left_bottom.y, right_top.x, right_top.y)
        } else {
            Self::default()
        }
    }

    pub fn is_empty(self) -> bool {
        MathHelper::close_to_percent(self.width(), 0.0, 0.00001)
            && MathHelper::close_to_percent(self.height(), 0.0, 0.00001)
    }

    pub fn encloses(self, inner: Self) -> bool {
        self.left < inner.left
            && self.bottom < inner.bottom
            && self.right > inner.right
            && self.top > inner.top
    }

    pub fn contains_box(self, inner: Self) -> bool {
        self.left <= inner.left
            && self.bottom <= inner.bottom
            && self.right >= inner.right
            && self.top >= inner.top
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
            (point.y >= self.bottom) ^ (point.y > self.top)
        } else {
            (point.y > self.bottom) ^ (point.y >= self.top)
        };
        x_ok && y_ok
    }

    pub fn scale(self, scalar: f32) -> Self {
        assert!(scalar >= 0.0, "scalar cannot be negative");
        let center = self.center();
        let half_size = self.size() / 2.0 * scalar;
        Self::from_corners(center - half_size, center + half_size)
    }

    pub fn scale_non_uniform(self, scale: Vector2) -> Self {
        let center = self.center();
        let half_size = (self.size() / 2.0) * scale;
        Self::from_corners(center - half_size, center + half_size)
    }

    pub fn translated(self, point: Vector2) -> Self {
        Self::new(
            self.left + point.x,
            self.bottom + point.y,
            self.right + point.x,
            self.top + point.y,
        )
    }

    pub fn area(box2: Self) -> f32 {
        box2.width() * box2.height()
    }

    pub fn perimeter(box2: Self) -> f32 {
        (box2.width() + box2.height()) * 2.0
    }

    pub fn union_points(a: Vector2, b: Vector2) -> Self {
        let min = Vector2::component_min(a, b);
        let max = Vector2::component_max(a, b);
        Self::new(min.x, min.y, max.x, max.y)
    }

    pub fn extend_to_contain(self, vec: Vector2) -> Self {
        let min = Vector2::component_min(vec, self.bottom_left());
        let max = Vector2::component_max(vec, self.top_right());
        Self::new(min.x, min.y, max.x, max.y)
    }

    pub fn closest_point(self, position: Vector2) -> Vector2 {
        Vector2::new(
            MathHelper::clamp(position.x, self.left, self.right),
            MathHelper::clamp(position.y, self.bottom, self.top),
        )
    }
}

impl ApproxEq for Box2 {
    fn approx_eq(&self, other: Self) -> bool {
        MathHelper::close_to_percent(self.left, other.left, 0.00001)
            && MathHelper::close_to_percent(self.bottom, other.bottom, 0.00001)
            && MathHelper::close_to_percent(self.right, other.right, 0.00001)
            && MathHelper::close_to_percent(self.top, other.top, 0.00001)
    }

    fn approx_eq_with_tolerance(&self, other: Self, tolerance: f64) -> bool {
        MathHelper::close_to_percent(self.left, other.left, tolerance)
            && MathHelper::close_to_percent(self.bottom, other.bottom, tolerance)
            && MathHelper::close_to_percent(self.right, other.right, tolerance)
            && MathHelper::close_to_percent(self.top, other.top, tolerance)
    }
}

impl fmt::Debug for Box2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {}, {})", self.left, self.bottom, self.right, self.top)
    }
}

impl fmt::Display for Box2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {}, {})", self.left, self.bottom, self.right, self.top)
    }
}

#[cfg(test)]
mod tests {
    use super::Box2;
    use crate::Vector2;

    #[test]
    fn intersection_and_union_behave_like_aabb_ops() {
        let a = Box2::new(0.0, 0.0, 10.0, 10.0);
        let b = Box2::new(5.0, 5.0, 15.0, 15.0);
        assert!(a.intersects(b));
        assert_eq!(a.intersect(b), Box2::new(5.0, 5.0, 10.0, 10.0));
        assert_eq!(a.union(b), Box2::new(0.0, 0.0, 15.0, 15.0));
        assert_eq!(a.closest_point(Vector2::new(20.0, -3.0)), Vector2::new(10.0, 0.0));
    }
}
