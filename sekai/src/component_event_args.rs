use crate::EntityUid;

pub trait IComponent: Send + Sync {
    fn name(&self) -> &str;
}

pub struct ComponentEventArgs<'a> {
    pub component: &'a dyn IComponent,
    pub owner: EntityUid,
}

impl<'a> ComponentEventArgs<'a> {
    pub fn new(component: &'a dyn IComponent, owner: EntityUid) -> Self {
        Self { component, owner }
    }
}

pub struct AddedComponentEventArgs<'a>(pub ComponentEventArgs<'a>);
pub struct RemovedComponentEventArgs<'a>(pub ComponentEventArgs<'a>);
pub struct DeletedComponentEventArgs<'a>(pub ComponentEventArgs<'a>);
