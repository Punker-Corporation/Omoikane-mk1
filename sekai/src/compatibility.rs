use serde::{Deserialize, Serialize};
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl ProtocolVersion {
    pub const CURRENT: Self = Self {
        major: 1,
        minor: 0,
        patch: 0,
    };

    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub const fn is_wire_compatible_with(self, other: Self) -> bool {
        self.major == other.major
    }

    pub const fn negotiated_with(self, other: Self) -> Self {
        Self {
            major: self.major,
            minor: if self.minor < other.minor {
                self.minor
            } else {
                other.minor
            },
            patch: if self.patch < other.patch {
                self.patch
            } else {
                other.patch
            },
        }
    }
}

impl Default for ProtocolVersion {
    fn default() -> Self {
        Self::CURRENT
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ProtocolFeature {
    BinaryComponentState = 0,
    IncrementalSnapshots = 1,
    ChunkDeletionDeltas = 2,
    PlayerStateDeltas = 3,
    ClientPredictionReplay = 4,
    ContactEventStream = 5,
    DeterministicStateDigest = 6,
    ContentAddressedChunks = 7,
}

impl ProtocolFeature {
    pub const ALL: [Self; 8] = [
        Self::BinaryComponentState,
        Self::IncrementalSnapshots,
        Self::ChunkDeletionDeltas,
        Self::PlayerStateDeltas,
        Self::ClientPredictionReplay,
        Self::ContactEventStream,
        Self::DeterministicStateDigest,
        Self::ContentAddressedChunks,
    ];

    pub const fn bit(self) -> u64 {
        1 << self as u8
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::BinaryComponentState => "binary-component-state",
            Self::IncrementalSnapshots => "incremental-snapshots",
            Self::ChunkDeletionDeltas => "chunk-deletion-deltas",
            Self::PlayerStateDeltas => "player-state-deltas",
            Self::ClientPredictionReplay => "client-prediction-replay",
            Self::ContactEventStream => "contact-event-stream",
            Self::DeterministicStateDigest => "deterministic-state-digest",
            Self::ContentAddressedChunks => "content-addressed-chunks",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureSet(pub u64);

impl FeatureSet {
    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    pub const fn from_feature(feature: ProtocolFeature) -> Self {
        Self(feature.bit())
    }

    pub const fn stable() -> Self {
        Self(
            ProtocolFeature::BinaryComponentState.bit()
                | ProtocolFeature::IncrementalSnapshots.bit()
                | ProtocolFeature::ChunkDeletionDeltas.bit()
                | ProtocolFeature::PlayerStateDeltas.bit(),
        )
    }

    pub const fn experimental() -> Self {
        Self(
            Self::stable().0
                | ProtocolFeature::ClientPredictionReplay.bit()
                | ProtocolFeature::ContactEventStream.bit()
                | ProtocolFeature::DeterministicStateDigest.bit()
                | ProtocolFeature::ContentAddressedChunks.bit(),
        )
    }

    pub const fn bits(self) -> u64 {
        self.0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn contains(self, feature: ProtocolFeature) -> bool {
        (self.0 & feature.bit()) == feature.bit()
    }

    pub const fn contains_all(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn insert(&mut self, feature: ProtocolFeature) {
        self.0 |= feature.bit();
    }

    pub fn remove(&mut self, feature: ProtocolFeature) {
        self.0 &= !feature.bit();
    }

    pub fn names(self) -> Vec<&'static str> {
        ProtocolFeature::ALL
            .into_iter()
            .filter(|feature| self.contains(*feature))
            .map(ProtocolFeature::name)
            .collect()
    }
}

impl BitOr for FeatureSet {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for FeatureSet {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for FeatureSet {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for FeatureSet {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Sub for FeatureSet {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 & !rhs.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityProfile {
    pub protocol: ProtocolVersion,
    pub supported_features: FeatureSet,
    pub required_features: FeatureSet,
    pub max_state_payload_size: usize,
    pub tick_rate: u16,
}

impl CompatibilityProfile {
    pub const DEFAULT_MAX_STATE_PAYLOAD_SIZE: usize = 1300;
    pub const DEFAULT_TICK_RATE: u16 = 60;

    pub fn stable() -> Self {
        Self {
            protocol: ProtocolVersion::CURRENT,
            supported_features: FeatureSet::stable(),
            required_features: FeatureSet::from_feature(ProtocolFeature::BinaryComponentState),
            max_state_payload_size: Self::DEFAULT_MAX_STATE_PAYLOAD_SIZE,
            tick_rate: Self::DEFAULT_TICK_RATE,
        }
    }

    pub fn experimental() -> Self {
        Self {
            supported_features: FeatureSet::experimental(),
            required_features: FeatureSet::from_feature(ProtocolFeature::BinaryComponentState)
                | FeatureSet::from_feature(ProtocolFeature::IncrementalSnapshots),
            ..Self::stable()
        }
    }

    pub fn negotiate(&self, remote: &Self) -> CompatibilityReport {
        let shared_features = self.supported_features & remote.supported_features;
        let required_features = self.required_features | remote.required_features;
        let missing_required_features = required_features - shared_features;
        let protocol_compatible = self.protocol.is_wire_compatible_with(remote.protocol);
        let accepted = protocol_compatible && missing_required_features.is_empty();
        let negotiated = accepted.then(|| Self {
            protocol: self.protocol.negotiated_with(remote.protocol),
            supported_features: shared_features,
            required_features,
            max_state_payload_size: self
                .max_state_payload_size
                .min(remote.max_state_payload_size),
            tick_rate: self.tick_rate.min(remote.tick_rate),
        });

        CompatibilityReport {
            accepted,
            protocol_compatible,
            negotiated,
            local_protocol: self.protocol,
            remote_protocol: remote.protocol,
            shared_features,
            missing_required_features,
        }
    }
}

impl Default for CompatibilityProfile {
    fn default() -> Self {
        Self::experimental()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityReport {
    pub accepted: bool,
    pub protocol_compatible: bool,
    pub negotiated: Option<CompatibilityProfile>,
    pub local_protocol: ProtocolVersion,
    pub remote_protocol: ProtocolVersion,
    pub shared_features: FeatureSet,
    pub missing_required_features: FeatureSet,
}

#[cfg(test)]
mod tests {
    use super::{CompatibilityProfile, FeatureSet, ProtocolFeature, ProtocolVersion};

    #[test]
    fn feature_sets_track_stable_and_experimental_bits() {
        let stable = FeatureSet::stable();
        assert!(stable.contains(ProtocolFeature::BinaryComponentState));
        assert!(!stable.contains(ProtocolFeature::DeterministicStateDigest));

        let experimental = FeatureSet::experimental();
        assert!(experimental.contains_all(stable));
        assert!(
            experimental
                .names()
                .contains(&ProtocolFeature::ContentAddressedChunks.name())
        );
    }

    #[test]
    fn compatibility_profiles_negotiate_shared_features() {
        let server = CompatibilityProfile::experimental();
        let mut client = CompatibilityProfile::stable();
        client.max_state_payload_size = 900;

        let report = server.negotiate(&client);
        assert!(report.accepted);
        let negotiated = report.negotiated.unwrap();
        assert_eq!(negotiated.max_state_payload_size, 900);
        assert!(
            negotiated
                .supported_features
                .contains(ProtocolFeature::IncrementalSnapshots)
        );
        assert!(
            !negotiated
                .supported_features
                .contains(ProtocolFeature::ContentAddressedChunks)
        );
    }

    #[test]
    fn compatibility_profiles_reject_major_protocol_mismatch() {
        let server = CompatibilityProfile::experimental();
        let mut client = CompatibilityProfile::experimental();
        client.protocol = ProtocolVersion::new(2, 0, 0);

        let report = server.negotiate(&client);
        assert!(!report.accepted);
        assert!(!report.protocol_compatible);
    }

    #[test]
    fn compatibility_profiles_report_missing_required_features() {
        let mut server = CompatibilityProfile::experimental();
        let client = CompatibilityProfile::stable();
        server.required_features |=
            FeatureSet::from_feature(ProtocolFeature::ClientPredictionReplay);

        let report = server.negotiate(&client);
        assert!(!report.accepted);
        assert!(
            report
                .missing_required_features
                .contains(ProtocolFeature::ClientPredictionReplay)
        );
    }
}
