use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BodyType {
    Kinematic = 0,
    KinematicController = 1 << 1,
    Static = 1 << 2,
    Dynamic = 1 << 3,
}

impl BodyType {
    pub fn is_static(self) -> bool {
        matches!(self, Self::Static)
    }

    pub fn is_dynamic(self) -> bool {
        matches!(self, Self::Dynamic)
    }
}
