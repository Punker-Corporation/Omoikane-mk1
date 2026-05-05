use crate::{Contact, ContactStatus, Fixture};

#[derive(Debug, Clone, Default)]
pub struct ContactManager {
    active_contacts: Vec<Contact>,
}

impl ContactManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn contact_count(&self) -> usize {
        self.active_contacts.len()
    }

    pub fn contacts(&self) -> &[Contact] {
        &self.active_contacts
    }

    pub fn contact(&self, index: usize) -> Option<&Contact> {
        self.active_contacts.get(index)
    }

    pub fn has_contact_pair(&self, fixture_a_key: &str, fixture_b_key: &str) -> bool {
        self.active_contacts
            .iter()
            .any(|contact| contact.matches_pair(fixture_a_key, fixture_b_key))
    }

    pub fn insert_contact(&mut self, contact: Contact) -> usize {
        let index = self.active_contacts.len();
        self.active_contacts.push(contact);
        index
    }

    pub fn add_pair_with_keys(
        &mut self,
        fixture_a_key: &str,
        fixture_a: &Fixture,
        fixture_b_key: &str,
        fixture_b: &Fixture,
    ) -> Option<usize> {
        if fixture_a_key == fixture_b_key {
            return None;
        }

        if self.has_contact_pair(fixture_a_key, fixture_b_key) {
            return None;
        }

        let contact = Contact::from_fixtures(fixture_a_key, fixture_a, fixture_b_key, fixture_b);
        self.active_contacts.push(contact);
        Some(self.active_contacts.len() - 1)
    }

    #[cfg(test)]
    pub fn destroy_fixture_contacts(&mut self, fixture_id: &str) -> usize {
        let before = self.active_contacts.len();
        self.active_contacts
            .retain(|contact| contact.fixture_a != fixture_id && contact.fixture_b != fixture_id);
        before - self.active_contacts.len()
    }

    pub fn update_touching(&mut self, index: usize, touching: bool) -> Option<ContactStatus> {
        self.active_contacts
            .get_mut(index)
            .map(|contact| contact.update_touching(touching))
    }

    pub fn refresh_contact_manifold(
        &mut self,
        index: usize,
        manifold: crate::ContactManifold,
    ) -> Option<ContactStatus> {
        self.active_contacts
            .get_mut(index)
            .map(|contact| contact.refresh_manifold(manifold))
    }

    pub fn set_contact_enabled(&mut self, index: usize, enabled: bool) -> bool {
        let Some(contact) = self.active_contacts.get_mut(index) else {
            return false;
        };
        contact.enabled = enabled;
        true
    }

    pub fn set_contact_point_impulse(
        &mut self,
        index: usize,
        point_index: usize,
        normal_impulse: f32,
        tangent_impulse: f32,
    ) -> bool {
        let Some(contact) = self.active_contacts.get_mut(index) else {
            return false;
        };
        contact.set_point_impulse(point_index, normal_impulse, tangent_impulse)
    }

    pub fn clear(&mut self) {
        self.active_contacts.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::ContactManager;
    use crate::{AabbShape, Fixture, PhysShape};
    use keisan::Box2;

    #[test]
    fn contact_manager_adds_updates_and_removes_contacts() {
        let fixture_a = Fixture::new(
            "a",
            PhysShape::Aabb(AabbShape::new(Box2::new(0.0, 0.0, 1.0, 1.0), 0.0)),
        );
        let fixture_b = Fixture::new(
            "b",
            PhysShape::Aabb(AabbShape::new(Box2::new(0.5, 0.5, 1.5, 1.5), 0.0)),
        );
        let mut manager = ContactManager::new();
        let index = manager
            .add_pair_with_keys("a", &fixture_a, "b", &fixture_b)
            .unwrap();
        assert_eq!(manager.contact_count(), 1);
        assert!(manager.update_touching(index, true).is_some());
        assert_eq!(manager.destroy_fixture_contacts("a"), 1);
        assert_eq!(manager.contact_count(), 0);
    }

    #[test]
    fn contact_manager_refreshes_contact_manifold() {
        let fixture_a = Fixture::new(
            "a",
            PhysShape::Aabb(AabbShape::new(Box2::new(0.0, 0.0, 1.0, 1.0), 0.0)),
        );
        let fixture_b = Fixture::new(
            "b",
            PhysShape::Aabb(AabbShape::new(Box2::new(0.5, 0.5, 1.5, 1.5), 0.0)),
        );
        let mut manager = ContactManager::new();
        let index = manager
            .add_pair_with_keys("a", &fixture_a, "b", &fixture_b)
            .unwrap();

        assert_eq!(
            manager.refresh_contact_manifold(index, crate::ContactManifold::default()),
            Some(crate::ContactStatus::StartTouching)
        );
        assert!(manager.contact(index).unwrap().is_touching);
    }

    #[test]
    fn contact_manager_exposes_indexed_contact_state() {
        let fixture_a = Fixture::new(
            "a",
            PhysShape::Aabb(AabbShape::new(Box2::new(0.0, 0.0, 1.0, 1.0), 0.0)),
        );
        let fixture_b = Fixture::new(
            "b",
            PhysShape::Aabb(AabbShape::new(Box2::new(0.5, 0.5, 1.5, 1.5), 0.0)),
        );
        let mut manager = ContactManager::new();
        let index = manager
            .add_pair_with_keys("a", &fixture_a, "b", &fixture_b)
            .unwrap();

        assert_eq!(manager.contact(index).unwrap().fixture_a, "a");
        assert!(manager.set_contact_enabled(index, false));
        assert!(!manager.contact(index).unwrap().enabled);
        assert!(!manager.set_contact_enabled(index + 1, true));
    }

    #[test]
    fn contact_manager_updates_contact_point_impulses() {
        let fixture_a = Fixture::new(
            "a",
            PhysShape::Aabb(AabbShape::new(Box2::new(0.0, 0.0, 1.0, 1.0), 0.0)),
        );
        let fixture_b = Fixture::new(
            "b",
            PhysShape::Aabb(AabbShape::new(Box2::new(0.5, 0.5, 1.5, 1.5), 0.0)),
        );
        let mut manager = ContactManager::new();
        let index = manager
            .add_pair_with_keys("a", &fixture_a, "b", &fixture_b)
            .unwrap();
        let mut manifold = crate::ContactManifold::default();
        manifold.points.push(crate::ContactManifoldPoint {
            local_point: keisan::Vector2::ZERO,
            normal_impulse: 0.0,
            tangent_impulse: 0.0,
        });
        let _ = manager.refresh_contact_manifold(index, manifold);

        assert!(manager.set_contact_point_impulse(index, 0, 3.5, 1.25));
        let point = &manager.contact(index).unwrap().manifold.points[0];
        assert_eq!(point.normal_impulse, 3.5);
        assert_eq!(point.tangent_impulse, 1.25);
        assert!(!manager.set_contact_point_impulse(index, 1, 1.0, 1.0));
    }

    #[test]
    fn contact_manager_detects_existing_pairs_symmetrically() {
        let fixture_a = Fixture::new(
            "a",
            PhysShape::Aabb(AabbShape::new(Box2::new(0.0, 0.0, 1.0, 1.0), 0.0)),
        );
        let fixture_b = Fixture::new(
            "b",
            PhysShape::Aabb(AabbShape::new(Box2::new(0.5, 0.5, 1.5, 1.5), 0.0)),
        );
        let mut manager = ContactManager::new();

        assert!(
            manager
                .add_pair_with_keys("body_a:a", &fixture_a, "body_b:b", &fixture_b)
                .is_some()
        );

        assert!(manager.has_contact_pair("body_a:a", "body_b:b"));
        assert!(manager.has_contact_pair("body_b:b", "body_a:a"));
        assert!(
            manager
                .add_pair_with_keys("body_b:b", &fixture_b, "body_a:a", &fixture_a)
                .is_none()
        );
    }

    #[test]
    fn contact_manager_can_track_duplicate_fixture_ids_with_distinct_keys() {
        let fixture_a = Fixture::new(
            "main",
            PhysShape::Aabb(AabbShape::new(Box2::new(0.0, 0.0, 1.0, 1.0), 0.0)),
        );
        let fixture_b = Fixture::new(
            "main",
            PhysShape::Aabb(AabbShape::new(Box2::new(0.5, 0.5, 1.5, 1.5), 0.0)),
        );
        let mut manager = ContactManager::new();
        assert!(
            manager
                .add_pair_with_keys("1:main", &fixture_a, "2:main", &fixture_b)
                .is_some()
        );
        assert_eq!(manager.contact_count(), 1);
    }
}
