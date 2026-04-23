use crate::Component;
use butsuri::Joint;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JointComponentState {
    pub joints: Vec<Joint>,
}

#[derive(Debug, Clone)]
pub struct JointComponent {
    pub base: Component,
    pub joints: HashMap<String, Joint>,
}

impl JointComponent {
    pub fn new() -> Self {
        Self {
            base: Component::new("JointComponent"),
            joints: HashMap::new(),
        }
    }

    pub fn joint_count(&self) -> usize {
        self.joints.len()
    }

    pub fn add_joint(&mut self, joint: Joint) -> bool {
        let id = joint.id.clone();
        self.joints.insert(id, joint).is_none()
    }

    pub fn remove_joint(&mut self, id: &str) -> Option<Joint> {
        self.joints.remove(id)
    }

    pub fn get_component_state(&self) -> JointComponentState {
        let mut joints = self.joints.values().cloned().collect::<Vec<_>>();
        joints.sort_by(|a, b| a.id.cmp(&b.id));
        JointComponentState { joints }
    }

    pub fn handle_component_state(&mut self, state: JointComponentState) {
        self.joints.clear();
        for joint in state.joints {
            self.joints.insert(joint.id.clone(), joint);
        }
    }
}

impl Default for JointComponent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{JointComponent, JointComponentState};
    use butsuri::{Joint, JointType};

    #[test]
    fn joint_component_tracks_joints_by_id() {
        let mut component = JointComponent::new();
        let mut joint = Joint::new(1, 2, JointType::Distance);
        joint.id = "rope".to_string();
        assert!(component.add_joint(joint));
        assert_eq!(component.joint_count(), 1);
        assert!(component.remove_joint("rope").is_some());
    }

    #[test]
    fn joint_component_roundtrips_component_state() {
        let mut component = JointComponent::new();
        let mut joint = Joint::new(1, 2, JointType::Distance);
        joint.id = "rope".to_string();
        component.add_joint(joint);
        let state = component.get_component_state();
        let mut restored = JointComponent::new();
        restored.handle_component_state(JointComponentState {
            joints: state.joints,
        });
        assert_eq!(restored.joint_count(), 1);
        assert!(restored.joints.contains_key("rope"));
    }
}
