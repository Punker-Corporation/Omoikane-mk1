use crate::{
    CollisionChangeMessage, Component, JointAddedEvent, JointRemovedEvent, PhysicsSleepMessage,
    PhysicsWakeMessage,
};
use butsuri::{Contact, ContactManager, ContactStatus};
use keisan::Vector2;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};

use crate::EntityUid;

#[derive(Debug, Clone, PartialEq)]
pub struct PhysicsContactEvent {
    pub status: ContactStatus,
    pub contact: Contact,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhysicsRuntimeEvent {
    Wake(PhysicsWakeMessage),
    Sleep(PhysicsSleepMessage),
    CollisionChange(CollisionChangeMessage),
    JointAdded(JointAddedEvent),
    JointRemoved(JointRemovedEvent),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SharedPhysicsMapComponentState {
    pub auto_clear_forces: bool,
    pub gravity: Vector2,
    pub bodies: Vec<EntityUid>,
    pub awake_bodies: Vec<EntityUid>,
}

#[derive(Debug, Clone)]
pub struct SharedPhysicsMapComponent {
    pub base: Component,
    pub auto_clear_forces: bool,
    pub gravity: Vector2,
    pub bodies: HashSet<EntityUid>,
    pub awake_bodies: HashSet<EntityUid>,
    contact_manager: ContactManager,
    contact_events: VecDeque<PhysicsContactEvent>,
    runtime_events: VecDeque<PhysicsRuntimeEvent>,
    deferred_updates: HashSet<EntityUid>,
    queued_wake: HashSet<EntityUid>,
    queued_sleep: HashSet<EntityUid>,
    queued_collision_changes: VecDeque<(EntityUid, bool)>,
}

impl SharedPhysicsMapComponent {
    pub fn new() -> Self {
        Self {
            base: Component::new("SharedPhysicsMapComponent"),
            auto_clear_forces: false,
            gravity: Vector2::ZERO,
            bodies: HashSet::new(),
            awake_bodies: HashSet::new(),
            contact_manager: ContactManager::new(),
            contact_events: VecDeque::new(),
            runtime_events: VecDeque::new(),
            deferred_updates: HashSet::new(),
            queued_wake: HashSet::new(),
            queued_sleep: HashSet::new(),
            queued_collision_changes: VecDeque::new(),
        }
    }

    pub fn add_body(&mut self, body: EntityUid, awake: bool) {
        self.bodies.insert(body);
        if awake {
            self.awake_bodies.insert(body);
        }
    }

    pub fn remove_body(&mut self, body: EntityUid) {
        self.bodies.remove(&body);
        self.awake_bodies.remove(&body);
        self.queued_wake.remove(&body);
        self.queued_sleep.remove(&body);
        self.queued_collision_changes
            .retain(|(queued_body, _)| *queued_body != body);
    }

    pub fn replace_contacts(&mut self, contacts: ContactManager) {
        self.contact_manager = contacts;
    }

    pub fn contacts(&self) -> &[Contact] {
        self.contact_manager.contacts()
    }

    pub fn contact_mut(&mut self, index: usize) -> Option<&mut Contact> {
        self.contact_manager.contact_mut(index)
    }

    pub fn contact_count(&self) -> usize {
        self.contact_manager.contact_count()
    }

    pub fn queue_contact_event(&mut self, status: ContactStatus, contact: Contact) {
        if status != ContactStatus::NoContact {
            self.contact_events
                .push_back(PhysicsContactEvent { status, contact });
        }
    }

    pub fn drain_contact_events(&mut self) -> Vec<PhysicsContactEvent> {
        self.contact_events.drain(..).collect()
    }

    pub fn queue_runtime_event(&mut self, event: PhysicsRuntimeEvent) {
        self.runtime_events.push_back(event);
    }

    pub fn drain_runtime_events(&mut self) -> Vec<PhysicsRuntimeEvent> {
        self.runtime_events.drain(..).collect()
    }

    pub fn add_awake_body(&mut self, body: EntityUid) {
        self.queued_sleep.remove(&body);
        self.queued_wake.insert(body);
    }

    pub fn remove_sleep_body(&mut self, body: EntityUid) {
        self.queued_wake.remove(&body);
        self.queued_sleep.insert(body);
    }

    pub fn queue_collision_change(&mut self, body: EntityUid, can_collide: bool) {
        self.queued_collision_changes.push_back((body, can_collide));
    }

    pub fn queue_deferred_update(&mut self, entity: EntityUid) {
        self.deferred_updates.insert(entity);
    }

    pub fn process_changes(&mut self) {
        while let Some((body, can_collide)) = self.queued_collision_changes.pop_front() {
            if can_collide {
                self.bodies.insert(body);
            } else {
                self.remove_body(body);
            }
        }

        for body in self.queued_wake.drain() {
            if self.bodies.contains(&body) {
                self.awake_bodies.insert(body);
            }
        }

        for body in self.queued_sleep.drain() {
            self.awake_bodies.remove(&body);
        }
    }

    pub fn process_queue(&mut self) -> Vec<EntityUid> {
        self.deferred_updates.drain().collect()
    }

    pub fn get_component_state(&self) -> SharedPhysicsMapComponentState {
        let mut bodies = self.bodies.iter().copied().collect::<Vec<_>>();
        let mut awake_bodies = self.awake_bodies.iter().copied().collect::<Vec<_>>();
        bodies.sort();
        awake_bodies.sort();
        SharedPhysicsMapComponentState {
            auto_clear_forces: self.auto_clear_forces,
            gravity: self.gravity,
            bodies,
            awake_bodies,
        }
    }

    pub fn handle_component_state(&mut self, state: SharedPhysicsMapComponentState) {
        self.auto_clear_forces = state.auto_clear_forces;
        self.gravity = state.gravity;
        self.bodies = state.bodies.into_iter().collect();
        self.awake_bodies = state.awake_bodies.into_iter().collect();
        self.deferred_updates.clear();
        self.queued_wake.clear();
        self.queued_sleep.clear();
        self.queued_collision_changes.clear();
        self.contact_manager.clear();
        self.contact_events.clear();
        self.runtime_events.clear();
    }
}

impl Default for SharedPhysicsMapComponent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PhysicsContactEvent, PhysicsRuntimeEvent, SharedPhysicsMapComponent,
        SharedPhysicsMapComponentState,
    };
    use crate::EntityUid;
    use butsuri::{Contact, ContactStatus, ContactType};
    use keisan::Vector2;

    #[test]
    fn physics_map_component_tracks_body_membership_and_queues() {
        let mut map = SharedPhysicsMapComponent::new();
        let body = EntityUid::new(9);
        map.add_body(body, true);
        map.remove_sleep_body(body);
        map.process_changes();
        assert!(map.bodies.contains(&body));
        assert!(!map.awake_bodies.contains(&body));
        assert_eq!(map.contact_count(), 0);
        map.queue_deferred_update(body);
        assert_eq!(map.process_queue(), vec![body]);
    }

    #[test]
    fn physics_map_component_uses_latest_awake_intent() {
        let mut map = SharedPhysicsMapComponent::new();
        let body = EntityUid::new(9);
        map.add_body(body, false);

        map.remove_sleep_body(body);
        map.add_awake_body(body);
        map.process_changes();
        assert!(map.awake_bodies.contains(&body));

        map.add_awake_body(body);
        map.remove_sleep_body(body);
        map.process_changes();
        assert!(!map.awake_bodies.contains(&body));
    }

    #[test]
    fn physics_map_component_removes_pending_collision_changes_for_removed_body() {
        let mut map = SharedPhysicsMapComponent::new();
        let body = EntityUid::new(9);
        map.add_body(body, true);
        map.queue_collision_change(body, true);
        map.remove_body(body);

        map.process_changes();
        assert!(!map.bodies.contains(&body));
        assert!(!map.awake_bodies.contains(&body));
    }

    #[test]
    fn physics_map_component_queues_and_drains_contact_events() {
        let mut map = SharedPhysicsMapComponent::new();
        map.queue_contact_event(
            ContactStatus::StartTouching,
            Contact::new("a", "b", ContactType::Aabb),
        );
        assert_eq!(
            map.drain_contact_events(),
            vec![PhysicsContactEvent {
                status: ContactStatus::StartTouching,
                contact: Contact::new("a", "b", ContactType::Aabb),
            }]
        );
    }

    #[test]
    fn physics_map_component_queues_and_drains_runtime_events() {
        let mut map = SharedPhysicsMapComponent::new();
        map.queue_runtime_event(PhysicsRuntimeEvent::Wake(crate::PhysicsWakeMessage {
            body: EntityUid::new(5),
        }));
        assert_eq!(
            map.drain_runtime_events(),
            vec![PhysicsRuntimeEvent::Wake(crate::PhysicsWakeMessage {
                body: EntityUid::new(5),
            })]
        );
    }

    #[test]
    fn physics_map_component_roundtrips_state() {
        let mut map = SharedPhysicsMapComponent::new();
        map.auto_clear_forces = true;
        map.gravity = Vector2::new(0.0, -9.8);
        map.add_body(EntityUid::new(9), true);
        let state = map.get_component_state();
        let mut restored = SharedPhysicsMapComponent::new();
        restored.handle_component_state(SharedPhysicsMapComponentState {
            auto_clear_forces: state.auto_clear_forces,
            gravity: state.gravity,
            bodies: state.bodies,
            awake_bodies: state.awake_bodies,
        });
        assert!(restored.auto_clear_forces);
        assert!(restored.bodies.contains(&EntityUid::new(9)));
        assert!(restored.awake_bodies.contains(&EntityUid::new(9)));
    }
}
