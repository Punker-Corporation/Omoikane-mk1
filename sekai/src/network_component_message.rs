use crate::{ComponentMessage, EntityUid};

pub trait NetChannel: Send + Sync {}
pub trait CommonSession: Send + Sync {}

#[derive(Clone)]
pub struct NetworkComponentMessage<C = (), S = (), M = ComponentMessage> {
    pub channel: C,
    pub entity_uid: EntityUid,
    pub net_id: u32,
    pub message: M,
    pub session: Option<S>,
}

impl<C, S, M> NetworkComponentMessage<C, S, M> {
    pub fn new(
        channel: C,
        entity_uid: EntityUid,
        net_id: u32,
        message: M,
        session: Option<S>,
    ) -> Self {
        Self {
            channel,
            entity_uid,
            net_id,
            message,
            session,
        }
    }
}
