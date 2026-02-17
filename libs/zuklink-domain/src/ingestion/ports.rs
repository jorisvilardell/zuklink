//! Port trait for the Ingestion Service
//!
//! This module defines the trait that abstracts the ingestion service operations.
//! This allows for mocking the service itself at the application layer if needed,
//! while the concrete `IngestionService` in `service.rs` provides the implementation.

use std::future::Future;

use crate::ingestion::{error::IngestionError, ids::SegmentId};

/// Port trait for ingestion operations
///
/// This trait defines the contract for the ingestion service layer.
/// It can be implemented by the concrete `IngestionService<R>` or by mock
/// implementations for testing at the application/adapter layer.
pub trait IngestionServicePort: Send + Sync {
    /// Ingest raw data and return the segment ID
    ///
    /// # Arguments
    ///
    /// * `data` - The raw bytes to ingest
    ///
    /// # Returns
    ///
    /// The unique `SegmentId` of the ingested segment
    ///
    /// # Errors
    ///
    /// - `IngestionError::EmptySegment` if data is empty
    /// - `IngestionError::SegmentTooLarge` if data exceeds max size
    /// - `IngestionError::InvalidData` if data doesn't meet minimum size
    /// - `IngestionError::StorageFailure` if storage operation fails
    fn ingest_data(
        &self,
        data: Vec<u8>,
    ) -> impl Future<Output = Result<SegmentId, IngestionError>> + Send;

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
    fn get_segment_data(
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
    ///
    /// # Errors
    ///
    /// Returns `IngestionError` if the check operation fails
    fn segment_exists(
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
    fn delete_segment(
        &self,
        segment_id: &SegmentId,
    ) -> impl Future<Output = Result<(), IngestionError>> + Send;
}
