use crate::{EntityEventBus, EntityUid, EventSource, OrderingData};
use std::any::{Any, TypeId};

#[derive(Debug, Clone, PartialEq)]
pub struct EntitySystemInfo {
    pub name: String,
    pub updates_before: Vec<TypeId>,
    pub updates_after: Vec<TypeId>,
    pub updates_outside_prediction: bool,
}

impl EntitySystemInfo {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            updates_before: Vec::new(),
            updates_after: Vec::new(),
            updates_outside_prediction: false,
        }
    }
}

pub struct EntitySystemSubscriptions<'a> {
    bus: &'a mut EntityEventBus,
    subscriber: String,
    order_key: String,
}

impl<'a> EntitySystemSubscriptions<'a> {
    pub fn new(bus: &'a mut EntityEventBus, subscriber: impl Into<String>, order_key: impl Into<String>) -> Self {
        Self {
            bus,
            subscriber: subscriber.into(),
            order_key: order_key.into(),
        }
    }

    pub fn subscribe_event<T>(
        &mut self,
        source: EventSource,
        handler: impl FnMut(&mut T) + Send + 'static,
        before: impl IntoIterator<Item = impl Into<String>>,
        after: impl IntoIterator<Item = impl Into<String>>,
    ) where
        T: std::any::Any + Send + 'static,
    {
        self.bus.subscribe_event(
            source,
            self.subscriber.clone(),
            Some(OrderingData::new(self.order_key.clone(), before, after)),
            handler,
        );
    }

    pub fn subscribe_local_event<T>(
        &mut self,
        component_name: impl Into<String>,
        handler: impl FnMut(EntityUid, &str, &mut T) + Send + 'static,
        before: impl IntoIterator<Item = impl Into<String>>,
        after: impl IntoIterator<Item = impl Into<String>>,
    ) where
        T: std::any::Any + Send + 'static,
    {
        self.bus.subscribe_local_event(
            component_name,
            self.subscriber.clone(),
            Some(OrderingData::new(self.order_key.clone(), before, after)),
            handler,
        );
    }

    pub fn subscribe_local_broadcast_event<T>(
        &mut self,
        handler: impl FnMut(&mut T) + Send + 'static,
        before: impl IntoIterator<Item = impl Into<String>>,
        after: impl IntoIterator<Item = impl Into<String>>,
    ) where
        T: std::any::Any + Send + 'static,
    {
        self.subscribe_event(EventSource::Local, handler, before, after);
    }

    pub fn subscribe_network_event<T>(
        &mut self,
        handler: impl FnMut(&mut T) + Send + 'static,
        before: impl IntoIterator<Item = impl Into<String>>,
        after: impl IntoIterator<Item = impl Into<String>>,
    ) where
        T: std::any::Any + Send + 'static,
    {
        self.subscribe_event(EventSource::Network, handler, before, after);
    }

    pub fn subscribe_all_event<T>(
        &mut self,
        handler: impl FnMut(&mut T) + Send + 'static,
        before: impl IntoIterator<Item = impl Into<String>>,
        after: impl IntoIterator<Item = impl Into<String>>,
    ) where
        T: std::any::Any + Send + 'static,
    {
        self.subscribe_event(EventSource::All, handler, before, after);
    }
}

pub trait EntitySystem: Any + Send {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn info(&self) -> &EntitySystemInfo;
    fn initialize(&mut self) {}
    fn shutdown(&mut self) {}
    fn update(&mut self, _frame_time: f32) {}
    fn frame_update(&mut self, _frame_time: f32) {}
    fn register_subscriptions(&self, _subscriptions: &mut EntitySystemSubscriptions<'_>) {}
}
