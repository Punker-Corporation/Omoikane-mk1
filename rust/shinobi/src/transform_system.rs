use keisan::{Angle, Vector2};
use sekai::{EntityUid, SharedTransformSystem};
use std::collections::HashMap;

use crate::ClientEntityManager;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformLerp {
    pub source: Vector2,
    pub destination: Vector2,
    pub source_angle: Angle,
    pub destination_angle: Angle,
    pub parent: EntityUid,
}

#[derive(Debug, Clone)]
pub struct TransformSystem {
    shared: SharedTransformSystem,
    lerps: HashMap<EntityUid, TransformLerp>,
    pub max_interpolation_distance: f32,
    pub max_interpolation_angle: f32,
}

impl Default for TransformSystem {
    fn default() -> Self {
        Self {
            shared: SharedTransformSystem::new(),
            lerps: HashMap::new(),
            max_interpolation_distance: 2.0,
            max_interpolation_angle: std::f32::consts::PI / 4.0,
        }
    }
}

impl TransformSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn note_transform_state(
        &mut self,
        entities: &ClientEntityManager,
        uid: EntityUid,
        destination: Vector2,
        destination_angle: Angle,
        parent: EntityUid,
    ) {
        let Some(transform) = entities.inner.transforms.get(&uid) else {
            return;
        };
        if transform.parent != parent {
            self.lerps.remove(&uid);
            return;
        }
        let source = transform.local_position;
        let source_angle = transform.local_rotation;
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
                    parent,
                },
            );
        } else {
            self.lerps.remove(&uid);
        }
    }

    pub fn frame_update(&mut self, entities: &mut ClientEntityManager, step: f32) {
        let mut finished = Vec::new();
        for (uid, lerp) in &self.lerps {
            let Some(transform) = entities.inner.transforms.get_mut(uid) else {
                finished.push(*uid);
                continue;
            };
            if transform.parent != lerp.parent {
                finished.push(*uid);
                continue;
            }
            transform.local_position = Vector2::lerp(lerp.source, lerp.destination, step);
            transform.local_rotation = Angle::lerp(lerp.source_angle, lerp.destination_angle, step);
            transform.rebuild_for_manager();
            if step >= 1.0 {
                finished.push(*uid);
            }
        }
        for uid in finished {
            self.lerps.remove(&uid);
        }
    }

    pub fn world_position(&self, entities: &ClientEntityManager, uid: EntityUid) -> Option<Vector2> {
        self.shared.get_world_position(&entities.inner, uid)
    }
}

#[cfg(test)]
mod tests {
    use super::TransformSystem;
    use crate::ClientEntityManager;
    use keisan::{Angle, Vector2};
    use sekai::EntityUid;

    #[test]
    fn transform_system_lerps_known_transforms() {
        let mut entities = ClientEntityManager::new();
        let uid = entities.create_entity(None, EntityUid::new(9));
        entities.initialize_entity(uid);
        let mut system = TransformSystem::new();
        system.note_transform_state(&entities, uid, Vector2::new(1.0, 0.0), Angle::ZERO, EntityUid::INVALID);
        system.frame_update(&mut entities, 0.5);
        assert_eq!(entities.inner.transforms.get(&uid).unwrap().local_position.x, 0.5);
    }
}
