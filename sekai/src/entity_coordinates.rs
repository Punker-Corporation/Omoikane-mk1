use crate::{EntityUid, GridId, MapCoordinates, MapId};
use core::fmt;
use keisan::{Vector2, Vector2i};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformState {
    pub map_id: MapId,
    pub grid_id: GridId,
    pub world_position: Vector2,
}

pub trait EntityCoordinateResolver {
    fn entity_exists(&self, entity: EntityUid) -> bool;
    fn transform_state(&self, entity: EntityUid) -> Option<TransformState>;
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EntityCoordinates {
    pub entity_id: EntityUid,
    pub position: Vector2,
}

impl EntityCoordinates {
    pub const INVALID: Self = Self {
        entity_id: EntityUid::INVALID,
        position: Vector2::ZERO,
    };

    pub const fn new(entity_id: EntityUid, position: Vector2) -> Self {
        Self {
            entity_id,
            position,
        }
    }

    pub fn new_xy(entity_id: EntityUid, x: f32, y: f32) -> Self {
        Self::new(entity_id, Vector2::new(x, y))
    }

    pub fn x(self) -> f32 {
        self.position.x
    }

    pub fn y(self) -> f32 {
        self.position.y
    }

    pub fn is_valid<R: EntityCoordinateResolver>(self, resolver: &R) -> bool {
        self.entity_id.is_valid()
            && resolver.entity_exists(self.entity_id)
            && self.position.x.is_finite()
            && self.position.y.is_finite()
    }

    pub fn to_map<R: EntityCoordinateResolver>(self, resolver: &R) -> MapCoordinates {
        if !self.is_valid(resolver) {
            return MapCoordinates::NULLSPACE;
        }
        let transform = resolver.transform_state(self.entity_id).unwrap();
        MapCoordinates::new(transform.world_position + self.position, transform.map_id)
    }

    pub fn to_map_pos<R: EntityCoordinateResolver>(self, resolver: &R) -> Vector2 {
        self.to_map(resolver).position
    }

    pub fn with_position(self, new_position: Vector2) -> Self {
        Self::new(self.entity_id, new_position)
    }

    pub fn with_entity_id(self, entity_id: EntityUid) -> Self {
        Self::new(entity_id, self.position)
    }

    pub fn get_grid_id<R: EntityCoordinateResolver>(self, resolver: &R) -> GridId {
        resolver
            .transform_state(self.entity_id)
            .map(|x| x.grid_id)
            .unwrap_or(GridId::INVALID)
    }

    pub fn get_map_id<R: EntityCoordinateResolver>(self, resolver: &R) -> MapId {
        resolver
            .transform_state(self.entity_id)
            .map(|x| x.map_id)
            .unwrap_or(MapId::NULLSPACE)
    }

    pub fn offset(self, position: Vector2) -> Self {
        Self::new(self.entity_id, self.position + position)
    }

    pub fn in_range<R: EntityCoordinateResolver>(
        self,
        resolver: &R,
        other: Self,
        range: f32,
    ) -> bool {
        if !self.is_valid(resolver) || !other.is_valid(resolver) {
            return false;
        }
        if self.entity_id == other.entity_id {
            return (other.position - self.position).length_squared() < range * range;
        }
        self.to_map(resolver)
            .in_range(other.to_map(resolver), range)
    }

    pub fn try_distance<R: EntityCoordinateResolver>(
        self,
        resolver: &R,
        other: Self,
    ) -> Option<f32> {
        if !self.is_valid(resolver) || !other.is_valid(resolver) {
            return None;
        }
        if self.entity_id == other.entity_id {
            return Some((self.position - other.position).length());
        }
        let map_a = self.to_map(resolver);
        let map_b = other.to_map(resolver);
        if map_a.map_id != map_b.map_id {
            return None;
        }
        Some((map_a.position - map_b.position).length())
    }

    pub fn to_vector2i<R: EntityCoordinateResolver>(self, resolver: &R) -> Vector2i {
        let pos = self.to_map_pos(resolver);
        Vector2i::new(pos.x.floor() as i32, pos.y.floor() as i32)
    }
}

impl core::ops::Add for EntityCoordinates {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        assert!(
            self.entity_id == rhs.entity_id,
            "Can't sum EntityCoordinates with different relative entities."
        );
        Self::new(self.entity_id, self.position + rhs.position)
    }
}

impl core::ops::Sub for EntityCoordinates {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        assert!(
            self.entity_id == rhs.entity_id,
            "Can't subtract EntityCoordinates with different relative entities."
        );
        Self::new(self.entity_id, self.position - rhs.position)
    }
}

impl core::ops::Mul for EntityCoordinates {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        assert!(
            self.entity_id == rhs.entity_id,
            "Can't multiply EntityCoordinates with different relative entities."
        );
        Self::new(self.entity_id, self.position * rhs.position)
    }
}

impl core::ops::Mul<f32> for EntityCoordinates {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.entity_id, self.position * rhs)
    }
}

impl core::ops::Mul<i32> for EntityCoordinates {
    type Output = Self;

    fn mul(self, rhs: i32) -> Self::Output {
        Self::new(self.entity_id, self.position * rhs as f32)
    }
}

impl fmt::Display for EntityCoordinates {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EntId={}, X={:.2}, Y={:.2}",
            self.entity_id, self.position.x, self.position.y
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{EntityCoordinateResolver, EntityCoordinates, TransformState};
    use crate::{EntityUid, GridId, MapId};
    use keisan::Vector2;

    struct FakeResolver;

    impl EntityCoordinateResolver for FakeResolver {
        fn entity_exists(&self, entity: EntityUid) -> bool {
            entity.is_valid()
        }

        fn transform_state(&self, entity: EntityUid) -> Option<TransformState> {
            if entity.is_valid() {
                Some(TransformState {
                    map_id: MapId::new(1),
                    grid_id: GridId::new(2),
                    world_position: Vector2::new(10.0, 20.0),
                })
            } else {
                None
            }
        }
    }

    #[test]
    fn entity_coordinates_resolve_into_map_space() {
        let resolver = FakeResolver;
        let coords = EntityCoordinates::new(EntityUid::new(3), Vector2::new(1.0, 2.0));
        let map = coords.to_map(&resolver);
        assert_eq!(map.position, Vector2::new(11.0, 22.0));
        assert_eq!(coords.get_grid_id(&resolver), GridId::new(2));
    }
}
