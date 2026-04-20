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
    pub linear_velocity: Vector2,
    pub angular_velocity: f32,
    pub force: Vector2,
    pub torque: f32,
    pub predict: bool,
}

impl PhysicsComponent {
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

        if self.awake == value {
            return;
        }

        self.awake = value;
        if !value {
            self.reset_dynamics();
            self.sleep_time = 0.0;
        }
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

    pub fn reset_dynamics(&mut self) {
        self.torque = 0.0;
        self.angular_velocity = 0.0;
        self.force = Vector2::ZERO;
        self.linear_velocity = Vector2::ZERO;
    }

    pub fn get_component_state(&self) -> PhysicsComponentState {
        PhysicsComponentState::new(
            self.can_collide,
            self.sleeping_allowed,
            self.fixed_rotation,
            self.body_status,
            self.linear_velocity,
            self.angular_velocity,
            self.body_type,
        )
    }

    pub fn handle_component_state(&mut self, state: PhysicsComponentState) {
        self.sleeping_allowed = state.sleeping_allowed;
        self.fixed_rotation = state.fixed_rotation;
        self.can_collide = state.can_collide;
        self.body_status = state.status;
        self.linear_velocity = state.linear_velocity;
        self.angular_velocity = state.angular_velocity;
        self.set_body_type(state.body_type);
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
    use crate::{EntityUid, FixturesComponent, GridId, MapId, TransformComponent, TransformResolver, WorldTransform};
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
}
