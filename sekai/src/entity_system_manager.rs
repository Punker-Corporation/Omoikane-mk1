use crate::{EntityEventBus, EntitySystem, EntitySystemSubscriptions, EventSource};
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::mpsc::Receiver;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemChangedArgs {
    pub system_name: String,
}

pub struct EntitySystemManager {
    systems: HashMap<String, Box<dyn EntitySystem>>,
    order: Vec<String>,
    pub event_bus: EntityEventBus,
    pub initialized: bool,
}

impl Default for EntitySystemManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EntitySystemManager {
    pub fn new() -> Self {
        Self {
            systems: HashMap::new(),
            order: Vec::new(),
            event_bus: EntityEventBus::new(),
            initialized: false,
        }
    }

    pub fn add_system(&mut self, system: Box<dyn EntitySystem>) {
        let name = system.info().name.clone();
        let mut subscriptions =
            EntitySystemSubscriptions::new(&mut self.event_bus, name.clone(), name.clone());
        system.register_subscriptions(&mut subscriptions);
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
            self.event_bus.unsubscribe_subscriber(name);
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

    pub fn system_count(&self) -> usize {
        self.systems.len()
    }

    pub fn system_names(&self) -> Vec<String> {
        self.order.clone()
    }

    pub fn has_system_type<T>(&self) -> bool
    where
        T: EntitySystem + 'static,
    {
        self.get_system::<T>().is_some()
    }

    pub fn get_system<T>(&self) -> Option<&T>
    where
        T: EntitySystem + 'static,
    {
        self.systems
            .values()
            .find_map(|system| system.as_any().downcast_ref::<T>())
    }

    pub fn get_system_mut<T>(&mut self) -> Option<&mut T>
    where
        T: EntitySystem + 'static,
    {
        self.systems
            .values_mut()
            .find_map(|system| system.as_any_mut().downcast_mut::<T>())
    }

    pub fn system_type_ids(&self) -> Vec<TypeId> {
        self.systems
            .values()
            .map(|system| system.as_any().type_id())
            .collect()
    }

    pub fn raise_event<T>(&mut self, source: EventSource, event: &mut T)
    where
        T: std::any::Any + Send + 'static,
    {
        self.event_bus.raise_event(source, event);
    }

    pub fn raise_local_event<T>(
        &mut self,
        uid: crate::EntityUid,
        component_names: &[String],
        event: &mut T,
        broadcast: bool,
    ) where
        T: std::any::Any + Send + 'static,
    {
        self.event_bus
            .raise_local_event(uid, component_names, event, broadcast);
    }

    pub fn raise_component_event<T>(
        &mut self,
        uid: crate::EntityUid,
        component_name: &str,
        event: &mut T,
    ) where
        T: std::any::Any + Send + 'static,
    {
        self.event_bus
            .raise_component_event(uid, component_name, event);
    }

    pub fn queue_event<T>(&mut self, source: EventSource, event: T)
    where
        T: std::any::Any + Send + 'static,
    {
        self.event_bus.queue_event(source, event);
    }

    pub fn await_event<T>(&mut self, source: EventSource) -> Receiver<T>
    where
        T: std::any::Any + Clone + Send + 'static,
    {
        self.event_bus.await_event(source)
    }

    pub fn process_event_queue(&mut self) {
        self.event_bus.process_event_queue();
    }
}

#[cfg(test)]
mod tests {
    use super::EntitySystemManager;
    use crate::{
        EntityPausedEvent, EntitySystem, EntitySystemInfo, EntitySystemSubscriptions, EntityUid,
        EventSource,
    };
    use std::sync::{Arc, Mutex};

    struct FakeSystem {
        info: EntitySystemInfo,
        updates: Arc<Mutex<u32>>,
    }

    impl EntitySystem for FakeSystem {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }

        fn info(&self) -> &EntitySystemInfo {
            &self.info
        }

        fn update(&mut self, _frame_time: f32) {
            *self.updates.lock().unwrap() += 1;
        }

        fn register_subscriptions(&self, subscriptions: &mut EntitySystemSubscriptions<'_>) {
            let updates = self.updates.clone();
            subscriptions.subscribe_event(
                EventSource::Local,
                move |event: &mut i32| {
                    *updates.lock().unwrap() += *event as u32;
                },
                std::iter::empty::<String>(),
                std::iter::empty::<String>(),
            );
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

    #[test]
    fn entity_system_manager_registers_event_bus_subscriptions() {
        let count = Arc::new(Mutex::new(0));
        let mut manager = EntitySystemManager::new();
        manager.add_system(Box::new(FakeSystem {
            info: EntitySystemInfo::new("physics"),
            updates: count.clone(),
        }));

        let mut ev = 2i32;
        manager.raise_event(EventSource::Local, &mut ev);
        assert_eq!(*count.lock().unwrap(), 2);
    }

    #[test]
    fn entity_system_manager_registers_local_component_subscriptions() {
        struct LocalSystem {
            info: EntitySystemInfo,
            hits: Arc<Mutex<Vec<(i32, bool)>>>,
        }

        impl EntitySystem for LocalSystem {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }

            fn info(&self) -> &EntitySystemInfo {
                &self.info
            }

            fn register_subscriptions(&self, subscriptions: &mut EntitySystemSubscriptions<'_>) {
                let hits = self.hits.clone();
                subscriptions.subscribe_local_event(
                    "MetaDataComponent",
                    move |uid: EntityUid, _component: &str, ev: &mut EntityPausedEvent| {
                        hits.lock().unwrap().push((uid.raw(), ev.paused));
                    },
                    std::iter::empty::<String>(),
                    std::iter::empty::<String>(),
                );
            }
        }

        let hits = Arc::new(Mutex::new(Vec::new()));
        let mut manager = EntitySystemManager::new();
        manager.add_system(Box::new(LocalSystem {
            info: EntitySystemInfo::new("metadata"),
            hits: hits.clone(),
        }));

        let mut ev = EntityPausedEvent {
            entity: EntityUid::new(99),
            paused: true,
        };
        manager.raise_local_event(
            EntityUid::new(99),
            &[String::from("MetaDataComponent")],
            &mut ev,
            false,
        );
        assert_eq!(&*hits.lock().unwrap(), &[(99, true)]);
    }

    #[test]
    fn entity_system_manager_can_lookup_registered_system_by_type() {
        let count = Arc::new(Mutex::new(0));
        let mut manager = EntitySystemManager::new();
        manager.add_system(Box::new(FakeSystem {
            info: EntitySystemInfo::new("typed"),
            updates: count,
        }));

        assert!(manager.has_system_type::<FakeSystem>());
        assert!(manager.get_system::<FakeSystem>().is_some());
        assert!(manager.get_system_mut::<FakeSystem>().is_some());
        assert_eq!(manager.system_type_ids().len(), 1);
    }

    #[test]
    fn entity_system_manager_can_queue_and_await_events() {
        let mut manager = EntitySystemManager::new();
        let receiver = manager.await_event::<i32>(EventSource::Local);
        manager.queue_event(EventSource::Local, 12i32);
        manager.process_event_queue();
        assert_eq!(receiver.recv().unwrap(), 12);
    }

    #[test]
    fn entity_system_manager_can_raise_direct_component_events() {
        struct LocalSystem {
            info: EntitySystemInfo,
            hits: Arc<Mutex<Vec<(i32, bool)>>>,
        }

        impl EntitySystem for LocalSystem {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }

            fn info(&self) -> &EntitySystemInfo {
                &self.info
            }

            fn register_subscriptions(&self, subscriptions: &mut EntitySystemSubscriptions<'_>) {
                let hits = self.hits.clone();
                subscriptions.subscribe_local_event(
                    "MetaDataComponent",
                    move |uid: EntityUid, _component: &str, ev: &mut EntityPausedEvent| {
                        hits.lock().unwrap().push((uid.raw(), ev.paused));
                    },
                    std::iter::empty::<String>(),
                    std::iter::empty::<String>(),
                );
            }
        }

        let hits = Arc::new(Mutex::new(Vec::new()));
        let mut manager = EntitySystemManager::new();
        manager.add_system(Box::new(LocalSystem {
            info: EntitySystemInfo::new("metadata_direct"),
            hits: hits.clone(),
        }));

        let mut ev = EntityPausedEvent {
            entity: EntityUid::new(77),
            paused: true,
        };
        manager.raise_component_event(EntityUid::new(77), "MetaDataComponent", &mut ev);
        assert_eq!(&*hits.lock().unwrap(), &[(77, true)]);
    }
}
