use crate::{Contact, ContactStatus, Fixture};

#[derive(Debug, Clone, Default)]
pub struct ContactManager {
    active_contacts: Vec<Contact>,
}

impl ContactManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_contacts(contacts: Vec<Contact>) -> Self {
        Self {
            active_contacts: contacts,
        }
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

    pub fn contact_pair(&self, fixture_a_key: &str, fixture_b_key: &str) -> Option<&Contact> {
        self.active_contacts
            .iter()
            .find(|contact| contact.matches_pair(fixture_a_key, fixture_b_key))
    }

    pub fn has_contact_pair(&self, fixture_a_key: &str, fixture_b_key: &str) -> bool {
        self.contact_pair(fixture_a_key, fixture_b_key).is_some()
    }

    pub fn insert_contact(&mut self, contact: Contact) -> usize {
        let index = self.active_contacts.len();
        self.active_contacts.push(contact);
        index
    }

    pub fn insert_refreshed_contact(
        &mut self,
        mut contact: Contact,
        manifold: crate::ContactManifold,
    ) -> (usize, ContactStatus, Contact) {
        let status = contact.refresh_manifold(manifold);
        let snapshot = contact.clone();
        let index = self.insert_contact(contact);
        (index, status, snapshot)
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

    pub fn add_pair_with_manifold(
        &mut self,
        fixture_a_key: &str,
        fixture_a: &Fixture,
        fixture_b_key: &str,
        fixture_b: &Fixture,
        manifold: crate::ContactManifold,
    ) -> Option<(usize, ContactStatus, Contact)> {
        let index = self.add_pair_with_keys(fixture_a_key, fixture_a, fixture_b_key, fixture_b)?;
        self.refresh_contact_manifold(index, manifold)
            .map(|(status, contact)| (index, status, contact))
    }

    #[cfg(test)]
    pub fn destroy_fixture_contacts(&mut self, fixture_id: &str) -> usize {
        let before = self.active_contacts.len();
        self.active_contacts
            .retain(|contact| contact.fixture_a != fixture_id && contact.fixture_b != fixture_id);
        before - self.active_contacts.len()
    }

    pub fn refresh_contact_manifold(
        &mut self,
        index: usize,
        manifold: crate::ContactManifold,
    ) -> Option<(ContactStatus, Contact)> {
        self.active_contacts.get_mut(index).map(|contact| {
            let status = contact.refresh_manifold(manifold);
            (status, contact.clone())
        })
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
        assert!(
            manager
                .refresh_contact_manifold(index, crate::ContactManifold::default())
                .is_some_and(|(status, _)| status == crate::ContactStatus::StartTouching)
        );
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
            manager
                .refresh_contact_manifold(index, crate::ContactManifold::default())
                .map(|(status, _)| status),
            Some(crate::ContactStatus::StartTouching),
        );
        assert!(manager.contact(index).unwrap().is_touching);
    }

    #[test]
    fn contact_manager_inserts_refreshed_contact() {
        let fixture_a = Fixture::new(
            "a",
            PhysShape::Aabb(AabbShape::new(Box2::new(0.0, 0.0, 1.0, 1.0), 0.0)),
        );
        let fixture_b = Fixture::new(
            "b",
            PhysShape::Aabb(AabbShape::new(Box2::new(0.5, 0.5, 1.5, 1.5), 0.0)),
        );
        let contact = crate::Contact::from_fixtures("a", &fixture_a, "b", &fixture_b);
        let mut manager = ContactManager::new();

        let (index, status, snapshot) =
            manager.insert_refreshed_contact(contact, crate::ContactManifold::default());

        assert_eq!(index, 0);
        assert_eq!(status, crate::ContactStatus::StartTouching);
        assert!(snapshot.is_touching);
        assert_eq!(manager.contact_count(), 1);
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
        assert!(manager.contact(index).unwrap().enabled);
        assert!(manager.contact(index + 1).is_none());
    }

    #[test]
    fn contact_manager_restores_snapshot_and_finds_pairs() {
        let fixture_a = Fixture::new(
            "a",
            PhysShape::Aabb(AabbShape::new(Box2::new(0.0, 0.0, 1.0, 1.0), 0.0)),
        );
        let fixture_b = Fixture::new(
            "b",
            PhysShape::Aabb(AabbShape::new(Box2::new(0.5, 0.5, 1.5, 1.5), 0.0)),
        );
        let mut original = ContactManager::new();
        let _ = original.add_pair_with_keys("body_a:a", &fixture_a, "body_b:b", &fixture_b);

        let restored = ContactManager::from_contacts(original.contacts().to_vec());

        assert_eq!(restored.contact_count(), 1);
        assert!(restored.contact_pair("body_a:a", "body_b:b").is_some());
        assert!(restored.contact_pair("body_b:b", "body_a:a").is_some());
        assert!(restored.contact_pair("body_a:a", "body_c:c").is_none());
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

    #[test]
    fn contact_manager_adds_pair_with_manifold() {
        let fixture_a = Fixture::new(
            "a",
            PhysShape::Aabb(AabbShape::new(Box2::new(0.0, 0.0, 1.0, 1.0), 0.0)),
        );
        let fixture_b = Fixture::new(
            "b",
            PhysShape::Aabb(AabbShape::new(Box2::new(0.5, 0.5, 1.5, 1.5), 0.0)),
        );
        let mut manager = ContactManager::new();

        let (index, status, snapshot) = manager
            .add_pair_with_manifold(
                "body_a:a",
                &fixture_a,
                "body_b:b",
                &fixture_b,
                crate::ContactManifold::default(),
            )
            .unwrap();

        assert_eq!(index, 0);
        assert_eq!(status, crate::ContactStatus::StartTouching);
        assert!(snapshot.is_touching);
        assert!(
            manager
                .add_pair_with_manifold(
                    "body_b:b",
                    &fixture_b,
                    "body_a:a",
                    &fixture_a,
                    crate::ContactManifold::default(),
                )
                .is_none()
        );
    }
}
