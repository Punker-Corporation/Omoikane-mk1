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

    pub fn add_pair(&mut self, fixture_a: &Fixture, fixture_b: &Fixture) -> Option<usize> {
        if fixture_a.id == fixture_b.id {
            return None;
        }

        if self
            .active_contacts
            .iter()
            .any(|contact| {
                (contact.fixture_a == fixture_a.id && contact.fixture_b == fixture_b.id)
                    || (contact.fixture_a == fixture_b.id && contact.fixture_b == fixture_a.id)
            })
        {
            return None;
        }

        let contact_type = match (&fixture_a.shape, &fixture_b.shape) {
            (PhysShape::Aabb(_), PhysShape::Aabb(_)) => ContactType::Aabb,
            (PhysShape::Circle(_), PhysShape::Circle(_)) => ContactType::Circle,
            _ => ContactType::Mixed,
        };

        let mut contact = Contact::new(fixture_a.id.clone(), fixture_b.id.clone(), contact_type);
        contact.reset_friction(fixture_a.friction, fixture_b.friction);
        contact.reset_restitution(fixture_a.restitution, fixture_b.restitution);
        self.active_contacts.push(contact);
        Some(self.active_contacts.len() - 1)
    }

    pub fn destroy(&mut self, index: usize) -> Option<Contact> {
        (index < self.active_contacts.len()).then(|| self.active_contacts.swap_remove(index))
    }

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
        let fixture_a = Fixture::new("a", PhysShape::Aabb(AabbShape::new(Box2::new(0.0, 0.0, 1.0, 1.0), 0.0)));
        let fixture_b = Fixture::new("b", PhysShape::Aabb(AabbShape::new(Box2::new(0.5, 0.5, 1.5, 1.5), 0.0)));
        let mut manager = ContactManager::new();
        let index = manager.add_pair(&fixture_a, &fixture_b).unwrap();
        assert_eq!(manager.contact_count(), 1);
        assert!(manager.update_touching(index, true).is_some());
        assert_eq!(manager.destroy_fixture_contacts("a"), 1);
        assert_eq!(manager.contact_count(), 0);
    }
}
