use crate::{Fixture, PhysShape};
use keisan::Vector2;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContactStatus {
    NoContact,
    StartTouching,
    EndTouching,
}

impl ContactStatus {
    pub fn is_contact_change(self) -> bool {
        self != Self::NoContact
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContactType {
    Aabb,
    Circle,
    Mixed,
}

impl ContactType {
    pub fn from_shapes(shape_a: &PhysShape, shape_b: &PhysShape) -> Self {
        match (shape_a, shape_b) {
            (PhysShape::Aabb(_), PhysShape::Aabb(_)) => Self::Aabb,
            (PhysShape::Circle(_), PhysShape::Circle(_)) => Self::Circle,
            _ => Self::Mixed,
        }
    }
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

    pub fn reset_material(&mut self, fixture_a: &Fixture, fixture_b: &Fixture) {
        self.contact_type = ContactType::from_shapes(&fixture_a.shape, &fixture_b.shape);
        self.reset_friction(fixture_a.friction, fixture_b.friction);
        self.reset_restitution(fixture_a.restitution, fixture_b.restitution);
    }

    pub fn from_fixtures(
        fixture_a_key: impl Into<String>,
        fixture_a: &Fixture,
        fixture_b_key: impl Into<String>,
        fixture_b: &Fixture,
    ) -> Self {
        let mut contact = Self::new(
            fixture_a_key,
            fixture_b_key,
            ContactType::from_shapes(&fixture_a.shape, &fixture_b.shape),
        );
        contact.reset_material(fixture_a, fixture_b);
        contact
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
    use crate::{AabbShape, CircleShape, Fixture, PhysShape};
    use keisan::{Box2, Vector2};

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
    fn contact_status_identifies_real_transitions() {
        assert!(!ContactStatus::NoContact.is_contact_change());
        assert!(ContactStatus::StartTouching.is_contact_change());
        assert!(ContactStatus::EndTouching.is_contact_change());
    }

    #[test]
    fn contact_matches_fixture_pairs_symmetrically() {
        let contact = Contact::new("body_a:main", "body_b:main", ContactType::Aabb);

        assert!(contact.matches_pair("body_a:main", "body_b:main"));
        assert!(contact.matches_pair("body_b:main", "body_a:main"));
        assert!(!contact.matches_pair("body_a:main", "body_c:main"));
    }

    #[test]
    fn contact_builds_type_and_material_from_fixtures() {
        let mut fixture_a = Fixture::new(
            "a",
            PhysShape::Aabb(AabbShape::new(Box2::new(0.0, 0.0, 1.0, 1.0), 0.0)),
        );
        fixture_a.friction = 0.25;
        fixture_a.restitution = 0.1;
        let mut fixture_b =
            Fixture::new("b", PhysShape::Circle(CircleShape::new(Vector2::ZERO, 0.5)));
        fixture_b.friction = 1.0;
        fixture_b.restitution = 0.8;

        let contact = Contact::from_fixtures("body_a:a", &fixture_a, "body_b:b", &fixture_b);

        assert_eq!(contact.contact_type, ContactType::Mixed);
        assert_eq!(contact.friction, 0.5);
        assert_eq!(contact.restitution, 0.8);
    }
}
