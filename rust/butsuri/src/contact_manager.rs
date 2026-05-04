use crate::{Contact, ContactStatus, ContactType, Fixture, PhysShape};

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

    pub fn contact_mut(&mut self, index: usize) -> Option<&mut Contact> {
        self.active_contacts.get_mut(index)
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

        let contact_type = match (&fixture_a.shape, &fixture_b.shape) {
            (PhysShape::Aabb(_), PhysShape::Aabb(_)) => ContactType::Aabb,
            (PhysShape::Circle(_), PhysShape::Circle(_)) => ContactType::Circle,
            _ => ContactType::Mixed,
        };

        let mut contact = Contact::new(
            fixture_a_key.to_string(),
            fixture_b_key.to_string(),
            contact_type,
        );
        contact.reset_friction(fixture_a.friction, fixture_b.friction);
        contact.reset_restitution(fixture_a.restitution, fixture_b.restitution);
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
