use crate::ClientEntityManager;
use butsuri::BodyType;
use sekai::SharedPhysicsSystem;

#[derive(Debug, Clone, Default)]
pub struct PhysicsSystem {
    shared: SharedPhysicsSystem,
    pub prediction_enabled: bool,
}

impl PhysicsSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&self, entities: &mut ClientEntityManager, frame_time: f32) {
        let bodies: Vec<_> = entities.inner.physics.keys().copied().collect();
        for uid in bodies {
            let Some(body) = entities.inner.physics.get(&uid).cloned() else {
                continue;
            };
            if !body.awake || body.body_type == BodyType::Static {
                continue;
            }

            if let Some(transform) = entities.inner.transforms.get_mut(&uid) {
                transform.local_position = transform.local_position + body.linear_velocity * frame_time;
                transform.rebuild_for_manager();
            }
        }
    }

    pub fn frame_update(&self, entities: &mut ClientEntityManager, frame_time: f32) {
        if self.prediction_enabled {
            self.update(entities, frame_time);
        }
    }

    pub fn world_aabb(&self, entities: &ClientEntityManager, uid: sekai::EntityUid) -> Option<keisan::Box2> {
        self.shared.get_world_aabb(&entities.inner, uid)
    }
}

#[cfg(test)]
mod tests {
    use super::PhysicsSystem;
    use crate::ClientEntityManager;
    use butsuri::{AabbShape, BodyType, Fixture, PhysShape};
    use keisan::Box2;
    use sekai::EntityUid;

    #[test]
    fn physics_system_integrates_dynamic_bodies() {
        let mut entities = ClientEntityManager::new();
        let uid = entities.create_entity(None, EntityUid::new(5));
        entities.initialize_entity(uid);
        let body = entities.inner.ensure_physics(uid);
        body.set_body_type(BodyType::Dynamic);
        body.awake = true;
        body.can_collide = true;
        body.linear_velocity = keisan::Vector2::new(2.0, 0.0);
        entities
            .inner
            .ensure_fixtures(uid)
            .insert_fixture(Fixture::new("main", PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0))));
        let system = PhysicsSystem::new();
        system.update(&mut entities, 0.5);
        assert_eq!(entities.inner.transforms.get(&uid).unwrap().local_position.x, 1.0);
        assert!(system.world_aabb(&entities, uid).is_some());
    }
}
