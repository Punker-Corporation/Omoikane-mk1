use crate::EntityUid;
use std::any::{Any, TypeId};
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::mpsc::{channel, Receiver};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventSource {
    None,
    Local,
    Network,
    All,
}

impl EventSource {
    fn matches(self, source: EventSource) -> bool {
        match self {
            EventSource::All => source != EventSource::None,
            EventSource::None => false,
            _ => self == source,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderingData {
    pub order_key: String,
    pub before: Vec<String>,
    pub after: Vec<String>,
}

impl OrderingData {
    pub fn new(
        order_key: impl Into<String>,
        before: impl IntoIterator<Item = impl Into<String>>,
        after: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            order_key: order_key.into(),
            before: before.into_iter().map(Into::into).collect(),
            after: after.into_iter().map(Into::into).collect(),
        }
    }
}

type BroadcastCallback = dyn FnMut(&mut dyn Any) + Send;
type LocalCallback = dyn FnMut(EntityUid, &str, &mut dyn Any) + Send;

struct BroadcastSubscription {
    subscriber: String,
    source: EventSource,
    ordering: Option<OrderingData>,
    callback: Box<BroadcastCallback>,
}

struct LocalSubscription {
    subscriber: String,
    component_name: String,
    ordering: Option<OrderingData>,
    callback: Box<LocalCallback>,
}

struct QueuedEvent {
    source: EventSource,
    event: Box<dyn Any + Send>,
}

struct AwaitingEvent {
    source: EventSource,
    notify: Box<dyn FnMut(&mut dyn Any) -> bool + Send>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum OrderedTarget {
    Local(usize),
    Broadcast(usize),
}

pub struct EntityEventBus {
    broadcast_subscriptions: HashMap<TypeId, Vec<BroadcastSubscription>>,
    local_subscriptions: HashMap<TypeId, Vec<LocalSubscription>>,
    event_queue: VecDeque<QueuedEvent>,
    awaiting_events: HashMap<TypeId, Vec<AwaitingEvent>>,
}

impl EntityEventBus {
    pub fn new() -> Self {
        Self {
            broadcast_subscriptions: HashMap::new(),
            local_subscriptions: HashMap::new(),
            event_queue: VecDeque::new(),
            awaiting_events: HashMap::new(),
        }
    }

    pub fn subscribe_event<T>(
        &mut self,
        source: EventSource,
        subscriber: impl Into<String>,
        ordering: Option<OrderingData>,
        mut handler: impl FnMut(&mut T) + Send + 'static,
    ) where
        T: Any + Send + 'static,
    {
        let subscriber = subscriber.into();
        let type_id = TypeId::of::<T>();
        let callback = move |event: &mut dyn Any| {
            if let Some(event) = event.downcast_mut::<T>() {
                handler(event);
            }
        };
        self.broadcast_subscriptions
            .entry(type_id)
            .or_default()
            .push(BroadcastSubscription {
                subscriber,
                source,
                ordering,
                callback: Box::new(callback),
            });
    }

    pub fn subscribe_local_event<T>(
        &mut self,
        component_name: impl Into<String>,
        subscriber: impl Into<String>,
        ordering: Option<OrderingData>,
        mut handler: impl FnMut(EntityUid, &str, &mut T) + Send + 'static,
    ) where
        T: Any + Send + 'static,
    {
        let component_name = component_name.into();
        let subscriber = subscriber.into();
        let type_id = TypeId::of::<T>();
        let callback = move |uid: EntityUid, component: &str, event: &mut dyn Any| {
            if let Some(event) = event.downcast_mut::<T>() {
                handler(uid, component, event);
            }
        };
        self.local_subscriptions
            .entry(type_id)
            .or_default()
            .push(LocalSubscription {
                subscriber,
                component_name,
                ordering,
                callback: Box::new(callback),
            });
    }

    pub fn unsubscribe_subscriber(&mut self, subscriber: &str) {
        for subscriptions in self.broadcast_subscriptions.values_mut() {
            subscriptions.retain(|subscription| subscription.subscriber != subscriber);
        }
        for subscriptions in self.local_subscriptions.values_mut() {
            subscriptions.retain(|subscription| subscription.subscriber != subscriber);
        }
    }

    pub fn await_event<T>(&mut self, source: EventSource) -> Receiver<T>
    where
        T: Any + Clone + Send + 'static,
    {
        let (sender, receiver) = channel::<T>();
        self.awaiting_events
            .entry(TypeId::of::<T>())
            .or_default()
            .push(AwaitingEvent {
                source,
                notify: Box::new(move |event| {
                    if let Some(event) = event.downcast_mut::<T>() {
                        let _ = sender.send(event.clone());
                        return true;
                    }
                    false
                }),
            });
        receiver
    }

    pub fn unsubscribe_event<T>(&mut self, source: EventSource, subscriber: &str)
    where
        T: Any + Send + 'static,
    {
        if let Some(subscriptions) = self.broadcast_subscriptions.get_mut(&TypeId::of::<T>()) {
            subscriptions.retain(|subscription| {
                !(subscription.subscriber == subscriber && subscription.source == source)
            });
        }
    }

    pub fn unsubscribe_local_event<T>(&mut self, component_name: &str, subscriber: &str)
    where
        T: Any + Send + 'static,
    {
        if let Some(subscriptions) = self.local_subscriptions.get_mut(&TypeId::of::<T>()) {
            subscriptions.retain(|subscription| {
                !(subscription.subscriber == subscriber && subscription.component_name == component_name)
            });
        }
    }

    pub fn raise_event<T>(&mut self, source: EventSource, event: &mut T)
    where
        T: Any + Send + 'static,
    {
        let type_id = TypeId::of::<T>();
        if let Some(subscriptions) = self.broadcast_subscriptions.get_mut(&type_id) {
            let order = ordered_indices_broadcast(subscriptions, source);
            for idx in order {
                if let Some(subscription) = subscriptions.get_mut(idx) {
                    (subscription.callback)(event);
                }
            }
        }
        self.process_awaiting_messages(source, event);
    }

    pub fn queue_event<T>(&mut self, source: EventSource, event: T)
    where
        T: Any + Send + 'static,
    {
        self.event_queue.push_back(QueuedEvent {
            source,
            event: Box::new(event),
        });
    }

    pub fn process_event_queue(&mut self) {
        while let Some(mut queued) = self.event_queue.pop_front() {
            let type_id = (*queued.event).type_id();
            if let Some(subscriptions) = self.broadcast_subscriptions.get_mut(&type_id) {
                let order = ordered_indices_broadcast(subscriptions, queued.source);
                for idx in order {
                    if let Some(subscription) = subscriptions.get_mut(idx) {
                        (subscription.callback)(queued.event.as_mut());
                    }
                }
            }
            self.process_awaiting_messages_boxed(queued.source, queued.event.as_mut(), type_id);
        }
    }

    pub fn raise_local_event<T>(
        &mut self,
        uid: EntityUid,
        component_names: &[String],
        event: &mut T,
        broadcast: bool,
    ) where
        T: Any + Send + 'static,
    {
        let type_id = TypeId::of::<T>();
        let local_targets = self
            .local_subscriptions
            .get(&type_id)
            .map(|subscriptions| {
                subscriptions
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, subscription)| {
                        component_names
                            .iter()
                            .any(|name| name == &subscription.component_name)
                            .then_some(OrderedTarget::Local(idx))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let broadcast_targets = if broadcast {
            self.broadcast_subscriptions
                .get(&type_id)
                .map(|subscriptions| {
                    subscriptions
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, subscription)| {
                            subscription.source.matches(EventSource::Local).then_some(OrderedTarget::Broadcast(idx))
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let ordered_targets = self.ordered_local_and_broadcast_targets(
            &type_id,
            &local_targets,
            &broadcast_targets,
        );

        for target in ordered_targets {
            match target {
                OrderedTarget::Local(idx) => {
                    if let Some(subscriptions) = self.local_subscriptions.get_mut(&type_id) {
                        if let Some(subscription) = subscriptions.get_mut(idx) {
                            let component_name = subscription.component_name.clone();
                            (subscription.callback)(uid, &component_name, event);
                        }
                    }
                }
                OrderedTarget::Broadcast(idx) => {
                    if let Some(subscriptions) = self.broadcast_subscriptions.get_mut(&type_id) {
                        if let Some(subscription) = subscriptions.get_mut(idx) {
                            (subscription.callback)(event);
                        }
                    }
                }
            }
        }

        if broadcast {
            self.process_awaiting_messages(EventSource::Local, event);
        }
    }

    pub fn raise_component_event<T>(
        &mut self,
        uid: EntityUid,
        component_name: &str,
        event: &mut T,
    ) where
        T: Any + Send + 'static,
    {
        let type_id = TypeId::of::<T>();
        if let Some(subscriptions) = self.local_subscriptions.get_mut(&type_id) {
            let mut targets = subscriptions
                .iter()
                .enumerate()
                .filter_map(|(idx, subscription)| {
                    (subscription.component_name == component_name).then_some(OrderedTarget::Local(idx))
                })
                .collect::<Vec<_>>();

            targets = topological_indices(
                &targets,
                |target| match target {
                    OrderedTarget::Local(idx) => subscriptions[*idx]
                        .ordering
                        .as_ref()
                        .map(|ordering| ordering.order_key.clone()),
                    OrderedTarget::Broadcast(_) => None,
                },
                |target| match target {
                    OrderedTarget::Local(idx) => subscriptions[*idx]
                        .ordering
                        .as_ref()
                        .map(|ordering| ordering.before.clone())
                        .unwrap_or_default(),
                    OrderedTarget::Broadcast(_) => Vec::new(),
                },
                |target| match target {
                    OrderedTarget::Local(idx) => subscriptions[*idx]
                        .ordering
                        .as_ref()
                        .map(|ordering| ordering.after.clone())
                        .unwrap_or_default(),
                    OrderedTarget::Broadcast(_) => Vec::new(),
                },
            );

            for target in targets {
                if let OrderedTarget::Local(idx) = target {
                    if let Some(subscription) = subscriptions.get_mut(idx) {
                        let component_name = subscription.component_name.clone();
                        (subscription.callback)(uid, &component_name, event);
                    }
                }
            }
        }
    }

    fn ordered_local_and_broadcast_targets(
        &self,
        type_id: &TypeId,
        local_targets: &[OrderedTarget],
        broadcast_targets: &[OrderedTarget],
    ) -> Vec<OrderedTarget> {
        let mut targets = Vec::with_capacity(local_targets.len() + broadcast_targets.len());
        targets.extend_from_slice(local_targets);
        targets.extend_from_slice(broadcast_targets);

        topological_indices(
            &targets,
            |target| match target {
                OrderedTarget::Local(idx) => self
                    .local_subscriptions
                    .get(type_id)
                    .and_then(|subs| subs.get(*idx))
                    .and_then(|sub| sub.ordering.as_ref().map(|ordering| ordering.order_key.clone())),
                OrderedTarget::Broadcast(idx) => self
                    .broadcast_subscriptions
                    .get(type_id)
                    .and_then(|subs| subs.get(*idx))
                    .and_then(|sub| sub.ordering.as_ref().map(|ordering| ordering.order_key.clone())),
            },
            |target| match target {
                OrderedTarget::Local(idx) => self
                    .local_subscriptions
                    .get(type_id)
                    .and_then(|subs| subs.get(*idx))
                    .and_then(|sub| sub.ordering.as_ref().map(|ordering| ordering.before.clone()))
                    .unwrap_or_default(),
                OrderedTarget::Broadcast(idx) => self
                    .broadcast_subscriptions
                    .get(type_id)
                    .and_then(|subs| subs.get(*idx))
                    .and_then(|sub| sub.ordering.as_ref().map(|ordering| ordering.before.clone()))
                    .unwrap_or_default(),
            },
            |target| match target {
                OrderedTarget::Local(idx) => self
                    .local_subscriptions
                    .get(type_id)
                    .and_then(|subs| subs.get(*idx))
                    .and_then(|sub| sub.ordering.as_ref().map(|ordering| ordering.after.clone()))
                    .unwrap_or_default(),
                OrderedTarget::Broadcast(idx) => self
                    .broadcast_subscriptions
                    .get(type_id)
                    .and_then(|subs| subs.get(*idx))
                    .and_then(|sub| sub.ordering.as_ref().map(|ordering| ordering.after.clone()))
                    .unwrap_or_default(),
            },
        )
    }

    fn process_awaiting_messages<T>(&mut self, source: EventSource, event: &mut T)
    where
        T: Any + Send + 'static,
    {
        self.process_awaiting_messages_boxed(source, event, TypeId::of::<T>());
    }

    fn process_awaiting_messages_boxed(
        &mut self,
        source: EventSource,
        event: &mut dyn Any,
        type_id: TypeId,
    ) {
        if let Some(awaiters) = self.awaiting_events.get_mut(&type_id) {
            let mut current = std::mem::take(awaiters);
            let mut remaining = Vec::new();
            for mut awaiter in current.drain(..) {
                if awaiter.source.matches(source) && (awaiter.notify)(event) {
                    continue;
                }
                remaining.push(awaiter);
            }
            *awaiters = remaining;
        }
    }
}

impl Default for EntityEventBus {
    fn default() -> Self {
        Self::new()
    }
}

fn ordered_indices_broadcast(
    subscriptions: &[BroadcastSubscription],
    source: EventSource,
) -> Vec<usize> {
    let mut indices = subscriptions
        .iter()
        .enumerate()
        .filter_map(|(idx, subscription)| subscription.source.matches(source).then_some(idx))
        .collect::<Vec<_>>();
    indices.sort_by_key(|idx| *idx);
    topological_indices(
        &indices,
        |idx| subscriptions[*idx]
            .ordering
            .as_ref()
            .map(|ordering| ordering.order_key.clone()),
        |idx| subscriptions[*idx]
            .ordering
            .as_ref()
            .map(|ordering| ordering.before.clone())
            .unwrap_or_default(),
        |idx| subscriptions[*idx]
            .ordering
            .as_ref()
            .map(|ordering| ordering.after.clone())
            .unwrap_or_default(),
    )
}

fn topological_indices<T>(
    indices: &[T],
    order_key: impl Fn(&T) -> Option<String>,
    before: impl Fn(&T) -> Vec<String>,
    after: impl Fn(&T) -> Vec<String>,
) -> Vec<T>
where
    T: Copy + Eq + Hash,
{
    let mut name_to_index = HashMap::new();
    for idx in indices {
        if let Some(key) = order_key(idx) {
            name_to_index.insert(key, *idx);
        }
    }

    let mut outgoing: HashMap<T, Vec<T>> = HashMap::new();
    let mut indegree: HashMap<T, usize> = indices.iter().map(|idx| (*idx, 0)).collect();

    for idx in indices {
        for name in before(idx) {
            if let Some(target) = name_to_index.get(&name) {
                outgoing.entry(*idx).or_default().push(*target);
                *indegree.entry(*target).or_default() += 1;
            }
        }
        for name in after(idx) {
            if let Some(source) = name_to_index.get(&name) {
                outgoing.entry(*source).or_default().push(*idx);
                *indegree.entry(*idx).or_default() += 1;
            }
        }
    }

    let mut ready = indices
        .iter()
        .filter_map(|idx| (indegree.get(idx).copied().unwrap_or_default() == 0).then_some(*idx))
        .collect::<Vec<_>>();

    let mut ordered = Vec::with_capacity(indices.len());
    while let Some(idx) = ready.first().copied() {
        ready.remove(0);
        ordered.push(idx);
        if let Some(targets) = outgoing.get(&idx) {
            for target in targets {
                if let Some(entry) = indegree.get_mut(target) {
                    *entry -= 1;
                    if *entry == 0 {
                        ready.push(*target);
                    }
                }
            }
        }
    }

    if ordered.len() != indices.len() {
        return indices.to_vec();
    }

    ordered
}

#[cfg(test)]
mod tests {
    use super::{EntityEventBus, EventSource, OrderingData};
    use crate::EntityUid;
    use std::sync::{Arc, Mutex};

    #[test]
    fn event_bus_broadcast_dispatches_and_respects_ordering() {
        let mut bus = EntityEventBus::new();
        let calls = Arc::new(Mutex::new(Vec::<String>::new()));

        {
            let calls = calls.clone();
            bus.subscribe_event(
                EventSource::Local,
                "late",
                Some(OrderingData::new("late", std::iter::empty::<String>(), ["early"])),
                move |_: &mut i32| calls.lock().unwrap().push("late".to_string()),
            );
        }
        {
            let calls = calls.clone();
            bus.subscribe_event(
                EventSource::Local,
                "early",
                Some(OrderingData::new("early", ["late"], std::iter::empty::<String>())),
                move |_: &mut i32| calls.lock().unwrap().push("early".to_string()),
            );
        }

        let mut ev = 5i32;
        bus.raise_event(EventSource::Local, &mut ev);
        assert_eq!(&*calls.lock().unwrap(), &["early".to_string(), "late".to_string()]);
    }

    #[test]
    fn event_bus_local_dispatch_filters_by_component() {
        let mut bus = EntityEventBus::new();
        let hits = Arc::new(Mutex::new(Vec::<(i32, String)>::new()));
        {
            let hits = hits.clone();
            bus.subscribe_local_event(
                "Physics",
                "physics_sys",
                None,
                move |uid: EntityUid, component: &str, event: &mut i32| {
                    hits.lock().unwrap().push((uid.raw(), component.to_string()));
                    *event += 1;
                },
            );
        }

        let mut ev = 1i32;
        bus.raise_local_event(
            EntityUid::new(42),
            &[String::from("MetaDataComponent"), String::from("Physics")],
            &mut ev,
            false,
        );
        assert_eq!(ev, 2);
        assert_eq!(&*hits.lock().unwrap(), &[(42, "Physics".to_string())]);
    }

    #[test]
    fn event_bus_unsubscribe_removes_handlers() {
        let mut bus = EntityEventBus::new();
        bus.subscribe_event(EventSource::Local, "sub", None, |ev: &mut i32| *ev += 1);
        bus.unsubscribe_subscriber("sub");
        let mut ev = 0i32;
        bus.raise_event(EventSource::Local, &mut ev);
        assert_eq!(ev, 0);
    }

    #[test]
    fn event_bus_local_ordering_combines_local_and_broadcast_handlers() {
        let mut bus = EntityEventBus::new();
        let calls = Arc::new(Mutex::new(Vec::<String>::new()));

        {
            let calls = calls.clone();
            bus.subscribe_event(
                EventSource::Local,
                "broadcast_late",
                Some(OrderingData::new("broadcast_late", std::iter::empty::<String>(), ["local_early"])),
                move |_: &mut i32| calls.lock().unwrap().push("broadcast_late".to_string()),
            );
        }
        {
            let calls = calls.clone();
            bus.subscribe_local_event(
                "Physics",
                "local_early",
                Some(OrderingData::new("local_early", ["broadcast_late"], std::iter::empty::<String>())),
                move |_uid: EntityUid, _component: &str, _: &mut i32| {
                    calls.lock().unwrap().push("local_early".to_string())
                },
            );
        }

        let mut ev = 0i32;
        bus.raise_local_event(EntityUid::new(1), &[String::from("Physics")], &mut ev, true);
        assert_eq!(
            &*calls.lock().unwrap(),
            &["local_early".to_string(), "broadcast_late".to_string()]
        );
    }

    #[test]
    fn event_bus_await_event_receives_broadcast_and_clears_waiter() {
        let mut bus = EntityEventBus::new();
        let rx = bus.await_event::<i32>(EventSource::Local);

        let mut ev = 7i32;
        bus.raise_event(EventSource::Local, &mut ev);

        assert_eq!(rx.recv().unwrap(), 7);

        let rx2 = bus.await_event::<i32>(EventSource::Local);
        bus.queue_event(EventSource::Local, 11i32);
        bus.process_event_queue();
        assert_eq!(rx2.recv().unwrap(), 11);
    }

    #[test]
    fn event_bus_can_raise_direct_component_event_without_broadcast() {
        let mut bus = EntityEventBus::new();
        let hits = Arc::new(Mutex::new(Vec::<String>::new()));
        {
            let hits = hits.clone();
            bus.subscribe_local_event(
                "Physics",
                "physics_sys",
                None,
                move |_uid: EntityUid, component: &str, ev: &mut i32| {
                    hits.lock().unwrap().push(component.to_string());
                    *ev += 1;
                },
            );
        }
        {
            let hits = hits.clone();
            bus.subscribe_event(EventSource::Local, "broadcast_sys", None, move |_: &mut i32| {
                hits.lock().unwrap().push("broadcast".to_string());
            });
        }

        let mut ev = 2i32;
        bus.raise_component_event(EntityUid::new(9), "Physics", &mut ev);

        assert_eq!(ev, 3);
        assert_eq!(&*hits.lock().unwrap(), &["Physics".to_string()]);
    }
}
