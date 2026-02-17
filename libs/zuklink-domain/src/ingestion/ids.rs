use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for a Segment
///
/// SegmentId is a wrapper around UUID v7 to provide type safety and prevent
/// mixing up segment IDs with other UUIDs in the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SegmentId(Uuid);

impl SegmentId {
    /// Generate a new random SegmentId
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Create a SegmentId from an existing UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the inner UUID value
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for SegmentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SegmentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for SegmentId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<SegmentId> for Uuid {
    fn from(id: SegmentId) -> Self {
        id.0
    }
}
