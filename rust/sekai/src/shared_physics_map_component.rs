use crate::Component;
use keisan::Vector2;
use std::collections::{HashSet, VecDeque};

use crate::EntityUid;

#[derive(Debug, Clone)]
pub struct SharedPhysicsMapComponent {
    pub base: Component,
    pub auto_clear_forces: bool,
    pub gravity: Vector2,
    pub bodies: HashSet<EntityUid>,
    pub awake_bodies: HashSet<EntityUid>,
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
    }

    pub fn add_awake_body(&mut self, body: EntityUid) {
        self.queued_wake.insert(body);
    }

    pub fn remove_sleep_body(&mut self, body: EntityUid) {
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
}

impl Default for SharedPhysicsMapComponent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::SharedPhysicsMapComponent;
    use crate::EntityUid;

    #[test]
    fn physics_map_component_tracks_body_membership_and_queues() {
        let mut map = SharedPhysicsMapComponent::new();
        let body = EntityUid::new(9);
        map.add_body(body, true);
        map.remove_sleep_body(body);
        map.process_changes();
        assert!(map.bodies.contains(&body));
        assert!(!map.awake_bodies.contains(&body));
        map.queue_deferred_update(body);
        assert_eq!(map.process_queue(), vec![body]);
    }
}
