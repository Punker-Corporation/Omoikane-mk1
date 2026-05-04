use keisan::Vector2;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContactStatus {
    NoContact,
    StartTouching,
    EndTouching,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContactType {
    Aabb,
    Circle,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ContactManifoldPoint {
    pub local_point: Vector2,
    pub normal_impulse: f32,
    pub tangent_impulse: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContactManifold {
    pub normal: Vector2,
    pub points: Vec<ContactManifoldPoint>,
}

impl Default for ContactManifold {
    fn default() -> Self {
        Self {
            normal: Vector2::ZERO,
            points: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contact {
    pub fixture_a: String,
    pub fixture_b: String,
    pub contact_type: ContactType,
    pub manifold: ContactManifold,
    pub island_flag: bool,
    pub filter_flag: bool,
    pub is_touching: bool,
    pub enabled: bool,
    pub friction: f32,
    pub restitution: f32,
    pub tangent_speed: f32,
}

impl Contact {
    pub fn new(
        fixture_a: impl Into<String>,
        fixture_b: impl Into<String>,
        contact_type: ContactType,
    ) -> Self {
        Self {
            fixture_a: fixture_a.into(),
            fixture_b: fixture_b.into(),
            contact_type,
            manifold: ContactManifold::default(),
            island_flag: false,
            filter_flag: false,
            is_touching: false,
            enabled: true,
            friction: 0.0,
            restitution: 0.0,
            tangent_speed: 0.0,
        }
    }

    pub fn reset_restitution(&mut self, restitution_a: f32, restitution_b: f32) {
        self.restitution = restitution_a.max(restitution_b);
    }

    pub fn reset_friction(&mut self, friction_a: f32, friction_b: f32) {
        self.friction = (friction_a * friction_b).sqrt();
    }

    pub fn matches_pair(&self, fixture_a: &str, fixture_b: &str) -> bool {
        (self.fixture_a == fixture_a && self.fixture_b == fixture_b)
            || (self.fixture_a == fixture_b && self.fixture_b == fixture_a)
    }

    pub fn update_touching(&mut self, touching: bool) -> ContactStatus {
        let previous = self.is_touching;
        self.is_touching = touching;
        match (previous, touching) {
            (false, true) => ContactStatus::StartTouching,
            (true, false) => ContactStatus::EndTouching,
            _ => ContactStatus::NoContact,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Contact, ContactStatus, ContactType};

    #[test]
    fn contact_tracks_touching_state_and_mixes_materials() {
        let mut contact = Contact::new("a", "b", ContactType::Aabb);
        contact.reset_friction(0.25, 1.0);
        contact.reset_restitution(0.1, 0.8);
        assert_eq!(contact.update_touching(true), ContactStatus::StartTouching);
        assert!(contact.is_touching);
        assert_eq!(contact.restitution, 0.8);
    }

    #[test]
    fn contact_matches_fixture_pairs_symmetrically() {
        let contact = Contact::new("body_a:main", "body_b:main", ContactType::Aabb);

        assert!(contact.matches_pair("body_a:main", "body_b:main"));
        assert!(contact.matches_pair("body_b:main", "body_a:main"));
        assert!(!contact.matches_pair("body_a:main", "body_c:main"));
    }
}
