pub trait EntityEventArgs {}

impl<T> EntityEventArgs for T where T: Send + Sync + 'static {}

#[derive(Debug, Clone, Default)]
pub struct CancellableEntityEventArgs {
    cancelled: bool,
}

impl CancellableEntityEventArgs {
    pub fn cancelled(&self) -> bool {
        self.cancelled
    }

    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    pub fn uncancel(&mut self) {
        self.cancelled = false;
    }
}

#[derive(Debug, Clone)]
pub struct EntitySessionEventArgs<S> {
    pub sender_session: S,
}

impl<S> EntitySessionEventArgs<S> {
    pub fn new(sender_session: S) -> Self {
        Self { sender_session }
    }
}

#[derive(Debug, Clone)]
pub struct EntitySessionMessage<S, T> {
    pub event_args: EntitySessionEventArgs<S>,
    pub message: T,
}

impl<S, T> EntitySessionMessage<S, T> {
    pub fn new(event_args: EntitySessionEventArgs<S>, message: T) -> Self {
        Self { event_args, message }
    }
}
