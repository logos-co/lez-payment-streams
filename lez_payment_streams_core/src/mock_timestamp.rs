use core::mem::size_of;

use serde::{Deserialize, Serialize};

use crate::{Timestamp, VersionId, DEFAULT_VERSION};

/// Wire layout for the read-only mock time source account (MVP).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MockTimestamp {
    pub version: VersionId,
    pub timestamp: Timestamp,
}

impl MockTimestamp {
    pub const SIZE: usize = size_of::<VersionId>() + size_of::<Timestamp>();

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(Self::SIZE);
        buf.extend_from_slice(&self.version.to_le_bytes());
        buf.extend_from_slice(&self.timestamp.to_le_bytes());
        buf
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() != Self::SIZE {
            return None;
        }
        let vs = size_of::<VersionId>();
        let version = VersionId::from_le_bytes(data[..vs].try_into().ok()?);
        let timestamp = Timestamp::from_le_bytes(data[vs..Self::SIZE].try_into().ok()?);
        Some(Self { version, timestamp })
    }

    pub fn new(timestamp: Timestamp) -> Self {
        Self::new_with_version(timestamp, DEFAULT_VERSION)
    }

    pub fn new_with_version(timestamp: Timestamp, version: VersionId) -> Self {
        Self { version, timestamp }
    }

    pub fn advance_by(&mut self, delta: Timestamp) -> Option<()> {
        self.timestamp = self.timestamp.checked_add(delta)?;
        Some(())
    }

    pub fn increment(&mut self) -> Option<()> {
        self.advance_by(Timestamp::from(1u64))
    }
}

#[cfg(test)]
mod tests {
    use super::{MockTimestamp, Timestamp};
    use crate::DEFAULT_VERSION;

    #[test]
    fn mock_timestamp_roundtrip() {
        let time_original = MockTimestamp::new_with_version(42, DEFAULT_VERSION);
        let wire_bytes = time_original.to_bytes();
        assert_eq!(
            MockTimestamp::from_bytes(&wire_bytes),
            Some(time_original)
        );
    }

    #[test]
    fn advance_by_updates_timestamp() {
        let mut time_start = MockTimestamp::new(100);
        assert!(time_start.advance_by(50).is_some());
        assert_eq!(time_start.timestamp, 150);
    }

    #[test]
    fn increment_equals_advance_by_one() {
        let mut time_a = MockTimestamp::new(10);
        let mut time_b = MockTimestamp::new(10);
        assert!(time_a.increment().is_some());
        assert!(time_b.advance_by(1).is_some());
        assert_eq!(time_a, time_b);
    }

    #[test]
    fn advance_by_overflow_returns_none() {
        let mut time_at_max = MockTimestamp::new(Timestamp::MAX);
        assert!(time_at_max.advance_by(1).is_none());
    }
}
