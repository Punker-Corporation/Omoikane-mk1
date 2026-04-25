use crate::Component;

#[derive(Debug, Clone)]
pub struct IgnorePauseComponent {
    pub base: Component,
}

impl IgnorePauseComponent {
    pub fn new() -> Self {
        Self {
            base: Component::new("IgnorePauseComponent"),
        }
    }
}

impl Default for IgnorePauseComponent {
    fn default() -> Self {
        Self::new()
    }
}

