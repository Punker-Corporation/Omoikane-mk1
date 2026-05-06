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

    pub fn retarget_pair(
        &mut self,
        fixture_a_key: impl Into<String>,
        fixture_a: &Fixture,
        fixture_b_key: impl Into<String>,
        fixture_b: &Fixture,
    ) {
        self.fixture_a = fixture_a_key.into();
        self.fixture_b = fixture_b_key.into();
        self.reset_material(fixture_a, fixture_b);
    }

    pub fn refresh_manifold(&mut self, manifold: ContactManifold) -> ContactStatus {
        self.manifold = manifold;
        self.enabled = true;
        self.update_touching(true)
    }

    pub fn end_touching(&mut self) -> ContactStatus {
        self.update_touching(false)
    }

    pub fn set_point_impulse(
        &mut self,
        point_index: usize,
        normal_impulse: f32,
        tangent_impulse: f32,
    ) -> bool {
        let Some(point) = self.manifold.points.get_mut(point_index) else {
            return false;
        };
        point.normal_impulse = normal_impulse;
        point.tangent_impulse = tangent_impulse;
        true
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
    fn contact_retargets_pair_and_material() {
        let mut fixture_a = Fixture::new(
            "a",
            PhysShape::Aabb(AabbShape::new(Box2::new(0.0, 0.0, 1.0, 1.0), 0.0)),
        );
        fixture_a.friction = 0.25;
        let mut fixture_b =
            Fixture::new("b", PhysShape::Circle(CircleShape::new(Vector2::ZERO, 0.5)));
        fixture_b.friction = 1.0;

        let mut contact = Contact::new("old:a", "old:b", ContactType::Aabb);
        contact.retarget_pair("new:a", &fixture_a, "new:b", &fixture_b);

        assert!(contact.matches_pair("new:a", "new:b"));
        assert_eq!(contact.contact_type, ContactType::Mixed);
        assert_eq!(contact.friction, 0.5);
    }

    #[test]
    fn contact_refreshes_manifold_and_touching_state_together() {
        let mut contact = Contact::new("a", "b", ContactType::Aabb);
        contact.enabled = false;
        let manifold = super::ContactManifold {
            normal: Vector2::new(1.0, 0.0),
            points: Vec::new(),
        };

        assert_eq!(
            contact.refresh_manifold(manifold.clone()),
            ContactStatus::StartTouching
        );
        assert!(contact.is_touching);
        assert!(contact.enabled);
        assert_eq!(contact.manifold, manifold);
        assert_eq!(
            contact.refresh_manifold(super::ContactManifold::default()),
            ContactStatus::NoContact
        );
    }

    #[test]
    fn contact_ends_touching_state_semantically() {
        let mut contact = Contact::new("a", "b", ContactType::Aabb);
        assert_eq!(
            contact.refresh_manifold(super::ContactManifold::default()),
            ContactStatus::StartTouching
        );

        assert_eq!(contact.end_touching(), ContactStatus::EndTouching);
        assert!(!contact.is_touching);
        assert_eq!(contact.end_touching(), ContactStatus::NoContact);
    }

    #[test]
    fn contact_updates_manifold_point_impulses_by_index() {
        let mut contact = Contact::new("a", "b", ContactType::Aabb);
        contact.manifold.points.push(super::ContactManifoldPoint {
            local_point: Vector2::ZERO,
            normal_impulse: 0.0,
            tangent_impulse: 0.0,
        });

        assert!(contact.set_point_impulse(0, 3.5, 1.25));
        assert_eq!(contact.manifold.points[0].normal_impulse, 3.5);
        assert_eq!(contact.manifold.points[0].tangent_impulse, 1.25);
        assert!(!contact.set_point_impulse(1, 1.0, 1.0));
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
