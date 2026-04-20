use core::fmt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
pub struct EntityUid {
    uid: i32,
}

impl EntityUid {
    pub const CLIENT_UID: i32 = 2 << 29;
    pub const INVALID: Self = Self { uid: 0 };
    pub const FIRST_UID: Self = Self { uid: 1 };

    pub const fn new(uid: i32) -> Self {
        Self { uid }
    }

    pub fn parse(uid: &str) -> Result<Self, core::num::ParseIntError> {
        if let Some(stripped) = uid.strip_prefix('c') {
            Ok(Self::new(stripped.parse::<i32>()? | Self::CLIENT_UID))
        } else {
            Ok(Self::new(uid.parse()?))
        }
    }

    pub fn try_parse(uid: &str) -> Option<Self> {
        Self::parse(uid).ok()
    }

    pub fn is_valid(self) -> bool {
        self.uid > 0
    }

    pub fn is_client_side(self) -> bool {
        (self.uid & Self::CLIENT_UID) != 0
    }

    pub fn valid(self) -> bool {
        self.is_valid()
    }

    pub fn raw(self) -> i32 {
        self.uid
    }
}

impl From<EntityUid> for i32 {
    fn from(value: EntityUid) -> Self {
        value.uid
    }
}

impl fmt::Display for EntityUid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_client_side() {
            write!(f, "c{}", self.uid & !Self::CLIENT_UID)
        } else {
            write!(f, "{}", self.uid)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EntityUid;

    #[test]
    fn entity_uid_parsing_matches_csharp_style() {
        assert_eq!(EntityUid::parse("15").unwrap(), EntityUid::new(15));
        assert!(EntityUid::parse("c5").unwrap().is_client_side());
        assert_eq!(EntityUid::parse("c5").unwrap().to_string(), "c5");
    }
}
