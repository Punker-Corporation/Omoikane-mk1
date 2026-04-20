use std::any::Any;
use std::sync::Arc;

pub trait ComponentState: Any + Send + Sync {}

impl<T> ComponentState for T where T: Any + Send + Sync {}

pub type AnyComponentState = dyn Any + Send + Sync;
pub type ComponentStateValue = Arc<AnyComponentState>;
