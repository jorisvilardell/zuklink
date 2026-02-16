//! Domain errors for ingestion operations
//!
//! This module defines all possible errors that can occur during ingestion.
//! These are domain-level errors that abstract away infrastructure details.

use thiserror::Error;

/// Errors that can occur during data ingestion
///
/// These errors represent business-level failures and are independent of
/// infrastructure implementation details (e.g., no AWS SDK error types here).
#[derive(Error, Debug)]
pub enum IngestionError {
    /// Failed to store the segment in the storage backend
    #[error("Storage operation failed: {0}")]
    StorageFailure(String),

    /// The provided data is invalid or corrupted
    #[error("Invalid data: {0}")]
    InvalidData(String),

    /// The segment is too large to be ingested
    #[error("Segment size ({size} bytes) exceeds maximum allowed ({max} bytes)")]
    SegmentTooLarge { size: usize, max: usize },

    /// The segment is empty (no data to ingest)
    #[error("Cannot ingest empty segment")]
    EmptySegment,

    /// A segment with this ID already exists
    #[error("Segment {0} already exists")]
    SegmentAlreadyExists(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// An unexpected internal error occurred
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl IngestionError {
    /// Create a storage failure error with a message
    pub fn storage_failure(msg: impl Into<String>) -> Self {
        Self::StorageFailure(msg.into())
    }

    /// Create an invalid data error with a message
    pub fn invalid_data(msg: impl Into<String>) -> Self {
        Self::InvalidData(msg.into())
    }

    /// Create a segment too large error
    pub fn segment_too_large(size: usize, max: usize) -> Self {
        Self::SegmentTooLarge { size, max }
    }

    /// Create a config error with a message
    pub fn config_error(msg: impl Into<String>) -> Self {
        Self::ConfigError(msg.into())
    }

    /// Create an internal error with a message
    pub fn internal_error(msg: impl Into<String>) -> Self {
        Self::InternalError(msg.into())
    }
}

/// Result type alias for ingestion operations
pub type Result<T> = std::result::Result<T, IngestionError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_failure_error() {
        let err = IngestionError::storage_failure("S3 connection failed");
        assert!(matches!(err, IngestionError::StorageFailure(_)));
        assert_eq!(
            err.to_string(),
            "Storage operation failed: S3 connection failed"
        );
    }

    #[test]
    fn test_segment_too_large_error() {
        let err = IngestionError::segment_too_large(1024, 512);
        assert!(matches!(err, IngestionError::SegmentTooLarge { .. }));
        assert!(err.to_string().contains("1024"));
        assert!(err.to_string().contains("512"));
    }

    #[test]
    fn test_empty_segment_error() {
        let err = IngestionError::EmptySegment;
        assert_eq!(err.to_string(), "Cannot ingest empty segment");
    }

    #[test]
    fn test_invalid_data_error() {
        let err = IngestionError::invalid_data("Corrupted bytes");
        assert!(err.to_string().contains("Invalid data"));
    }
}
