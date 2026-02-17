//! Ports (trait definitions) for external dependencies
//!
//! This module defines the contracts (ports) that external adapters must implement.
//! Following hexagonal architecture, the domain defines what it needs, and the
//! infrastructure provides implementations.
//!
//! ## Static Dispatch
//!
//! We use native Rust async traits with `impl Future` return types instead of
//! `async_trait` to ensure zero-cost abstractions and static dispatch.

use std::future::Future;

use crate::ingestion::{entity::Segment, error::IngestionError, ids::SegmentId};

/// Port for storage operations
///
/// This trait abstracts away the storage backend (S3, filesystem, etc.).
/// Implementations must handle:
/// - Storing raw bytes with generated keys
/// - Returning storage location identifiers
/// - Converting infrastructure errors to domain errors
///
/// ## Implementation Note
///
/// This trait uses `impl Future` return types for true static dispatch.
/// The compiler will monomorphize each implementation, resulting in
/// zero-cost abstractions without trait objects.
///
pub trait StorageRepository: Send + Sync {
    /// Save a segment's data to storage
    ///
    /// This method persists the raw bytes of a segment and returns the storage key.
    /// The implementation should:
    /// 1. Generate or use the segment's ID to create a storage key
    /// 2. Store the bytes in the backend (S3, filesystem, etc.)
    /// 3. Return the full storage key/path
    /// 4. Convert any infrastructure errors to `IngestionError::StorageFailure`
    ///
    /// # Arguments
    ///
    /// * `segment` - The segment metadata
    /// * `data` - The raw bytes to store
    ///
    /// # Returns
    ///
    /// The storage key where the data was stored (e.g., "data/abc-123.zuk")
    ///
    /// # Errors
    ///
    /// Returns `IngestionError::StorageFailure` if the storage operation fails
    fn save(
        &self,
        segment: &Segment,
        data: &[u8],
    ) -> impl Future<Output = Result<String, IngestionError>> + Send;

    /// Retrieve a segment's data from storage
    ///
    /// # Arguments
    ///
    /// * `segment_id` - The unique identifier of the segment to retrieve
    ///
    /// # Returns
    ///
    /// The raw bytes of the segment
    ///
    /// # Errors
    ///
    /// Returns `IngestionError::StorageFailure` if the segment doesn't exist or retrieval fails
    fn get(
        &self,
        segment_id: &SegmentId,
    ) -> impl Future<Output = Result<Vec<u8>, IngestionError>> + Send;

    /// Check if a segment exists in storage
    ///
    /// # Arguments
    ///
    /// * `segment_id` - The unique identifier of the segment to check
    ///
    /// # Returns
    ///
    /// `true` if the segment exists, `false` otherwise
    fn exists(
        &self,
        segment_id: &SegmentId,
    ) -> impl Future<Output = Result<bool, IngestionError>> + Send;

    /// Delete a segment from storage
    ///
    /// # Arguments
    ///
    /// * `segment_id` - The unique identifier of the segment to delete
    ///
    /// # Errors
    ///
    /// Returns `IngestionError::StorageFailure` if deletion fails
    fn delete(
        &self,
        segment_id: &SegmentId,
    ) -> impl Future<Output = Result<(), IngestionError>> + Send;
}
