use sekai::{Component, EntityUid};

#[derive(Debug, Clone)]
pub(crate) struct ActorComponent {
    #[allow(dead_code)]
    pub(crate) base: Component,
    pub(crate) player_user_id: String,
}

impl ActorComponent {
    pub(crate) fn new(owner: EntityUid, player_user_id: impl Into<String>) -> Self {
        let mut base = Component::new("ActorComponent");
        base.owner = owner;
        Self {
            base,
            player_user_id: player_user_id.into(),
        }
    }
}
