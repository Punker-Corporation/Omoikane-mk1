use crate::{
    actor_component::ActorComponent, player_manager::PlayerManager,
    server_entity_manager::ServerEntityManager,
};
use sekai::EntityUid;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ActorAttachResult {
    pub(crate) result: bool,
    pub(crate) force_kicked: Option<String>,
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ActorSystem;

impl ActorSystem {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn attach(
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

        let attached_elsewhere = players
            .get_session(user_id)
            .and_then(|session| session.controlled_entity);
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

    fn detach_entity(
        &self,
        entities: &mut ServerEntityManager,
        players: &mut PlayerManager,
        uid: EntityUid,
    ) -> bool {
        let Some(component) = entities.actors.remove(&uid) else {
            return false;
        };
        let _ = players.set_attached_entity(&component.player_user_id, None);
        true
    }

    pub(crate) fn detach_player(
        &self,
        entities: &mut ServerEntityManager,
        players: &mut PlayerManager,
        user_id: &str,
    ) -> bool {
        let Some(uid) = players
            .get_session(user_id)
            .and_then(|session| session.controlled_entity)
        else {
            return true;
        };
        self.detach_entity(entities, players, uid)
    }
}

#[cfg(test)]
mod tests {
    use super::ActorSystem;
    use crate::{player_manager::PlayerManager, server_entity_manager::ServerEntityManager};

    #[test]
    fn actor_system_attaches_forces_and_detaches_players() {
        let mut players = PlayerManager::new(4);
        players.connect("u1", "pedel");
        players.connect("u2", "rika");

        let mut entities = ServerEntityManager::new();
        let uid = entities.inner.create_entity_uninitialized(Some("mob"));
        entities.inner.initialize_entity(uid);

        let system = ActorSystem::new();
        assert!(
            system
                .attach(&mut entities, &mut players, uid, "u1", false)
                .result
        );
        assert_eq!(
            players.get_session("u1").unwrap().controlled_entity,
            Some(uid)
        );

        let failed = system.attach(&mut entities, &mut players, uid, "u2", false);
        assert!(!failed.result);

        let forced = system.attach(&mut entities, &mut players, uid, "u2", true);
        assert!(forced.result);
        assert_eq!(forced.force_kicked.as_deref(), Some("u1"));
        assert_eq!(players.get_session("u1").unwrap().controlled_entity, None);
        assert_eq!(
            players.get_session("u2").unwrap().controlled_entity,
            Some(uid)
        );

        assert!(system.detach_player(&mut entities, &mut players, "u2"));
        assert_eq!(players.get_session("u2").unwrap().controlled_entity, None);
    }
}
