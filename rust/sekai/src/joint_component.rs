use crate::Component;
use butsuri::Joint;
use std::collections::HashMap;

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
}

impl Default for JointComponent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::JointComponent;
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
}
