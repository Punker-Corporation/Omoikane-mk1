use sekai::{Component, EntityUid};

#[derive(Debug, Clone)]
pub struct ActorComponent {
    pub base: Component,
    pub player_user_id: String,
}

impl ActorComponent {
    pub fn new(owner: EntityUid, player_user_id: impl Into<String>) -> Self {
        let mut base = Component::new("ActorComponent");
        base.owner = owner;
        Self {
            base,
            player_user_id: player_user_id.into(),
        }
    }
}
