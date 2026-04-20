use crate::EntitySystem;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemChangedArgs {
    pub system_name: String,
}

pub struct EntitySystemManager {
    systems: HashMap<String, Box<dyn EntitySystem>>,
    order: Vec<String>,
    pub initialized: bool,
}

impl EntitySystemManager {
    pub fn new() -> Self {
        Self {
            systems: HashMap::new(),
            order: Vec::new(),
            initialized: false,
        }
    }

    pub fn add_system(&mut self, system: Box<dyn EntitySystem>) {
        let name = system.info().name.clone();
        self.order.push(name.clone());
        self.systems.insert(name, system);
    }

    pub fn initialize(&mut self) {
        if self.initialized {
            return;
        }
        for name in &self.order {
            if let Some(system) = self.systems.get_mut(name) {
                system.initialize();
            }
        }
        self.initialized = true;
    }

    pub fn shutdown(&mut self) {
        for name in &self.order {
            if let Some(system) = self.systems.get_mut(name) {
                system.shutdown();
            }
        }
        self.initialized = false;
    }

    pub fn tick_update(&mut self, frame_time: f32, no_predictions: bool) {
        for name in &self.order {
            if let Some(system) = self.systems.get_mut(name) {
                if no_predictions && !system.info().updates_outside_prediction {
                    continue;
                }
                system.update(frame_time);
            }
        }
    }

    pub fn frame_update(&mut self, frame_time: f32) {
        for name in &self.order {
            if let Some(system) = self.systems.get_mut(name) {
                system.frame_update(frame_time);
            }
        }
    }

    pub fn contains(&self, name: &str) -> bool {
        self.systems.contains_key(name)
    }
}

#[cfg(test)]
mod tests {
    use super::EntitySystemManager;
    use crate::{EntitySystem, EntitySystemInfo};
    use std::sync::{Arc, Mutex};

    struct FakeSystem {
        info: EntitySystemInfo,
        updates: Arc<Mutex<u32>>,
    }

    impl EntitySystem for FakeSystem {
        fn info(&self) -> &EntitySystemInfo {
            &self.info
        }

        fn update(&mut self, _frame_time: f32) {
            *self.updates.lock().unwrap() += 1;
        }
    }

    #[test]
    fn entity_system_manager_runs_registered_systems() {
        let count = Arc::new(Mutex::new(0));
        let mut manager = EntitySystemManager::new();
        manager.add_system(Box::new(FakeSystem {
            info: EntitySystemInfo::new("physics"),
            updates: count.clone(),
        }));
        manager.initialize();
        manager.tick_update(0.016, false);
        assert_eq!(*count.lock().unwrap(), 1);
        assert!(manager.contains("physics"));
    }
}
