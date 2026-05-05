use keisan::{Angle, Vector2};
use sekai::EntityUid;
use std::collections::HashMap;

use crate::client_entity_manager::{ClientEntityManager, PendingTransformLerp};

#[derive(Debug, Clone, Copy, PartialEq)]
struct TransformLerp {
    source: Vector2,
    destination: Vector2,
    source_angle: Angle,
    destination_angle: Angle,
    parent: EntityUid,
    progress: f32,
}

#[derive(Debug, Clone)]
pub(crate) struct TransformSystem {
    lerps: HashMap<EntityUid, TransformLerp>,
    max_interpolation_distance: f32,
    max_interpolation_angle: f32,
}

impl Default for TransformSystem {
    fn default() -> Self {
        Self {
            lerps: HashMap::new(),
            max_interpolation_distance: 2.0,
            max_interpolation_angle: std::f32::consts::PI / 4.0,
        }
    }
}

impl TransformSystem {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn queue_snapshot_lerp(
        &mut self,
        uid: EntityUid,
        source: Vector2,
        destination: Vector2,
        source_angle: Angle,
        destination_angle: Angle,
        current_parent: EntityUid,
        destination_parent: EntityUid,
        current_anchored: bool,
        destination_anchored: bool,
    ) {
        if current_parent != destination_parent || current_anchored || destination_anchored {
            self.lerps.remove(&uid);
            return;
        }
        if (destination - source).length_squared()
            < self.max_interpolation_distance * self.max_interpolation_distance
            && (destination_angle.theta as f32 - source_angle.theta as f32).abs()
                < self.max_interpolation_angle
        {
            self.lerps.insert(
                uid,
                TransformLerp {
                    source,
                    destination,
                    source_angle,
                    destination_angle,
                    parent: destination_parent,
                    progress: 0.0,
                },
            );
        } else {
            self.lerps.remove(&uid);
        }
    }

    pub(crate) fn queue_pending_snapshot_lerps<I>(&mut self, lerps: I, skip_uid: Option<EntityUid>)
    where
        I: IntoIterator<Item = PendingTransformLerp>,
    {
        for lerp in lerps {
            if skip_uid == Some(lerp.uid) {
                continue;
            }
            self.queue_snapshot_lerp(
                lerp.uid,
                lerp.source,
                lerp.destination,
                lerp.source_angle,
                lerp.destination_angle,
                lerp.parent,
                lerp.parent,
                lerp.source_anchored,
                lerp.destination_anchored,
            );
        }
    }

    pub(crate) fn frame_update(&mut self, entities: &mut ClientEntityManager, step: f32) {
        let mut finished = Vec::new();
        for (uid, lerp) in &mut self.lerps {
            let Some(transform) = entities.inner.transforms.get(uid) else {
                finished.push(*uid);
                continue;
            };
            if transform.parent != lerp.parent {
                finished.push(*uid);
                continue;
            }
            lerp.progress = (lerp.progress + step).clamp(0.0, 1.0);
            let _ = entities.inner.set_local_transform_immediate(
                *uid,
                Vector2::lerp(lerp.source, lerp.destination, lerp.progress),
                Angle::lerp(lerp.source_angle, lerp.destination_angle, lerp.progress),
            );
            if lerp.progress >= 1.0 {
                finished.push(*uid);
            }
        }
        for uid in finished {
            self.lerps.remove(&uid);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TransformSystem;
    use crate::client_entity_manager::{ClientEntityManager, PendingTransformLerp};
    use butsuri::{AabbShape, Fixture, PhysShape};
    use keisan::{Angle, Box2, Vector2};
    use sekai::{EntityUid, MapId};

    #[test]
    fn transform_system_lerps_known_transforms() {
        let mut entities = ClientEntityManager::new();
        let uid = entities.create_entity(None, EntityUid::new(9));
        entities.inner.initialize_entity(uid);
        let mut system = TransformSystem::new();
        system.queue_snapshot_lerp(
            uid,
            Vector2::ZERO,
            Vector2::new(1.0, 0.0),
            Angle::ZERO,
            Angle::ZERO,
            EntityUid::INVALID,
            EntityUid::INVALID,
            false,
            false,
        );
        system.frame_update(&mut entities, 0.5);
        assert_eq!(
            entities
                .inner
                .transforms
                .get(&uid)
                .unwrap()
                .local_position
                .x,
            0.5
        );
        system.frame_update(&mut entities, 0.5);
        assert_eq!(
            entities
                .inner
                .transforms
                .get(&uid)
                .unwrap()
                .local_position
                .x,
            1.0
        );
    }

    #[test]
    fn transform_system_skips_lerps_for_anchored_transitions() {
        let mut entities = ClientEntityManager::new();
        let uid = entities.create_entity(None, EntityUid::new(10));
        entities.inner.initialize_entity(uid);
        let mut system = TransformSystem::new();
        system.queue_snapshot_lerp(
            uid,
            Vector2::ZERO,
            Vector2::new(1.0, 0.0),
            Angle::ZERO,
            Angle::ZERO,
            EntityUid::INVALID,
            EntityUid::INVALID,
            false,
            true,
        );
        system.frame_update(&mut entities, 0.5);
        assert_eq!(
            entities.inner.transforms.get(&uid).unwrap().local_position,
            Vector2::ZERO
        );
    }

    #[test]
    fn transform_system_can_queue_pending_snapshot_lerps_with_skip() {
        let mut entities = ClientEntityManager::new();
        let first = entities.create_entity(None, EntityUid::new(90));
        let second = entities.create_entity(None, EntityUid::new(91));
        entities.inner.initialize_entity(first);
        entities.inner.initialize_entity(second);

        let mut system = TransformSystem::new();
        system.queue_pending_snapshot_lerps(
            [
                PendingTransformLerp {
                    uid: first,
                    source: Vector2::ZERO,
                    destination: Vector2::new(1.0, 0.0),
                    source_angle: Angle::ZERO,
                    destination_angle: Angle::ZERO,
                    parent: EntityUid::INVALID,
                    source_anchored: false,
                    destination_anchored: false,
                },
                PendingTransformLerp {
                    uid: second,
                    source: Vector2::ZERO,
                    destination: Vector2::new(1.5, 0.0),
                    source_angle: Angle::ZERO,
                    destination_angle: Angle::ZERO,
                    parent: EntityUid::INVALID,
                    source_anchored: false,
                    destination_anchored: false,
                },
            ],
            Some(first),
        );

        system.frame_update(&mut entities, 1.0);
        assert_eq!(
            entities
                .inner
                .transforms
                .get(&first)
                .unwrap()
                .local_position,
            Vector2::ZERO
        );
        assert_eq!(
            entities
                .inner
                .transforms
                .get(&second)
                .unwrap()
                .local_position,
            Vector2::new(1.5, 0.0)
        );
    }

    #[test]
    fn transform_system_lerps_keep_client_spatial_runtime_in_sync() {
        let mut entities = ClientEntityManager::new();
        let map_owner = entities.ensure_map_entity(MapId::new(3));
        entities
            .inner
            .broadphases
            .insert(map_owner, sekai::BroadphaseComponent::new());
        entities.inner.ensure_physics_map(map_owner);

        let uid = entities.create_entity(None, EntityUid::new(11));
        entities.inner.initialize_entity(uid);
        let _ = entities.inner.apply_transform_state(
            uid,
            sekai::TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(3),
                grid_id: sekai::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = entities.inner.ensure_physics(uid);
        body.set_body_type(butsuri::BodyType::Dynamic);
        body.can_collide = true;
        body.awake = true;
        let _ = entities.inner.insert_fixture_and_reconcile(
            uid,
            Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );
        entities
            .inner
            .refresh_map_physics_runtime_many([MapId::new(3)]);

        let mut system = TransformSystem::new();
        system.queue_snapshot_lerp(
            uid,
            Vector2::ZERO,
            Vector2::new(1.0, 0.0),
            Angle::ZERO,
            Angle::ZERO,
            EntityUid::INVALID,
            EntityUid::INVALID,
            false,
            false,
        );
        system.frame_update(&mut entities, 1.0);

        assert_eq!(
            entities
                .inner
                .query_aabb_entities(map_owner, Box2::new(0.25, -1.0, 2.0, 1.0)),
            vec![uid]
        );
    }
}
