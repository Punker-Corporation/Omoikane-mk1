use crate::{Component, EntityCoordinates, EntityUid, GridId, MapCoordinates, MapId};
use keisan::{Angle, Matrix3, Vector2};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub trait TransformResolver {
    fn world_transform(&self, entity: EntityUid) -> Option<WorldTransform>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldTransform {
    pub map_id: MapId,
    pub grid_id: GridId,
    pub world_position: Vector2,
    pub world_rotation: Angle,
    pub world_matrix: Matrix3,
    pub inv_world_matrix: Matrix3,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TransformComponentState {
    pub local_position: Vector2,
    pub rotation: Angle,
    pub parent_id: EntityUid,
    pub map_id: MapId,
    pub grid_id: GridId,
    pub no_local_rotation: bool,
    pub anchored: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveEvent {
    pub sender: EntityUid,
    pub old_position: EntityCoordinates,
    pub new_position: EntityCoordinates,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RotateEvent {
    pub sender: EntityUid,
    pub old_rotation: Angle,
    pub new_rotation: Angle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnchorStateChangedEvent {
    pub entity: EntityUid,
    pub anchored: bool,
}

#[derive(Debug, Clone)]
pub struct TransformComponent {
    pub base: Component,
    pub parent: EntityUid,
    pub local_position: Vector2,
    pub local_rotation: Angle,
    pub no_local_rotation: bool,
    pub anchored: bool,
    pub local_matrix: Matrix3,
    pub inv_local_matrix: Matrix3,
    pub children: HashSet<EntityUid>,
    pub map_id: MapId,
    pub grid_id: GridId,
}

impl TransformComponent {
    pub fn new() -> Self {
        Self {
            base: Component::new("TransformComponent"),
            parent: EntityUid::INVALID,
            local_position: Vector2::ZERO,
            local_rotation: Angle::ZERO,
            no_local_rotation: false,
            anchored: false,
            local_matrix: Matrix3::IDENTITY,
            inv_local_matrix: Matrix3::IDENTITY,
            children: HashSet::new(),
            map_id: MapId::NULLSPACE,
            grid_id: GridId::INVALID,
        }
    }

    pub fn updates_deferred(&self) -> bool {
        false
    }

    pub fn world_rotation<R: TransformResolver>(&self, resolver: &R) -> Angle {
        let mut parent = self.parent;
        let mut rotation = self.local_rotation;
        while parent.is_valid() {
            let Some(parent_xform) = resolver.world_transform(parent) else {
                break;
            };
            rotation = rotation + parent_xform.world_rotation;
            parent = EntityUid::INVALID;
        }
        rotation
    }

    pub fn world_position<R: TransformResolver>(&self, resolver: &R) -> Vector2 {
        if self.parent.is_valid() {
            if let Some(parent) = resolver.world_transform(self.parent) {
                return parent.world_matrix * self.local_position;
            }
        }
        self.local_position
    }

    pub fn coordinates(&self) -> EntityCoordinates {
        let valid = self.parent.is_valid();
        EntityCoordinates::new(
            if valid { self.parent } else { self.base.owner },
            if valid {
                self.local_position
            } else {
                Vector2::ZERO
            },
        )
    }

    pub fn map_position<R: TransformResolver>(&self, resolver: &R) -> MapCoordinates {
        MapCoordinates::new(self.world_position(resolver), self.map_id)
    }

    pub fn set_local_rotation(&mut self, value: Angle) -> Option<RotateEvent> {
        if self.no_local_rotation || self.local_rotation == value {
            return None;
        }
        let old = self.local_rotation;
        self.local_rotation = value;
        self.rebuild_matrices();
        Some(RotateEvent {
            sender: self.base.owner,
            old_rotation: old,
            new_rotation: value,
        })
    }

    pub fn set_local_position(&mut self, value: Vector2) -> Option<MoveEvent> {
        if self.anchored || self.local_position == value {
            return None;
        }
        let old = self.coordinates();
        self.local_position = value;
        self.rebuild_matrices();
        Some(MoveEvent {
            sender: self.base.owner,
            old_position: old,
            new_position: self.coordinates(),
        })
    }

    pub fn set_parent(&mut self, value: EntityUid) {
        self.parent = value;
        self.rebuild_matrices();
    }

    pub fn set_anchored(&mut self, value: bool) -> AnchorStateChangedEvent {
        self.anchored = value;
        AnchorStateChangedEvent {
            entity: self.base.owner,
            anchored: value,
        }
    }

    pub fn get_component_state(&self) -> TransformComponentState {
        TransformComponentState {
            local_position: self.local_position,
            rotation: self.local_rotation,
            parent_id: self.parent,
            map_id: self.map_id,
            grid_id: self.grid_id,
            no_local_rotation: self.no_local_rotation,
            anchored: self.anchored,
        }
    }

    pub fn handle_transform_state(&mut self, state: TransformComponentState) {
        self.parent = state.parent_id;
        self.local_position = state.local_position;
        self.local_rotation = state.rotation;
        self.map_id = state.map_id;
        self.grid_id = state.grid_id;
        self.no_local_rotation = state.no_local_rotation;
        self.anchored = state.anchored;
        self.rebuild_matrices();
    }

    pub fn get_world_position_rotation_matrix<R: TransformResolver>(
        &self,
        resolver: &R,
    ) -> (Vector2, Angle, Matrix3) {
        if self.parent.is_valid() {
            if let Some(parent) = resolver.world_transform(self.parent) {
                let matrix = self.local_matrix * parent.world_matrix;
                let position = Vector2::new(matrix.r0c2, matrix.r1c2);
                return (
                    position,
                    self.local_rotation + parent.world_rotation,
                    matrix,
                );
            }
        }
        (self.local_position, self.local_rotation, self.local_matrix)
    }

    fn rebuild_matrices(&mut self) {
        let pos = self.local_position;
        let rot = self.local_rotation.theta as f32;
        self.local_matrix = Matrix3::create_transform(pos.x, pos.y, rot, 1.0, 1.0);
        self.inv_local_matrix = Matrix3::create_inverse_transform(pos.x, pos.y, rot, 1.0, 1.0);
    }

    pub fn rebuild_for_manager(&mut self) {
        self.rebuild_matrices();
    }
}

#[cfg(test)]
mod tests {
    use super::{TransformComponent, TransformResolver, WorldTransform};
    use crate::{EntityUid, GridId, MapId};
    use keisan::{Angle, Matrix3, Vector2};

    struct FakeResolver;

    impl TransformResolver for FakeResolver {
        fn world_transform(&self, entity: EntityUid) -> Option<WorldTransform> {
            if entity == EntityUid::new(10) {
                let world_matrix = Matrix3::create_transform(5.0, 6.0, 0.0, 1.0, 1.0);
                Some(WorldTransform {
                    map_id: MapId::new(1),
                    grid_id: GridId::new(2),
                    world_position: Vector2::new(5.0, 6.0),
                    world_rotation: Angle::ZERO,
                    world_matrix,
                    inv_world_matrix: world_matrix.inverted(),
                })
            } else {
                None
            }
        }
    }

    #[test]
    fn transform_component_tracks_local_and_parented_motion() {
        let resolver = FakeResolver;
        let mut xform = TransformComponent::new();
        xform.base.owner = EntityUid::new(20);
        xform.set_parent(EntityUid::new(10));
        xform.set_local_position(Vector2::new(1.0, 2.0));
        assert_eq!(xform.world_position(&resolver), Vector2::new(6.0, 8.0));
        let rotate = xform.set_local_rotation(Angle::from_degrees(45.0)).unwrap();
        assert_eq!(rotate.new_rotation, Angle::from_degrees(45.0));
    }
}
