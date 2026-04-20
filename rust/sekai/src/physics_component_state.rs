use butsuri::BodyType;
use keisan::Vector2;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BodyStatus {
    OnGround,
    InAir,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PhysicsComponentState {
    pub can_collide: bool,
    pub sleeping_allowed: bool,
    pub fixed_rotation: bool,
    pub status: BodyStatus,
    pub linear_velocity: Vector2,
    pub angular_velocity: f32,
    pub body_type: BodyType,
}

impl PhysicsComponentState {
    pub fn new(
        can_collide: bool,
        sleeping_allowed: bool,
        fixed_rotation: bool,
        status: BodyStatus,
        linear_velocity: Vector2,
        angular_velocity: f32,
        body_type: BodyType,
    ) -> Self {
        Self {
            can_collide,
            sleeping_allowed,
            fixed_rotation,
            status,
            linear_velocity,
            angular_velocity,
            body_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BodyStatus, BodyType, PhysicsComponentState};
    use keisan::Vector2;

    #[test]
    fn physics_component_state_keeps_runtime_flags_and_velocities() {
        let state = PhysicsComponentState::new(
            true,
            false,
            true,
            BodyStatus::OnGround,
            Vector2::new(1.0, -2.0),
            0.5,
            BodyType::Dynamic,
        );
        assert!(state.can_collide);
        assert_eq!(state.status, BodyStatus::OnGround);
        assert_eq!(state.body_type, BodyType::Dynamic);
    }
}
