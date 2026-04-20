use keisan::Vector2;
use serde::{Deserialize, Serialize};

use crate::BodyType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JointType {
    Unknown,
    Revolute,
    Prismatic,
    Distance,
    Gear,
    Wheel,
    Weld,
    Friction,
    Rope,
    Motor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LimitState {
    Inactive,
    AtLower,
    AtUpper,
    Equal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JointState {
    pub id: String,
    pub enabled: bool,
    pub collide_connected: bool,
    pub uid_a: i32,
    pub uid_b: i32,
    pub local_anchor_a: Vector2,
    pub local_anchor_b: Vector2,
    pub breakpoint: f32,
    pub joint_type: JointType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Joint {
    pub id: String,
    pub enabled: bool,
    pub collide_connected: bool,
    pub body_a_uid: i32,
    pub body_b_uid: i32,
    pub local_anchor_a: Vector2,
    pub local_anchor_b: Vector2,
    pub breakpoint: f32,
    pub joint_type: JointType,
    pub island_flag: bool,
}

impl Joint {
    pub fn new(body_a_uid: i32, body_b_uid: i32, joint_type: JointType) -> Self {
        assert!(body_a_uid != body_b_uid, "joint cannot connect the same body twice");
        Self {
            id: String::new(),
            enabled: true,
            collide_connected: true,
            body_a_uid,
            body_b_uid,
            local_anchor_a: Vector2::ZERO,
            local_anchor_b: Vector2::ZERO,
            breakpoint: f32::MAX,
            joint_type,
            island_flag: false,
        }
    }

    pub fn get_state(&self) -> JointState {
        JointState {
            id: self.id.clone(),
            enabled: self.enabled,
            collide_connected: self.collide_connected,
            uid_a: self.body_a_uid,
            uid_b: self.body_b_uid,
            local_anchor_a: self.local_anchor_a,
            local_anchor_b: self.local_anchor_b,
            breakpoint: self.breakpoint,
            joint_type: self.joint_type,
        }
    }

    pub fn apply_state(&mut self, state: JointState) {
        self.id = state.id;
        self.enabled = state.enabled;
        self.collide_connected = state.collide_connected;
        self.local_anchor_a = state.local_anchor_a;
        self.local_anchor_b = state.local_anchor_b;
        self.breakpoint = state.breakpoint;
        self.joint_type = state.joint_type;
    }

    pub fn should_break(&self, error_squared: f32) -> bool {
        error_squared > self.breakpoint * self.breakpoint
    }

    pub fn blocks_collisions(&self, body_a: BodyType, body_b: BodyType) -> bool {
        self.enabled && !self.collide_connected && body_a != BodyType::Static && body_b != BodyType::Static
    }
}

#[cfg(test)]
mod tests {
    use super::{Joint, JointType};

    #[test]
    fn joint_roundtrips_state_and_break_logic() {
        let mut joint = Joint::new(1, 2, JointType::Distance);
        joint.breakpoint = 2.0;
        let state = joint.get_state();
        let mut restored = Joint::new(1, 2, JointType::Unknown);
        restored.apply_state(state);
        assert_eq!(restored.joint_type, JointType::Distance);
        assert!(restored.should_break(5.0));
    }
}
