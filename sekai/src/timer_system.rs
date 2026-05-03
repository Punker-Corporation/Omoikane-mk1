use crate::EntityManager;

#[derive(Debug, Default, Clone, Copy)]
pub struct TimerSystem;

impl TimerSystem {
    pub fn update(&self, manager: &mut EntityManager, frame_time: f32) {
        let timers: Vec<_> = manager.timers.keys().copied().collect();

        for uid in &timers {
            if let Some(timer) = manager.timers.get_mut(uid) {
                timer.update(frame_time);
            }
        }

        for uid in timers {
            let remove = match manager.timers.get(&uid) {
                Some(timer) => timer.remove_on_empty && timer.timer_count() == 0,
                None => false,
            };

            if remove && !manager.deleted(uid) {
                manager.timers.remove(&uid);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TimerSystem;
    use crate::EntityManager;
    use std::sync::{Arc, Mutex};

    #[test]
    fn timer_system_updates_and_cleans_empty_components() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        let fired = Arc::new(Mutex::new(0));
        let fired_clone = fired.clone();
        let timer = manager.ensure_timer(uid);
        timer.spawn(20, move || {
            *fired_clone.lock().unwrap() += 1;
        });
        TimerSystem.update(&mut manager, 0.03);
        assert_eq!(*fired.lock().unwrap(), 1);
        assert!(!manager.timers.contains_key(&uid));
    }
}
