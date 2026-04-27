use crate::{ServerEntityManager, TransformSystem};
use sekai::{
    BodyType, CollisionChangeMessage, EntityUid, MapId, MapManager, PhysicsRuntimeEvent,
    PhysicsSleepMessage, PhysicsWakeMessage,
};
#[derive(Debug, Clone, Default)]
pub struct PhysicsSystem;

impl PhysicsSystem {
    pub fn new() -> Self {
        Self::default()
    }

    fn mark_body_changed(&self, entities: &mut ServerEntityManager, uid: EntityUid) {
        let tick = entities.inner.current_tick;
        if let Some(body) = entities.inner.physics.get_mut(&uid) {
            body.base.last_modified_tick = tick;
        }
        entities.inner.dirty_entity(uid);
    }

    fn refresh_body_runtime(
        &self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        runtime_event: Option<PhysicsRuntimeEvent>,
    ) {
        if let Some(runtime_event) = runtime_event {
            let _ = entities
                .inner
                .queue_entity_runtime_event(uid, runtime_event);
        }
        entities.inner.refresh_entity_physics_runtime(uid);
    }

    fn wake_sleep_event(&self, uid: EntityUid, awake: bool) -> PhysicsRuntimeEvent {
        if awake {
            PhysicsRuntimeEvent::Wake(PhysicsWakeMessage { body: uid })
        } else {
            PhysicsRuntimeEvent::Sleep(PhysicsSleepMessage { body: uid })
        }
    }

    pub fn handle_grid_init(&self, entities: &mut ServerEntityManager, uid: EntityUid) -> bool {
        if !entities.inner.entity_exists(uid) {
            return false;
        }
        let tick = entities.inner.current_tick;
        let body = entities.inner.ensure_physics(uid);
        body.set_body_type(BodyType::Static);
        body.base.last_modified_tick = tick;
        entities.inner.dirty_entity(uid);
        let _ = self.set_can_collide(entities, uid, true);
        true
    }

    pub fn handle_dynamic_init(&self, entities: &mut ServerEntityManager, uid: EntityUid) -> bool {
        if !entities.inner.entity_exists(uid) {
            return false;
        }
        let tick = entities.inner.current_tick;
        let body = entities.inner.ensure_physics(uid);
        body.set_body_type(BodyType::Dynamic);
        body.base.last_modified_tick = tick;
        entities.inner.dirty_entity(uid);
        let _ = self.set_can_collide(entities, uid, true);
        let _ = self.set_awake(entities, uid, true);
        true
    }

    pub fn set_body_type(
        &self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        body_type: BodyType,
    ) -> bool {
        let tick = entities.inner.current_tick;
        let Some(body) = entities.inner.physics.get_mut(&uid) else {
            return false;
        };
        if body.body_type == body_type {
            return false;
        }
        body.set_body_type(body_type);
        let _ = tick;
        self.mark_body_changed(entities, uid);
        self.refresh_body_runtime(entities, uid, None);
        true
    }

    pub fn set_linear_velocity(
        &self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        velocity: keisan::Vector2,
    ) -> bool {
        let tick = entities.inner.current_tick;
        let Some(body) = entities.inner.physics.get_mut(&uid) else {
            return false;
        };
        let previous = body.linear_velocity;
        let previous_awake = body.awake;
        body.set_linear_velocity(velocity);
        if body.linear_velocity == previous {
            return false;
        }
        let awake_changed = body.awake != previous_awake;
        let _ = tick;
        self.mark_body_changed(entities, uid);
        if awake_changed {
            self.refresh_body_runtime(entities, uid, None);
        }
        true
    }

    pub fn set_angular_velocity(
        &self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        velocity: f32,
    ) -> bool {
        let tick = entities.inner.current_tick;
        let Some(body) = entities.inner.physics.get_mut(&uid) else {
            return false;
        };
        let previous = body.angular_velocity;
        let previous_awake = body.awake;
        body.set_angular_velocity(velocity);
        if body.angular_velocity == previous {
            return false;
        }
        let awake_changed = body.awake != previous_awake;
        let _ = tick;
        self.mark_body_changed(entities, uid);
        if awake_changed {
            self.refresh_body_runtime(entities, uid, None);
        }
        true
    }

    pub fn set_sleeping_allowed(
        &self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        sleeping_allowed: bool,
    ) -> bool {
        let tick = entities.inner.current_tick;
        let Some(body) = entities.inner.physics.get_mut(&uid) else {
            return false;
        };
        let previous_sleeping_allowed = body.sleeping_allowed;
        let previous_awake = body.awake;
        body.set_sleeping_allowed(sleeping_allowed);
        if body.sleeping_allowed == previous_sleeping_allowed {
            return false;
        }
        let awake_changed = body.awake != previous_awake;
        let _ = tick;
        self.mark_body_changed(entities, uid);
        if awake_changed {
            self.refresh_body_runtime(entities, uid, None);
        }
        true
    }

    pub fn set_fixed_rotation(
        &self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        fixed_rotation: bool,
    ) -> bool {
        let tick = entities.inner.current_tick;
        let Some(body) = entities.inner.physics.get_mut(&uid) else {
            return false;
        };
        if body.fixed_rotation == fixed_rotation {
            return false;
        }
        body.set_fixed_rotation(fixed_rotation);
        let _ = tick;
        self.mark_body_changed(entities, uid);
        true
    }

    pub fn set_body_status(
        &self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        status: sekai::BodyStatus,
    ) -> bool {
        let tick = entities.inner.current_tick;
        let Some(body) = entities.inner.physics.get_mut(&uid) else {
            return false;
        };
        if body.body_status == status {
            return false;
        }
        body.set_body_status(status);
        let _ = tick;
        self.mark_body_changed(entities, uid);
        true
    }

    pub fn set_ignore_gravity(
        &self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        ignore_gravity: bool,
    ) -> bool {
        let Some(body) = entities.inner.physics.get_mut(&uid) else {
            return false;
        };
        if body.ignore_gravity == ignore_gravity {
            return false;
        }
        body.set_ignore_gravity(ignore_gravity);
        true
    }

    pub fn set_linear_damping(
        &self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        damping: f32,
    ) -> bool {
        let Some(body) = entities.inner.physics.get_mut(&uid) else {
            return false;
        };
        let previous = body.linear_damping;
        body.set_linear_damping(damping);
        body.linear_damping != previous
    }

    pub fn set_angular_damping(
        &self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        damping: f32,
    ) -> bool {
        let Some(body) = entities.inner.physics.get_mut(&uid) else {
            return false;
        };
        let previous = body.angular_damping;
        body.set_angular_damping(damping);
        body.angular_damping != previous
    }

    pub fn apply_force(
        &self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        force: keisan::Vector2,
    ) -> bool {
        let tick = entities.inner.current_tick;
        let Some(body) = entities.inner.physics.get_mut(&uid) else {
            return false;
        };
        let previous_force = body.force;
        let previous_awake = body.awake;
        body.apply_force(force);
        if body.force == previous_force {
            return false;
        }
        let awake_changed = body.awake != previous_awake;
        if awake_changed {
            let _ = tick;
            self.mark_body_changed(entities, uid);
            self.refresh_body_runtime(entities, uid, None);
        }
        true
    }

    pub fn apply_torque(
        &self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        torque: f32,
    ) -> bool {
        let tick = entities.inner.current_tick;
        let Some(body) = entities.inner.physics.get_mut(&uid) else {
            return false;
        };
        let previous_torque = body.torque;
        let previous_awake = body.awake;
        body.apply_torque(torque);
        if body.torque == previous_torque {
            return false;
        }
        let awake_changed = body.awake != previous_awake;
        if awake_changed {
            let _ = tick;
            self.mark_body_changed(entities, uid);
            self.refresh_body_runtime(entities, uid, None);
        }
        true
    }

    pub fn apply_linear_impulse(
        &self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        impulse: keisan::Vector2,
    ) -> bool {
        let tick = entities.inner.current_tick;
        let Some(body) = entities.inner.physics.get_mut(&uid) else {
            return false;
        };
        let previous_velocity = body.linear_velocity;
        let previous_awake = body.awake;
        body.apply_linear_impulse(impulse);
        if body.linear_velocity == previous_velocity {
            return false;
        }
        let awake_changed = body.awake != previous_awake;
        let _ = tick;
        self.mark_body_changed(entities, uid);
        if awake_changed {
            self.refresh_body_runtime(entities, uid, None);
        }
        true
    }

    pub fn apply_angular_impulse(
        &self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        impulse: f32,
    ) -> bool {
        let tick = entities.inner.current_tick;
        let Some(body) = entities.inner.physics.get_mut(&uid) else {
            return false;
        };
        let previous_velocity = body.angular_velocity;
        let previous_awake = body.awake;
        body.apply_angular_impulse(impulse);
        if body.angular_velocity == previous_velocity {
            return false;
        }
        let awake_changed = body.awake != previous_awake;
        let _ = tick;
        self.mark_body_changed(entities, uid);
        if awake_changed {
            self.refresh_body_runtime(entities, uid, None);
        }
        true
    }

    pub fn set_map_gravity(
        &self,
        entities: &mut ServerEntityManager,
        maps: &MapManager,
        map_id: MapId,
        gravity: keisan::Vector2,
    ) -> bool {
        let owner = maps.get_map_entity_id(map_id);
        if !owner.is_valid() || !entities.inner.entity_exists(owner) {
            return false;
        }
        entities.inner.set_map_gravity(map_id, gravity)
    }

    pub fn set_auto_clear_forces(
        &self,
        entities: &mut ServerEntityManager,
        maps: &MapManager,
        map_id: MapId,
        enabled: bool,
    ) -> bool {
        let owner = maps.get_map_entity_id(map_id);
        if !owner.is_valid() || !entities.inner.entity_exists(owner) {
            return false;
        }
        entities.inner.set_map_auto_clear_forces(map_id, enabled)
    }

    pub fn set_awake(
        &self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        awake: bool,
    ) -> bool {
        let tick = entities.inner.current_tick;
        let Some(body) = entities.inner.physics.get_mut(&uid) else {
            return false;
        };
        let previous = body.awake;
        body.set_awake(awake);
        if body.awake == previous {
            return false;
        }
        let _ = tick;
        self.mark_body_changed(entities, uid);
        self.refresh_body_runtime(entities, uid, Some(self.wake_sleep_event(uid, awake)));
        true
    }

    pub fn set_can_collide(
        &self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        can_collide: bool,
    ) -> bool {
        let tick = entities.inner.current_tick;
        let Some(body) = entities.inner.physics.get_mut(&uid) else {
            return false;
        };
        if body.can_collide == can_collide {
            return false;
        }
        body.can_collide = can_collide;
        let _ = tick;
        self.mark_body_changed(entities, uid);
        self.refresh_body_runtime(
            entities,
            uid,
            Some(PhysicsRuntimeEvent::CollisionChange(
                CollisionChangeMessage {
                    owner: uid,
                    can_collide,
                },
            )),
        );
        true
    }

    pub fn update(&self, entities: &mut ServerEntityManager, broadphase_owner: EntityUid) -> usize {
        let bodies = entities.inner.physics_body_entities(true);
        entities
            .inner
            .refresh_broadphase_runtime(broadphase_owner, &bodies)
    }

    pub fn step_simulation(
        &self,
        entities: &mut ServerEntityManager,
        transforms: &mut TransformSystem,
        frame_time: f32,
    ) -> usize {
        let bodies = entities.inner.physics_body_entities(true);
        let mut moved = 0;

        for uid in bodies {
            let Some(step) = entities.inner.step_physics_body(uid, frame_time) else {
                continue;
            };
            if step.state_changed {
                self.mark_body_changed(entities, uid);
            }
            if step.awake_changed {
                self.refresh_body_runtime(
                    entities,
                    uid,
                    Some(self.wake_sleep_event(uid, step.awake)),
                );
            }

            if step.linear_velocity == keisan::Vector2::ZERO && step.angular_velocity == 0.0 {
                continue;
            }

            if transforms.offset_local_transform(
                entities,
                uid,
                step.linear_velocity * frame_time,
                keisan::Angle::new((step.angular_velocity * frame_time) as f64),
            ) {
                moved += 1;
            }
        }

        moved
    }

    pub fn sync_map_physics(&self, entities: &mut ServerEntityManager, maps: &MapManager) -> usize {
        let mut total = 0;
        let mut refreshed_maps = Vec::new();

        for map_id in maps.get_all_map_ids() {
            let owner = maps.get_map_entity_id(map_id);
            if !owner.is_valid() || !entities.inner.entity_exists(owner) {
                continue;
            }

            let bodies = entities.inner.physics_bodies_in_map(map_id, true);
            total += bodies.len();
            refreshed_maps.push(map_id);
        }

        entities
            .inner
            .refresh_map_physics_runtime_many(refreshed_maps);

        total
    }
}

#[cfg(test)]
mod tests {
    use super::PhysicsSystem;
    use crate::{ServerEntityManager, TransformSystem};
    use butsuri::{AabbShape, BodyType, CircleShape, CollisionRay, Fixture, PhysShape};
    use keisan::{Box2, Vector2};
    use sekai::{BroadphaseComponent, MapId, MapManager};

    fn configure_dynamic_body(entities: &mut ServerEntityManager, uid: sekai::EntityUid) {
        assert!(entities.inner.configure_physics_body(
            uid,
            Some(BodyType::Dynamic),
            None,
            Some(true),
            None,
        ));
    }

    fn configure_awake_dynamic_body(entities: &mut ServerEntityManager, uid: sekai::EntityUid) {
        assert!(entities.inner.configure_physics_body(
            uid,
            Some(BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
    }

    fn mutate_body<F>(entities: &mut ServerEntityManager, uid: sekai::EntityUid, mutate: F)
    where
        F: FnOnce(&mut sekai::PhysicsComponent),
    {
        assert!(entities.inner.mutate_physics_and_reconcile(uid, mutate));
    }

    #[test]
    fn physics_system_initializes_grid_bodies_and_syncs_broadphase() {
        let mut entities = ServerEntityManager::new();
        let uid = entities.inner.create_entity_uninitialized(None);
        let broadphase_uid = entities.inner.create_entity_uninitialized(None);
        entities
            .inner
            .broadphases
            .insert(broadphase_uid, BroadphaseComponent::new());
        let system = PhysicsSystem::new();
        assert!(system.handle_grid_init(&mut entities, uid));
        entities.inner.current_tick = jikan::GameTick::new(2);
        configure_dynamic_body(&mut entities, uid);
        entities
            .inner
            .ensure_fixtures(uid)
            .insert_fixture(Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ));
        assert_eq!(system.update(&mut entities, broadphase_uid), 1);
        assert_eq!(
            entities
                .inner
                .query_aabb_entities(broadphase_uid, Box2::new(-2.0, -2.0, 2.0, 2.0)),
            vec![uid]
        );
        assert_eq!(
            entities.inner.intersect_ray_on(
                broadphase_uid,
                CollisionRay::new(Vector2::new(-5.0, 0.0), Vector2::UNIT_X, -1),
                10.0,
                true,
            )[0]
            .entity,
            uid
        );
        assert!(system.set_linear_velocity(&mut entities, uid, Vector2::new(2.0, 0.0)));
        assert_eq!(
            entities
                .inner
                .physics
                .get(&uid)
                .unwrap()
                .base
                .last_modified_tick,
            jikan::GameTick::new(2)
        );
    }

    #[test]
    fn physics_system_steps_dynamic_bodies_into_transform_positions() {
        let mut entities = ServerEntityManager::new();
        let uid = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(uid);
        entities.inner.current_tick = jikan::GameTick::new(2);
        let system = PhysicsSystem::new();
        assert!(system.handle_dynamic_init(&mut entities, uid));
        assert!(system.set_linear_velocity(&mut entities, uid, Vector2::new(2.0, 0.0)));

        let mut transforms = TransformSystem::new();
        assert_eq!(
            system.step_simulation(&mut entities, &mut transforms, 0.5),
            1
        );
        assert_eq!(
            entities.inner.transforms.get(&uid).unwrap().local_position,
            Vector2::new(0.9, 0.0)
        );
    }

    #[test]
    fn physics_system_can_toggle_awake_and_collision_flags() {
        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(40)));
        let uid = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(uid);
        let _ = entities.inner.apply_transform_state(
            uid,
            sekai::TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: keisan::Angle::ZERO,
                parent_id: sekai::EntityUid::INVALID,
                map_id,
                grid_id: sekai::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let system = PhysicsSystem::new();
        assert!(system.handle_dynamic_init(&mut entities, uid));
        let _ = entities.inner.drain_map_runtime_events(map_id);
        assert!(system.set_awake(&mut entities, uid, false));
        assert!(!entities.inner.physics.get(&uid).unwrap().awake);
        assert_eq!(
            entities.inner.drain_map_runtime_events(map_id),
            vec![sekai::PhysicsRuntimeEvent::Sleep(
                sekai::PhysicsSleepMessage { body: uid }
            )]
        );
        assert!(system.set_can_collide(&mut entities, uid, false));
        assert!(!entities.inner.physics.get(&uid).unwrap().can_collide);
        assert_eq!(
            entities.inner.drain_map_runtime_events(map_id),
            vec![sekai::PhysicsRuntimeEvent::CollisionChange(
                sekai::CollisionChangeMessage {
                    owner: uid,
                    can_collide: false,
                }
            )]
        );
    }

    #[test]
    fn physics_system_can_toggle_sleeping_allowed_fixed_rotation_and_body_status() {
        let mut entities = ServerEntityManager::new();
        let uid = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(uid);
        let system = PhysicsSystem::new();
        assert!(system.handle_dynamic_init(&mut entities, uid));
        assert!(system.set_awake(&mut entities, uid, false));
        assert!(system.set_sleeping_allowed(&mut entities, uid, false));
        let body = entities.inner.physics.get(&uid).unwrap();
        assert!(body.awake);
        assert!(!body.sleeping_allowed);

        {
            let body = entities.inner.physics.get_mut(&uid).unwrap();
            body.angular_velocity = 3.0;
            body.torque = 2.0;
        }
        assert!(system.set_fixed_rotation(&mut entities, uid, true));
        let body = entities.inner.physics.get(&uid).unwrap();
        assert!(body.fixed_rotation);
        assert_eq!(body.angular_velocity, 0.0);
        assert_eq!(body.torque, 0.0);

        assert!(system.set_body_status(&mut entities, uid, sekai::BodyStatus::InAir));
        assert_eq!(
            entities.inner.physics.get(&uid).unwrap().body_status,
            sekai::BodyStatus::InAir
        );
    }

    #[test]
    fn physics_system_set_angular_velocity_wakes_dynamic_body() {
        let mut entities = ServerEntityManager::new();
        let uid = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(uid);
        let system = PhysicsSystem::new();
        assert!(system.handle_dynamic_init(&mut entities, uid));
        assert!(system.set_awake(&mut entities, uid, false));
        assert!(system.set_angular_velocity(&mut entities, uid, 1.5));
        let body = entities.inner.physics.get(&uid).unwrap();
        assert_eq!(body.angular_velocity, 1.5);
        assert!(body.awake);
    }

    #[test]
    fn physics_system_applies_force_and_impulses_authoritatively() {
        let mut entities = ServerEntityManager::new();
        let uid = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(uid);
        let system = PhysicsSystem::new();
        assert!(system.handle_dynamic_init(&mut entities, uid));
        assert!(system.set_awake(&mut entities, uid, false));

        assert!(system.apply_force(&mut entities, uid, Vector2::new(2.0, 1.0)));
        {
            let body = entities.inner.physics.get(&uid).unwrap();
            assert_eq!(body.force, Vector2::new(2.0, 1.0));
            assert!(body.awake);
        }

        assert!(system.apply_torque(&mut entities, uid, 1.5));
        assert_eq!(entities.inner.physics.get(&uid).unwrap().torque, 1.5);

        assert!(system.apply_linear_impulse(&mut entities, uid, Vector2::new(1.0, 0.0)));
        assert!(system.apply_angular_impulse(&mut entities, uid, 2.5));
        let body = entities.inner.physics.get(&uid).unwrap();
        assert_eq!(body.linear_velocity, Vector2::new(1.0, 0.0));
        assert_eq!(body.angular_velocity, 2.5);
    }

    #[test]
    fn physics_system_step_simulation_respects_ignore_gravity_and_damping() {
        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(38)));
        let system = PhysicsSystem::new();
        assert!(system.set_map_gravity(&mut entities, &maps, map_id, Vector2::new(0.0, -10.0)));

        let uid = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(uid);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(uid, |transform| {
                    transform.map_id = map_id;
                })
        );
        assert!(entities.inner.configure_physics_body(
            uid,
            Some(BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        assert!(entities.inner.mutate_physics_and_reconcile(uid, |body| {
            body.linear_velocity = Vector2::new(4.0, 0.0);
            body.angular_velocity = 4.0;
        }));
        assert!(system.set_ignore_gravity(&mut entities, uid, true));
        assert!(system.set_linear_damping(&mut entities, uid, 0.5));
        assert!(system.set_angular_damping(&mut entities, uid, 0.5));

        let mut transforms = TransformSystem::new();
        assert_eq!(
            system.step_simulation(&mut entities, &mut transforms, 0.5),
            1
        );
        let body = entities.inner.physics.get(&uid).unwrap();
        assert_eq!(body.linear_velocity, Vector2::new(3.0, 0.0));
        assert_eq!(body.angular_velocity, 3.0);
    }

    #[test]
    fn physics_system_gets_map_velocities_from_parent_chain() {
        let mut entities = ServerEntityManager::new();

        let parent = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(parent);
        let _ = entities.inner.apply_transform_state(
            parent,
            sekai::TransformComponentState {
                local_position: Vector2::new(5.0, 0.0),
                rotation: keisan::Angle::from_degrees(90.0),
                parent_id: sekai::EntityUid::INVALID,
                map_id: MapId::new(39),
                grid_id: sekai::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(entities.inner.configure_physics_body(
            parent,
            Some(BodyType::Dynamic),
            None,
            None,
            None,
        ));
        assert!(entities.inner.mutate_physics_and_reconcile(parent, |body| {
            body.linear_velocity = Vector2::new(2.0, 0.0);
            body.angular_velocity = 1.0;
        }));

        let child = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(child);
        let _ = entities.inner.apply_transform_state(
            child,
            sekai::TransformComponentState {
                local_position: Vector2::new(1.0, 0.0),
                rotation: keisan::Angle::ZERO,
                parent_id: parent,
                map_id: MapId::new(39),
                grid_id: sekai::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(entities.inner.configure_physics_body(
            child,
            Some(BodyType::Dynamic),
            None,
            None,
            None,
        ));
        assert!(entities.inner.mutate_physics_and_reconcile(child, |body| {
            body.linear_velocity = Vector2::new(1.0, 0.0);
            body.angular_velocity = 2.0;
        }));

        let linear = entities.inner.entity_map_linear_velocity(child).unwrap();
        assert!((linear.x - 2.0).abs() < 0.0001);
        assert!(linear.y.abs() < 0.0001);
        assert_eq!(entities.inner.entity_map_angular_velocity(child), Some(3.0));
    }

    #[test]
    fn physics_system_force_and_torque_do_not_dirty_network_state_without_awake_change() {
        let mut entities = ServerEntityManager::new();
        let uid = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(uid);
        entities.inner.current_tick = jikan::GameTick::new(12);
        let system = PhysicsSystem::new();
        assert!(system.handle_dynamic_init(&mut entities, uid));
        {
            let body = entities.inner.physics.get_mut(&uid).unwrap();
            body.base.last_modified_tick = jikan::GameTick::new(4);
        }
        entities
            .inner
            .metadata
            .get_mut(&uid)
            .unwrap()
            .entity_last_modified_tick = jikan::GameTick::new(4);

        assert!(system.apply_force(&mut entities, uid, Vector2::new(2.0, 0.0)));
        assert!(system.apply_torque(&mut entities, uid, 3.0));
        assert_eq!(
            entities
                .inner
                .physics
                .get(&uid)
                .unwrap()
                .base
                .last_modified_tick,
            jikan::GameTick::new(4)
        );
        assert_eq!(
            entities
                .inner
                .metadata
                .get(&uid)
                .unwrap()
                .entity_last_modified_tick,
            jikan::GameTick::new(4)
        );
    }

    #[test]
    fn physics_system_updates_awake_set_immediately_when_awake_changes() {
        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(20)));
        let map_owner = maps.get_map_entity_id(map_id);
        entities.inner.ensure_broadphase(map_owner);
        entities.inner.ensure_physics_map(map_owner);

        let uid = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(uid);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(uid, |transform| {
                    transform.map_id = map_id;
                })
        );
        assert!(entities.inner.configure_physics_body(
            uid,
            Some(BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        let _ = entities.inner.insert_fixture_and_reconcile(
            uid,
            Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        entities.inner.refresh_map_physics_runtime(map_id);
        assert!(entities.inner.map_contains_awake_body(map_id, uid));

        let system = PhysicsSystem::new();
        assert!(system.set_awake(&mut entities, uid, false));
        assert!(!entities.inner.map_contains_awake_body(map_id, uid));
        assert_eq!(
            entities
                .inner
                .map_physics_runtime_last_modified(map_id)
                .unwrap(),
            entities.inner.current_tick
        );
        assert!(system.set_awake(&mut entities, uid, true));
        assert!(entities.inner.map_contains_awake_body(map_id, uid));
    }

    #[test]
    fn physics_system_updates_awake_set_immediately_when_linear_velocity_wakes_body() {
        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(23)));
        let map_owner = maps.get_map_entity_id(map_id);
        entities.inner.ensure_broadphase(map_owner);
        entities.inner.ensure_physics_map(map_owner);

        let uid = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(uid);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(uid, |transform| {
                    transform.map_id = map_id;
                })
        );
        assert!(entities.inner.configure_physics_body(
            uid,
            Some(BodyType::Dynamic),
            Some(false),
            Some(true),
            None,
        ));
        let _ = entities.inner.insert_fixture_and_reconcile(
            uid,
            Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        entities.inner.refresh_map_physics_runtime(map_id);
        assert!(!entities.inner.map_contains_awake_body(map_id, uid));

        let system = PhysicsSystem::new();
        assert!(system.set_linear_velocity(&mut entities, uid, Vector2::new(1.0, 0.0)));
        assert!(entities.inner.physics.get(&uid).unwrap().awake);
        assert!(entities.inner.map_contains_awake_body(map_id, uid));
        assert_eq!(
            entities
                .inner
                .map_physics_runtime_last_modified(map_id)
                .unwrap(),
            entities.inner.current_tick
        );
    }

    #[test]
    fn physics_system_updates_contacts_immediately_when_collision_flags_change() {
        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(21)));
        let map_owner = maps.get_map_entity_id(map_id);
        entities.inner.ensure_broadphase(map_owner);
        entities.inner.ensure_physics_map(map_owner);

        let first = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(first);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(first, |transform| {
                    transform.map_id = map_id;
                })
        );
        configure_awake_dynamic_body(&mut entities, first);
        let _ = entities.inner.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "first",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        let second = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(second);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(second, |transform| {
                    transform.map_id = map_id;
                    transform.local_position = Vector2::new(0.5, 0.0);
                })
        );
        configure_awake_dynamic_body(&mut entities, second);
        let _ = entities.inner.insert_fixture_and_reconcile(
            second,
            Fixture::new(
                "second",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        entities.inner.refresh_map_physics_runtime(map_id);
        let system = PhysicsSystem::new();
        assert_eq!(entities.inner.map_contact_count(map_id), 1);
        let _ = entities.inner.drain_map_contact_events(map_id);

        assert!(system.set_can_collide(&mut entities, second, false));
        assert_eq!(entities.inner.map_contact_count(map_id), 0);
        assert_eq!(
            entities.inner.map_broadphase_last_modified(map_id).unwrap(),
            entities.inner.current_tick
        );
        let events = entities.inner.drain_map_contact_events(map_id);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, butsuri::ContactStatus::EndTouching);

        assert!(system.set_can_collide(&mut entities, second, true));
        assert_eq!(entities.inner.map_contact_count(map_id), 1);
        let events = entities.inner.drain_map_contact_events(map_id);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, butsuri::ContactStatus::StartTouching);
    }

    #[test]
    fn physics_system_updates_joint_filtered_contacts_immediately_when_body_type_changes() {
        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(22)));
        let map_owner = maps.get_map_entity_id(map_id);
        entities.inner.ensure_broadphase(map_owner);
        entities.inner.ensure_physics_map(map_owner);

        let first = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(first);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(first, |transform| {
                    transform.map_id = map_id;
                })
        );
        configure_awake_dynamic_body(&mut entities, first);
        let _ = entities.inner.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "first",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        let second = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(second);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(second, |transform| {
                    transform.map_id = map_id;
                    transform.local_position = Vector2::new(0.5, 0.0);
                })
        );
        configure_awake_dynamic_body(&mut entities, second);
        let _ = entities.inner.insert_fixture_and_reconcile(
            second,
            Fixture::new(
                "second",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        let mut joint =
            butsuri::Joint::new(first.raw(), second.raw(), butsuri::JointType::Distance);
        joint.id = "rope".to_string();
        joint.collide_connected = false;
        assert!(entities.inner.add_joint_between(joint));

        entities.inner.refresh_map_physics_runtime(map_id);
        let system = PhysicsSystem::new();
        assert_eq!(entities.inner.map_contact_count(map_id), 0);
        let _ = entities.inner.drain_map_contact_events(map_id);

        assert!(system.set_body_type(&mut entities, second, BodyType::Static));
        assert_eq!(entities.inner.map_contact_count(map_id), 1);
        let events = entities.inner.drain_map_contact_events(map_id);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, butsuri::ContactStatus::StartTouching);

        assert!(system.set_body_type(&mut entities, second, BodyType::Dynamic));
        assert_eq!(entities.inner.map_contact_count(map_id), 0);
        let events = entities.inner.drain_map_contact_events(map_id);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, butsuri::ContactStatus::EndTouching);
    }

    #[test]
    fn physics_system_syncs_broadphase_and_physics_map_per_map() {
        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(2)));
        let map_owner = maps.get_map_entity_id(map_id);

        let uid = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(uid);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(uid, |transform| {
                    transform.map_id = map_id;
                })
        );
        configure_awake_dynamic_body(&mut entities, uid);
        entities
            .inner
            .ensure_fixtures(uid)
            .insert_fixture(Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ));

        let system = PhysicsSystem::new();
        assert_eq!(system.sync_map_physics(&mut entities, &maps), 1);
        assert_eq!(
            entities
                .inner
                .query_aabb_entities(map_owner, Box2::new(-2.0, -2.0, 2.0, 2.0)),
            vec![uid]
        );
        assert!(entities.inner.map_contains_body(map_id, uid));
        assert!(entities.inner.map_contains_awake_body(map_id, uid));
        assert_eq!(entities.inner.map_contact_count(map_id), 0);
    }

    #[test]
    fn physics_system_sync_map_physics_creates_runtime_components_on_demand() {
        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(24)));
        let map_owner = maps.get_map_entity_id(map_id);
        assert!(!entities.inner.has_map_broadphase(map_id));
        assert!(!entities.inner.has_map_physics_runtime(map_id));

        let uid = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(uid);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(uid, |transform| {
                    transform.map_id = map_id;
                })
        );
        let body = entities.inner.ensure_physics(uid);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        body.awake = true;
        let _ = entities.inner.insert_fixture_and_reconcile(
            uid,
            Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        let system = PhysicsSystem::new();
        assert_eq!(system.sync_map_physics(&mut entities, &maps), 1);
        assert!(entities.inner.has_map_broadphase(map_id));
        assert!(entities.inner.has_map_physics_runtime(map_id));
        assert_eq!(
            entities
                .inner
                .query_aabb_entities(map_owner, Box2::new(-1.0, -1.0, 1.0, 1.0)),
            vec![uid]
        );
        assert!(entities.inner.map_contains_body(map_id, uid));
        assert!(entities.inner.map_contains_awake_body(map_id, uid));
    }

    #[test]
    fn physics_system_step_simulation_applies_gravity_and_auto_clears_forces() {
        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(25)));
        let system = PhysicsSystem::new();
        assert!(system.set_map_gravity(&mut entities, &maps, map_id, Vector2::new(0.0, -10.0)));
        assert!(system.set_auto_clear_forces(&mut entities, &maps, map_id, true));

        let uid = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(uid);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(uid, |transform| {
                    transform.map_id = map_id;
                })
        );
        configure_awake_dynamic_body(&mut entities, uid);
        mutate_body(&mut entities, uid, |body| {
            body.force = Vector2::new(2.0, 0.0);
            body.torque = 4.0;
        });
        let _ = entities.inner.insert_fixture_and_reconcile(
            uid,
            Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        entities.inner.refresh_map_physics_runtime(map_id);
        assert!(entities.inner.has_map_physics_runtime(map_id));

        let mut transforms = TransformSystem::new();
        assert_eq!(
            system.step_simulation(&mut entities, &mut transforms, 0.5),
            1
        );
        let body = entities.inner.physics.get(&uid).unwrap();
        assert_eq!(body.linear_velocity, Vector2::new(0.9, -4.5));
        assert_eq!(body.angular_velocity, 1.8);
        assert_eq!(body.force, Vector2::ZERO);
        assert_eq!(body.torque, 0.0);
        let transform = entities.inner.transforms.get(&uid).unwrap();
        assert_eq!(transform.local_position, Vector2::new(0.45, -2.25));
        assert!((transform.local_rotation.theta - 0.9).abs() < 0.0001);
    }

    #[test]
    fn physics_system_step_simulation_puts_idle_sleeping_allowed_bodies_to_sleep_and_refreshes_runtime()
     {
        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(28)));
        let map_owner = maps.get_map_entity_id(map_id);
        entities.inner.ensure_broadphase(map_owner);
        entities.inner.ensure_physics_map(map_owner);

        let uid = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(uid);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(uid, |transform| {
                    transform.map_id = map_id;
                })
        );
        configure_awake_dynamic_body(&mut entities, uid);
        mutate_body(&mut entities, uid, |body| body.sleeping_allowed = true);
        let _ = entities.inner.insert_fixture_and_reconcile(
            uid,
            Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        entities.inner.refresh_map_physics_runtime(map_id);
        assert!(entities.inner.map_contains_awake_body(map_id, uid));

        let system = PhysicsSystem::new();
        let mut transforms = TransformSystem::new();
        assert_eq!(
            system.step_simulation(&mut entities, &mut transforms, 0.6),
            0
        );
        let body = entities.inner.physics.get(&uid).unwrap();
        assert!(!body.awake);
        assert!(!entities.inner.map_contains_awake_body(map_id, uid));
        assert_eq!(
            entities
                .inner
                .map_physics_runtime_last_modified(map_id)
                .unwrap(),
            entities.inner.current_tick
        );
    }

    #[test]
    fn physics_system_step_simulation_keeps_idle_body_awake_when_sleeping_is_disabled() {
        let mut entities = ServerEntityManager::new();
        let uid = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(uid);
        assert!(entities.inner.configure_physics_body(
            uid,
            Some(BodyType::Dynamic),
            Some(true),
            None,
            None,
        ));
        mutate_body(&mut entities, uid, |body| body.sleeping_allowed = false);

        let system = PhysicsSystem::new();
        let mut transforms = TransformSystem::new();
        assert_eq!(
            system.step_simulation(&mut entities, &mut transforms, 0.6),
            0
        );
        let body = entities.inner.physics.get(&uid).unwrap();
        assert!(body.awake);
        assert_eq!(body.sleep_time, 0.0);
    }

    #[test]
    fn physics_system_step_simulation_does_not_dirty_idle_awake_bodies_without_physical_change() {
        let mut entities = ServerEntityManager::new();
        let uid = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(uid);
        entities.inner.current_tick = jikan::GameTick::new(9);
        assert!(entities.inner.configure_physics_body(
            uid,
            Some(BodyType::Dynamic),
            Some(true),
            None,
            None,
        ));
        {
            let body = entities.inner.physics.get_mut(&uid).unwrap();
            body.sleeping_allowed = false;
            body.base.last_modified_tick = jikan::GameTick::new(4);
        }
        entities
            .inner
            .metadata
            .get_mut(&uid)
            .unwrap()
            .entity_last_modified_tick = jikan::GameTick::new(4);

        let system = PhysicsSystem::new();
        let mut transforms = TransformSystem::new();
        assert_eq!(
            system.step_simulation(&mut entities, &mut transforms, 0.1),
            0
        );
        assert_eq!(
            entities
                .inner
                .physics
                .get(&uid)
                .unwrap()
                .base
                .last_modified_tick,
            jikan::GameTick::new(4)
        );
        assert_eq!(
            entities
                .inner
                .metadata
                .get(&uid)
                .unwrap()
                .entity_last_modified_tick,
            jikan::GameTick::new(4)
        );
    }

    #[test]
    fn physics_system_syncs_runtime_contacts_per_map() {
        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(3)));
        let first = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(first);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(first, |transform| {
                    transform.map_id = map_id;
                })
        );
        configure_dynamic_body(&mut entities, first);
        let _ = entities.inner.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        );

        let second = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(second);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(second, |transform| {
                    transform.map_id = map_id;
                })
        );
        configure_dynamic_body(&mut entities, second);
        let _ = entities.inner.insert_fixture_and_reconcile(
            second,
            Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        let system = PhysicsSystem::new();
        let _ = system.sync_map_physics(&mut entities, &maps);

        let contacts = entities.inner.map_contacts_snapshot(map_id);
        assert_eq!(contacts.len(), 1);
        let first_id = format!("{}:main", first.raw());
        let second_id = format!("{}:main", second.raw());
        assert!(contacts.iter().any(|contact| {
            (contact.fixture_a == first_id && contact.fixture_b == second_id)
                || (contact.fixture_a == second_id && contact.fixture_b == first_id)
        }));
        assert_eq!(contacts[0].manifold.points.len(), 1);
        assert_eq!(contacts[0].manifold.normal, Vector2::UNIT_X);
        let events = entities.inner.drain_map_contact_events(map_id);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, butsuri::ContactStatus::StartTouching);
    }

    #[test]
    fn physics_system_syncs_mixed_contact_manifolds_per_map() {
        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(4)));
        let first = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(first);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(first, |transform| {
                    transform.map_id = map_id;
                })
        );
        configure_dynamic_body(&mut entities, first);
        let _ = entities.inner.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "aabb",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        );

        let second = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(second);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(second, |transform| {
                    transform.map_id = map_id;
                    transform.local_position = Vector2::new(1.5, 0.0);
                })
        );
        configure_dynamic_body(&mut entities, second);
        let _ = entities.inner.insert_fixture_and_reconcile(
            second,
            Fixture::new(
                "circle",
                PhysShape::Circle(CircleShape::new(Vector2::ZERO, 1.0)),
            ),
        );

        let system = PhysicsSystem::new();
        let _ = system.sync_map_physics(&mut entities, &maps);

        let contacts = entities.inner.map_contacts_snapshot(map_id);
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].contact_type, butsuri::ContactType::Mixed);
        assert_eq!(contacts[0].manifold.points.len(), 1);
        assert!(contacts[0].manifold.normal.x.abs() > 0.9);
    }

    #[test]
    fn physics_system_intersect_ray_first_hit_returns_closest_entity() {
        let mut entities = ServerEntityManager::new();
        let map_owner = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(map_owner);
        entities
            .inner
            .broadphases
            .insert(map_owner, BroadphaseComponent::new());

        let far = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(far);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(far, |transform| {
                    transform.local_position = Vector2::new(8.0, 0.0);
                })
        );
        configure_dynamic_body(&mut entities, far);
        let _ = entities.inner.insert_fixture_and_reconcile(
            far,
            Fixture::new(
                "far",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        );

        let near = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(near);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(near, |transform| {
                    transform.local_position = Vector2::new(5.0, 0.0);
                })
        );
        configure_dynamic_body(&mut entities, near);
        let _ = entities.inner.insert_fixture_and_reconcile(
            near,
            Fixture::new(
                "near",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        );

        let system = PhysicsSystem::new();
        assert_eq!(system.update(&mut entities, map_owner), 2);
        let hits = entities.inner.intersect_ray_on(
            map_owner,
            CollisionRay::new(Vector2::ZERO, Vector2::UNIT_X, -1),
            10.0,
            true,
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entity, near);
        assert_eq!(hits[0].fixture_id, "near");
    }

    #[test]
    fn physics_system_intersect_ray_ignores_circle_aabb_false_positives() {
        let mut entities = ServerEntityManager::new();
        let map_owner = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(map_owner);
        entities
            .inner
            .broadphases
            .insert(map_owner, BroadphaseComponent::new());

        let uid = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(uid);
        configure_dynamic_body(&mut entities, uid);
        let _ = entities.inner.insert_fixture_and_reconcile(
            uid,
            Fixture::new(
                "circle",
                PhysShape::Circle(CircleShape::new(Vector2::ZERO, 1.0)),
            ),
        );

        let system = PhysicsSystem::new();
        assert_eq!(system.update(&mut entities, map_owner), 1);
        let hits = entities.inner.intersect_ray_on(
            map_owner,
            CollisionRay::new(Vector2::new(-2.0, 1.1), Vector2::UNIT_X, -1),
            10.0,
            false,
        );
        assert!(hits.is_empty());
    }

    #[test]
    fn physics_system_sync_map_physics_ignores_circle_aabb_false_contacts() {
        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(26)));

        let first = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(first);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(first, |transform| {
                    transform.map_id = map_id;
                })
        );
        configure_dynamic_body(&mut entities, first);
        let _ = entities.inner.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "box",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        );

        let second = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(second);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(second, |transform| {
                    transform.map_id = map_id;
                    transform.local_position = Vector2::new(1.4, 1.4);
                })
        );
        configure_dynamic_body(&mut entities, second);
        let _ = entities.inner.insert_fixture_and_reconcile(
            second,
            Fixture::new(
                "circle",
                PhysShape::Circle(CircleShape::new(Vector2::ZERO, 0.5)),
            ),
        );

        let system = PhysicsSystem::new();
        assert_eq!(system.sync_map_physics(&mut entities, &maps), 2);
        assert_eq!(entities.inner.map_contact_count(map_id), 0);
    }

    #[test]
    fn physics_system_sync_map_physics_ignores_rotated_aabb_false_contacts() {
        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(27)));

        let first = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(first);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(first, |transform| {
                    transform.map_id = map_id;
                    transform.local_rotation = keisan::Angle::from_degrees(45.0);
                })
        );
        configure_dynamic_body(&mut entities, first);
        let _ = entities.inner.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "first",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -0.25, 1.0, 0.25), 0.0)),
            ),
        );

        let second = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(second);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(second, |transform| {
                    transform.map_id = map_id;
                    transform.local_position = Vector2::new(-1.7, -1.7);
                    transform.local_rotation = keisan::Angle::from_degrees(45.0);
                })
        );
        configure_dynamic_body(&mut entities, second);
        let _ = entities.inner.insert_fixture_and_reconcile(
            second,
            Fixture::new(
                "second",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -0.25, 1.0, 0.25), 0.0)),
            ),
        );

        let system = PhysicsSystem::new();
        assert_eq!(system.sync_map_physics(&mut entities, &maps), 2);
        assert_eq!(entities.inner.map_contact_count(map_id), 0);
    }
}
