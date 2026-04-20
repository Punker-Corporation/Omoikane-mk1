use crate::{EntityManager, EntityUid};
use keisan::{Box2, Vector2};

#[derive(Debug, Default, Clone, Copy)]
pub struct SharedPhysicsSystem;

impl SharedPhysicsSystem {
    pub fn set_linear_velocity(&self, manager: &mut EntityManager, uid: EntityUid, velocity: Vector2) -> bool {
        let Some(body) = manager.physics.get_mut(&uid) else {
            return false;
        };
        body.set_linear_velocity(velocity);
        true
    }

    pub fn get_world_aabb(&self, manager: &EntityManager, uid: EntityUid) -> Option<Box2> {
        let body = manager.physics.get(&uid)?;
        let xform = manager.transforms.get(&uid)?;
        let fixtures = manager.fixtures.get(&uid)?;
        body.get_aabb(xform, fixtures, manager)
    }

    pub fn get_hard_aabb(&self, manager: &EntityManager, uid: EntityUid) -> Option<Box2> {
        let body = manager.physics.get(&uid)?;
        let xform = manager.transforms.get(&uid)?;
        let fixtures = manager.fixtures.get(&uid)?;
        body.get_hard_aabb(xform, fixtures, manager)
    }

    pub fn sync_broadphase(&self, manager: &mut EntityManager, broadphase_owner: EntityUid, bodies: &[EntityUid]) -> usize {
        let mut pending = Vec::new();

        for uid in bodies {
            let Some(body) = manager.physics.get(uid) else {
                continue;
            };
            if !body.can_collide {
                continue;
            }

            let Some(xform) = manager.transforms.get(uid) else {
                continue;
            };
            let Some(fixtures) = manager.fixtures.get(uid) else {
                continue;
            };
            let (world_pos, world_rot, _) = xform.get_world_position_rotation_matrix(manager);
            let transform = butsuri::Transform::from_angle_type(world_pos, world_rot);
            pending.extend(fixtures.fixtures.values().cloned().map(|fixture| (fixture, transform)));
        }

        let Some(broadphase) = manager.broadphases.get_mut(&broadphase_owner) else {
            return 0;
        };

        broadphase.tree = Default::default();
        for (fixture, transform) in &pending {
            broadphase.tree.insert(fixture.clone(), *transform);
        }

        pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::SharedPhysicsSystem;
    use crate::{BroadphaseComponent, EntityManager};
    use butsuri::{AabbShape, BodyType, Fixture, PhysShape};
    use keisan::Box2;

    #[test]
    fn physics_system_computes_aabb_and_syncs_broadphase() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        manager.initialize_entity(uid);

        let body = manager.ensure_physics(uid);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);

        manager
            .ensure_fixtures(uid)
            .insert_fixture(Fixture::new("main", PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0))));

        let broadphase_uid = manager.create_entity_uninitialized(None);
        manager.broadphases.insert(broadphase_uid, BroadphaseComponent::new());

        let system = SharedPhysicsSystem;
        let aabb = system.get_world_aabb(&manager, uid).unwrap();
        assert_eq!(aabb, Box2::new(-1.0, -1.0, 1.0, 1.0));
        assert_eq!(system.sync_broadphase(&mut manager, broadphase_uid, &[uid]), 1);
    }
}
