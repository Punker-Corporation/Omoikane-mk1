use crate::Component;
use butsuri::Broadphase;

#[derive(Debug, Clone)]
pub struct BroadphaseComponent {
    pub base: Component,
    pub tree: Broadphase,
}

impl BroadphaseComponent {
    pub fn new() -> Self {
        Self {
            base: Component::new("BroadphaseComponent"),
            tree: Broadphase::new(),
        }
    }
}

impl Default for BroadphaseComponent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::BroadphaseComponent;

    #[test]
    fn broadphase_component_starts_empty() {
        let component = BroadphaseComponent::new();
        assert!(component.tree.query_aabb(Default::default()).is_empty());
    }
}
