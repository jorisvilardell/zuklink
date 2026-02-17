//! Domain entities for data ingestion
//!
//! This module defines the core domain model for data segments in ZukLink.
//! A Segment represents an immutable unit of data that has been ingested
//! and stored in the distributed storage system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ingestion::ids::SegmentId;

/// A Segment represents an immutable chunk of ingested data
///
/// Segments are the fundamental unit of data in ZukLink. They are:
/// - **Immutable**: Once created, a segment never changes
/// - **Self-contained**: Each segment has all metadata needed to process it
/// - **Flat stored**: Segments are stored with UUID-based names in flat S3 structure
///
/// # Example
///
/// ```rust
/// use zuklink_domain::ingestion::entity::Segment;
///
/// let segment = Segment::new(vec![1, 2, 3, 4]);
/// println!("Created segment: {}", segment.id());
/// println!("Size: {} bytes", segment.size());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    /// Unique identifier for this segment
    id: SegmentId,

    /// Size of the data in bytes
    size_bytes: usize,

    /// Timestamp when the segment was created
    created_at: DateTime<Utc>,

    /// Storage key/path where this segment is stored
    /// This is optional as it's set after storage, not at creation
    storage_key: Option<String>,
}

impl Segment {
    /// Create a new Segment with the given data
    ///
    /// This is a pure domain constructor - it doesn't perform any I/O.
    /// The actual data is not stored in the entity to keep it lightweight.
    ///
    /// # Arguments
    ///
    /// * `data` - The raw bytes to be stored (used only to calculate size)
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            id: SegmentId::new(),
            size_bytes: data.len(),
            created_at: Utc::now(),
            storage_key: None,
        }
    }

    /// Create a Segment with explicit values (used for reconstruction)
    pub fn from_parts(
        id: SegmentId,
        size_bytes: usize,
        created_at: DateTime<Utc>,
        storage_key: Option<String>,
    ) -> Self {
        Self {
            id,
            size_bytes,
            created_at,
            storage_key,
        }
    }

    /// Get the segment's unique identifier
    pub fn id(&self) -> &SegmentId {
        &self.id
    }

    /// Get the size of the segment in bytes
    pub fn size(&self) -> usize {
        self.size_bytes
    }

    /// Get the creation timestamp
    pub fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }

    /// Get the storage key (if set)
    pub fn storage_key(&self) -> Option<&str> {
        self.storage_key.as_deref()
    }

    /// Set the storage key after the segment has been persisted
    ///
    /// This is typically called by the infrastructure layer after successful storage.
    pub fn set_storage_key(&mut self, key: String) {
        self.storage_key = Some(key);
    }

    /// Check if this segment has been persisted to storage
    pub fn is_persisted(&self) -> bool {
        self.storage_key.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_id_generation() {
        let id1 = SegmentId::new();
        let id2 = SegmentId::new();

        assert_ne!(id1, id2, "Each SegmentId should be unique");
    }

    #[test]
    fn test_segment_id_display() {
        let id = SegmentId::new();
        let display_str = format!("{}", id);

        // Should be a valid UUID string format
        assert_eq!(display_str.len(), 36); // UUID string length with hyphens
    }

    #[test]
    fn test_segment_creation() {
        let data = vec![1, 2, 3, 4, 5];
        let segment = Segment::new(data.clone());

        assert_eq!(segment.size(), data.len());
        assert!(!segment.is_persisted());
        assert!(segment.storage_key().is_none());
    }

    #[test]
    fn test_segment_set_storage_key() {
        let data = vec![1, 2, 3];
        let mut segment = Segment::new(data);

        assert!(!segment.is_persisted());

        segment.set_storage_key("data/abc-123.zuk".to_string());

        assert!(segment.is_persisted());
        assert_eq!(segment.storage_key(), Some("data/abc-123.zuk"));
    }

    #[test]
    fn test_segment_from_parts() {
        let id = SegmentId::new();
        let now = Utc::now();
        let key = Some("data/test.zuk".to_string());

        let segment = Segment::from_parts(id, 100, now, key.clone());

        assert_eq!(segment.id(), &id);
        assert_eq!(segment.size(), 100);
        assert_eq!(segment.created_at(), &now);
        assert_eq!(segment.storage_key(), Some("data/test.zuk"));
        assert!(segment.is_persisted());
    }
}
