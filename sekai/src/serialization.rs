use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha512};
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerializableComponentState {
    pub type_name: String,
    pub payload: Vec<u8>,
}

impl SerializableComponentState {
    pub fn new(type_name: impl Into<String>, payload: Vec<u8>) -> Self {
        Self {
            type_name: type_name.into(),
            payload,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MappedStringSerializer {
    strings: Vec<String>,
    reverse: HashMap<String, u32>,
    locked: bool,
    hash: Vec<u8>,
}

impl MappedStringSerializer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn locked(&self) -> bool {
        self.locked
    }

    pub fn add_string(&mut self, value: impl Into<String>) -> bool {
        let value = value.into();
        if self.locked || value.len() < 3 || value.len() > 420 {
            return false;
        }
        if self.reverse.contains_key(&value) {
            return false;
        }
        let index = self.strings.len() as u32 + 2;
        self.reverse.insert(value.clone(), index);
        self.strings.push(value);
        true
    }

    pub fn add_strings<I>(&mut self, strings: I)
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        for value in strings {
            let _ = self.add_string(value);
        }
    }

    pub fn lock_strings(&mut self) {
        if self.locked {
            return;
        }
        self.locked = true;
        let mut hasher = Sha512::new();
        for value in &self.strings {
            hasher.update(value.as_bytes());
            hasher.update([0]);
        }
        self.hash = hasher.finalize().to_vec();
    }

    pub fn mapped_strings_hash(&self) -> &[u8] {
        &self.hash
    }

    pub fn lookup_index(&self, value: &str) -> Option<u32> {
        self.reverse.get(value).copied()
    }

    pub fn lookup_string(&self, value: u32) -> Option<&str> {
        value
            .checked_sub(2)
            .and_then(|index| self.strings.get(index as usize))
            .map(String::as_str)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SerializerStats {
    pub bytes_serialized: usize,
    pub objects_serialized: usize,
    pub bytes_deserialized: usize,
    pub objects_deserialized: usize,
}

#[derive(Debug, Clone, Default)]
pub struct RobustSerializer {
    mapped_strings: MappedStringSerializer,
    serializable_types: BTreeSet<String>,
    stats: SerializerStats,
}

impl RobustSerializer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn initialize(&mut self) {
        self.mapped_strings.lock_strings();
    }

    pub fn register_type<T>(&mut self)
    where
        T: Serialize + DeserializeOwned + 'static,
    {
        self.serializable_types
            .insert(std::any::type_name::<T>().to_string());
    }

    pub fn can_serialize<T>(&self) -> bool
    where
        T: 'static,
    {
        self.serializable_types.contains(std::any::type_name::<T>())
    }

    pub fn serialize<T>(&mut self, value: &T) -> Result<Vec<u8>, bincode::Error>
    where
        T: Serialize,
    {
        let bytes = bincode::serialize(value)?;
        self.stats.bytes_serialized += bytes.len();
        self.stats.objects_serialized += 1;
        Ok(bytes)
    }

    pub fn serialize_direct<T>(&mut self, value: &T) -> Result<Vec<u8>, bincode::Error>
    where
        T: Serialize,
    {
        self.serialize(value)
    }

    pub fn deserialize<T>(&mut self, bytes: &[u8]) -> Result<T, bincode::Error>
    where
        T: DeserializeOwned,
    {
        let value = bincode::deserialize(bytes)?;
        self.stats.bytes_deserialized += bytes.len();
        self.stats.objects_deserialized += 1;
        Ok(value)
    }

    pub fn deserialize_direct<T>(&mut self, bytes: &[u8]) -> Result<T, bincode::Error>
    where
        T: DeserializeOwned,
    {
        self.deserialize(bytes)
    }

    pub fn serialize_component_state<T>(
        &mut self,
        value: &T,
    ) -> Result<SerializableComponentState, bincode::Error>
    where
        T: Serialize + 'static,
    {
        Ok(SerializableComponentState::new(
            std::any::type_name::<T>(),
            self.serialize(value)?,
        ))
    }

    pub fn deserialize_component_state<T>(
        &mut self,
        state: &SerializableComponentState,
    ) -> Result<T, bincode::Error>
    where
        T: DeserializeOwned + 'static,
    {
        self.deserialize(&state.payload)
    }

    pub fn stats(&self) -> SerializerStats {
        self.stats
    }
}

#[cfg(test)]
mod tests {
    use super::{MappedStringSerializer, RobustSerializer};
    use crate::game_state::PlayerState;
    use crate::{EntityUid, SessionStatus};

    #[test]
    fn mapped_string_serializer_locks_and_roundtrips_indices() {
        let mut serializer = MappedStringSerializer::new();
        assert!(serializer.add_string("appearance"));
        assert!(serializer.add_string("transform"));
        serializer.lock_strings();
        let index = serializer.lookup_index("appearance").unwrap();
        assert_eq!(serializer.lookup_string(index), Some("appearance"));
        assert!(!serializer.mapped_strings_hash().is_empty());
    }

    #[test]
    fn robust_serializer_roundtrips_registered_payloads() {
        let mut serializer = RobustSerializer::new();
        serializer.register_type::<PlayerState>();
        let player = PlayerState {
            user_id: "abc".to_string(),
            name: "pedel".to_string(),
            status: SessionStatus::Connected,
            ping: 4,
            controlled_entity: Some(EntityUid::new(5)),
        };
        let bytes = serializer.serialize(&player).unwrap();
        let decoded: PlayerState = serializer.deserialize(&bytes).unwrap();
        assert_eq!(decoded.name, "pedel");
        assert!(serializer.can_serialize::<PlayerState>());
        let state = serializer.serialize_component_state(&player).unwrap();
        let decoded_again: PlayerState = serializer.deserialize_component_state(&state).unwrap();
        assert_eq!(decoded_again.controlled_entity, Some(EntityUid::new(5)));
    }
}
