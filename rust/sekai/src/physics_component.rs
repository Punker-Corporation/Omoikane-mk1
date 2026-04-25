use crate::{BodyStatus, Component, FixturesComponent, PhysicsComponentState, TransformComponent, TransformResolver};
use butsuri::{BodyType, Transform as PhysicsTransform};
use keisan::{ApproxEq, Box2, Vector2};

#[derive(Debug, Clone)]
pub struct PhysicsComponent {
    pub base: Component,
    pub body_status: BodyStatus,
    pub island: bool,
    pub ignore_paused: bool,
    pub body_type: BodyType,
    pub awake: bool,
    pub sleeping_allowed: bool,
    pub sleep_time: f32,
    pub can_collide: bool,
    pub fixed_rotation: bool,
    pub ignore_gravity: bool,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub linear_velocity: Vector2,
    pub angular_velocity: f32,
    pub force: Vector2,
    pub torque: f32,
    pub predict: bool,
}

impl PhysicsComponent {
    const ANGULAR_VELOCITY_TOLERANCE: f32 = 0.00001;

    pub fn new() -> Self {
        Self {
            base: Component::new("Physics"),
            body_status: BodyStatus::OnGround,
            island: false,
            ignore_paused: false,
            body_type: BodyType::Static,
            awake: true,
            sleeping_allowed: true,
            sleep_time: 0.0,
            can_collide: false,
            fixed_rotation: false,
            ignore_gravity: false,
            linear_damping: 0.2,
            angular_damping: 0.2,
            linear_velocity: Vector2::ZERO,
            angular_velocity: 0.0,
            force: Vector2::ZERO,
            torque: 0.0,
            predict: true,
        }
    }

    pub fn set_body_type(&mut self, value: BodyType) {
        if self.body_type == value {
            return;
        }

        self.body_type = value;
        if self.body_type == BodyType::Static {
            self.awake = false;
            self.linear_velocity = Vector2::ZERO;
            self.angular_velocity = 0.0;
        } else {
            self.awake = true;
        }

        self.force = Vector2::ZERO;
        self.torque = 0.0;
    }

    pub fn set_awake(&mut self, value: bool) {
        if self.body_type == BodyType::Static {
            self.awake = false;
            return;
        }

        if !value && !self.sleeping_allowed {
            self.awake = true;
            self.sleep_time = 0.0;
            return;
        }

        if self.awake == value {
            return;
        }

        self.awake = value;
        if !value {
            self.reset_dynamics();
            self.sleep_time = 0.0;
        }
    }

    pub fn set_sleeping_allowed(&mut self, value: bool) {
        if self.sleeping_allowed == value {
            return;
        }

        self.sleeping_allowed = value;
        if !value {
            self.set_awake(true);
        }
    }

    pub fn set_fixed_rotation(&mut self, value: bool) {
        if self.fixed_rotation == value {
            return;
        }

        self.fixed_rotation = value;
        self.angular_velocity = 0.0;
        self.torque = 0.0;
    }

    pub fn set_linear_velocity(&mut self, velocity: Vector2) {
        if self.body_type == BodyType::Static {
            return;
        }

        if velocity.length_squared() > 0.0 {
            self.set_awake(true);
        }

        if self.linear_velocity.approx_eq_with_tolerance(velocity, 0.0001) {
            return;
        }

        self.linear_velocity = velocity;
    }

    pub fn set_angular_velocity(&mut self, velocity: f32) {
        if self.body_type == BodyType::Static {
            return;
        }

        if self.fixed_rotation {
            self.angular_velocity = 0.0;
            return;
        }

        if velocity * velocity > 0.0 {
            self.set_awake(true);
        }

        if (self.angular_velocity - velocity).abs() <= Self::ANGULAR_VELOCITY_TOLERANCE {
            return;
        }

        self.angular_velocity = velocity;
    }

    pub fn set_body_status(&mut self, status: BodyStatus) {
        self.body_status = status;
    }

    pub fn set_ignore_gravity(&mut self, value: bool) {
        self.ignore_gravity = value;
    }

    pub fn set_linear_damping(&mut self, value: f32) {
        if value.is_finite() {
            self.linear_damping = value.max(0.0);
        }
    }

    pub fn set_angular_damping(&mut self, value: f32) {
        if value.is_finite() {
            self.angular_damping = value.max(0.0);
        }
    }

    pub fn apply_force(&mut self, force: Vector2) {
        if self.body_type != BodyType::Dynamic {
            return;
        }

        self.set_awake(true);
        self.force = self.force + force;
    }

    pub fn apply_torque(&mut self, torque: f32) {
        if self.body_type != BodyType::Dynamic {
            return;
        }

        self.set_awake(true);
        self.torque += torque;
    }

    pub fn apply_linear_impulse(&mut self, impulse: Vector2) {
        if self.body_type == BodyType::Static {
            return;
        }

        self.set_awake(true);
        self.set_linear_velocity(self.linear_velocity + impulse);
    }

    pub fn apply_angular_impulse(&mut self, impulse: f32) {
        if self.body_type == BodyType::Static {
            return;
        }

        self.set_awake(true);
        self.set_angular_velocity(self.angular_velocity + impulse);
    }

    pub fn reset_dynamics(&mut self) {
        self.torque = 0.0;
        self.angular_velocity = 0.0;
        self.force = Vector2::ZERO;
        self.linear_velocity = Vector2::ZERO;
    }

    pub fn get_component_state(&self) -> PhysicsComponentState {
        PhysicsComponentState::new(
            self.can_collide,
            self.awake,
            self.sleeping_allowed,
            self.fixed_rotation,
            self.body_status,
            self.linear_velocity,
            self.angular_velocity,
            self.body_type,
        )
    }

    pub fn handle_component_state(&mut self, state: PhysicsComponentState) {
        self.set_sleeping_allowed(state.sleeping_allowed);
        self.set_fixed_rotation(state.fixed_rotation);
        self.can_collide = state.can_collide;
        self.set_body_status(state.status);
        self.set_body_type(state.body_type);
        self.set_linear_velocity(state.linear_velocity);
        self.set_angular_velocity(state.angular_velocity);
        self.set_awake(state.awake);
        self.predict = false;
    }

    pub fn get_aabb<R: TransformResolver>(
        &self,
        transform: &TransformComponent,
        fixtures: &FixturesComponent,
        resolver: &R,
    ) -> Option<Box2> {
        let (world_pos, world_rot, _) = transform.get_world_position_rotation_matrix(resolver);
        fixtures.compute_aabb(PhysicsTransform::from_angle_type(world_pos, world_rot))
    }

    pub fn get_hard_aabb<R: TransformResolver>(
        &self,
        transform: &TransformComponent,
        fixtures: &FixturesComponent,
        resolver: &R,
    ) -> Option<Box2> {
        let (world_pos, world_rot, _) = transform.get_world_position_rotation_matrix(resolver);
        fixtures.compute_hard_aabb(PhysicsTransform::from_angle_type(world_pos, world_rot))
    }
}

impl Default for PhysicsComponent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::PhysicsComponent;
    use crate::{BodyStatus, EntityUid, FixturesComponent, GridId, MapId, PhysicsComponentState, TransformComponent, TransformResolver, WorldTransform};
    use butsuri::{AabbShape, BodyType, Fixture, PhysShape};
    use keisan::{Angle, Box2, Matrix3, Vector2};

    struct Resolver;

    impl TransformResolver for Resolver {
        fn world_transform(&self, entity: EntityUid) -> Option<WorldTransform> {
            if entity.is_valid() {
                let matrix = Matrix3::create_transform(4.0, 5.0, 0.0, 1.0, 1.0);
                Some(WorldTransform {
                    map_id: MapId::new(1),
                    grid_id: GridId::new(2),
                    world_position: Vector2::new(4.0, 5.0),
                    world_rotation: Angle::ZERO,
                    world_matrix: matrix,
                    inv_world_matrix: matrix.inverted(),
                })
            } else {
                None
            }
        }
    }

    #[test]
    fn physics_component_builds_world_aabb_from_fixtures() {
        let mut body = PhysicsComponent::new();
        body.set_body_type(BodyType::Dynamic);
        let mut fixtures = FixturesComponent::new();
        fixtures.insert_fixture(Fixture::new(
            "main",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        ));
        let mut xform = TransformComponent::new();
        xform.base.owner = EntityUid::new(7);
        xform.set_parent(EntityUid::new(7));
        let bounds = body.get_aabb(&xform, &fixtures, &Resolver).unwrap();
        assert_eq!(bounds, Box2::new(3.0, 4.0, 5.0, 6.0));
    }

    #[test]
    fn physics_component_disabling_sleeping_wakes_body() {
        let mut body = PhysicsComponent::new();
        body.set_body_type(BodyType::Dynamic);
        body.set_awake(false);
        body.set_sleeping_allowed(false);
        assert!(body.awake);
        assert!(!body.sleeping_allowed);
    }

    #[test]
    fn physics_component_fixed_rotation_zeroes_angular_state() {
        let mut body = PhysicsComponent::new();
        body.set_body_type(BodyType::Dynamic);
        body.angular_velocity = 3.0;
        body.torque = 2.0;
        body.set_fixed_rotation(true);
        assert!(body.fixed_rotation);
        assert_eq!(body.angular_velocity, 0.0);
        assert_eq!(body.torque, 0.0);
        body.set_angular_velocity(5.0);
        assert_eq!(body.angular_velocity, 0.0);
    }

    #[test]
    fn physics_component_handle_state_keeps_non_sleeping_bodies_awake() {
        let mut body = PhysicsComponent::new();
        body.handle_component_state(PhysicsComponentState::new(
            true,
            false,
            false,
            false,
            BodyStatus::OnGround,
            Vector2::ZERO,
            0.0,
            BodyType::Dynamic,
        ));
        assert!(!body.sleeping_allowed);
        assert!(body.awake);
    }

    #[test]
    fn physics_component_accumulates_force_and_wakes_dynamic_bodies() {
        let mut body = PhysicsComponent::new();
        body.set_body_type(BodyType::Dynamic);
        body.set_awake(false);
        body.apply_force(Vector2::new(2.0, -1.0));
        assert!(body.awake);
        assert_eq!(body.force, Vector2::new(2.0, -1.0));
        body.apply_force(Vector2::new(-1.0, 3.0));
        assert_eq!(body.force, Vector2::new(1.0, 2.0));
    }

    #[test]
    fn physics_component_applies_linear_and_angular_impulses() {
        let mut body = PhysicsComponent::new();
        body.set_body_type(BodyType::Dynamic);
        body.set_awake(false);
        body.apply_linear_impulse(Vector2::new(1.5, -0.5));
        assert!(body.awake);
        assert_eq!(body.linear_velocity, Vector2::new(1.5, -0.5));

        body.apply_angular_impulse(2.0);
        assert_eq!(body.angular_velocity, 2.0);
    }

    #[test]
    fn physics_component_accumulates_torque_and_wakes_dynamic_bodies() {
        let mut body = PhysicsComponent::new();
        body.set_body_type(BodyType::Dynamic);
        body.set_awake(false);
        body.apply_torque(1.5);
        assert!(body.awake);
        assert_eq!(body.torque, 1.5);
        body.apply_torque(-0.5);
        assert_eq!(body.torque, 1.0);
    }

    #[test]
    fn physics_component_tracks_gravity_and_damping_flags() {
        let mut body = PhysicsComponent::new();
        body.set_ignore_gravity(true);
        body.set_linear_damping(0.6);
        body.set_angular_damping(0.4);
        assert!(body.ignore_gravity);
        assert_eq!(body.linear_damping, 0.6);
        assert_eq!(body.angular_damping, 0.4);
    }
}
