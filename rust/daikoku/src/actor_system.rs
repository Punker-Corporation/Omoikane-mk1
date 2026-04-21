use crate::{ActorComponent, PlayerManager, ServerEntityManager};
use sekai::EntityUid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachPlayerEvent {
    pub uid: EntityUid,
    pub player_user_id: String,
    pub force: bool,
    pub force_kicked: Option<String>,
    pub result: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DetachPlayerEvent {
    pub result: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerAttachedEvent {
    pub entity: EntityUid,
    pub player_user_id: String,
    pub kicked_user_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerDetachedEvent {
    pub entity: EntityUid,
    pub player_user_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActorAttachResult {
    pub result: bool,
    pub force_kicked: Option<String>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ActorSystem;

impl ActorSystem {
    pub fn new() -> Self {
        Self
    }

    pub fn attach(
        &self,
        entities: &mut ServerEntityManager,
        players: &mut PlayerManager,
        uid: EntityUid,
        user_id: &str,
        force: bool,
    ) -> ActorAttachResult {
        if entities.inner.deleted(uid) || players.get_session(user_id).is_none() {
            return ActorAttachResult::default();
        }

        let mut force_kicked = None;

        if let Some(existing) = entities.actors.get(&uid).cloned() {
            if !force {
                return ActorAttachResult::default();
            }

            force_kicked = Some(existing.player_user_id.clone());
            self.detach_entity(entities, players, uid);
        }

        let attached_elsewhere = players.get_session(user_id).and_then(|session| session.controlled_entity);
        if let Some(other_uid) = attached_elsewhere {
            let _ = self.detach_entity(entities, players, other_uid);
        }

        entities
            .actors
            .insert(uid, ActorComponent::new(uid, user_id.to_string()));

        let _ = players.set_attached_entity(user_id, Some(uid));

        ActorAttachResult {
            result: true,
            force_kicked,
        }
    }

    pub fn detach_entity(
        &self,
        entities: &mut ServerEntityManager,
        players: &mut PlayerManager,
        uid: EntityUid,
    ) -> Option<PlayerDetachedEvent> {
        let component = entities.actors.remove(&uid)?;
        let _ = players.set_attached_entity(&component.player_user_id, None);
        Some(PlayerDetachedEvent {
            entity: uid,
            player_user_id: component.player_user_id,
        })
    }

    pub fn detach_player(
        &self,
        entities: &mut ServerEntityManager,
        players: &mut PlayerManager,
        user_id: &str,
    ) -> bool {
        let Some(uid) = players.get_session(user_id).and_then(|session| session.controlled_entity) else {
            return true;
        };
        self.detach_entity(entities, players, uid).is_some()
    }

    pub fn player_for_entity<'a>(&self, entities: &'a ServerEntityManager, uid: EntityUid) -> Option<&'a str> {
        entities.actors.get(&uid).map(|actor| actor.player_user_id.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::ActorSystem;
    use crate::{PlayerManager, ServerEntityManager};

    #[test]
    fn actor_system_attaches_forces_and_detaches_players() {
        let mut players = PlayerManager::new(4);
        players.connect("u1", "pedel");
        players.connect("u2", "rika");

        let mut entities = ServerEntityManager::new();
        let uid = entities.create_entity(Some("mob"));
        entities.initialize_entity(uid);

        let system = ActorSystem::new();
        assert!(system.attach(&mut entities, &mut players, uid, "u1", false).result);
        assert_eq!(players.get_session("u1").unwrap().controlled_entity, Some(uid));

        let failed = system.attach(&mut entities, &mut players, uid, "u2", false);
        assert!(!failed.result);

        let forced = system.attach(&mut entities, &mut players, uid, "u2", true);
        assert!(forced.result);
        assert_eq!(forced.force_kicked.as_deref(), Some("u1"));
        assert_eq!(players.get_session("u1").unwrap().controlled_entity, None);
        assert_eq!(players.get_session("u2").unwrap().controlled_entity, Some(uid));

        assert!(system.detach_player(&mut entities, &mut players, "u2"));
        assert_eq!(players.get_session("u2").unwrap().controlled_entity, None);
    }
}
