//! Ingestion service - Business logic orchestration
//!
//! This module contains the core business logic for data ingestion.
//! The service coordinates between the domain entities and the storage port.

use super::{IngestionError, Segment, SegmentId};
use crate::ports::StorageRepository;

/// Configuration for the ingestion service
#[derive(Debug, Clone)]
pub struct IngestionConfig {
    /// Maximum allowed segment size in bytes (default: 100MB)
    pub max_segment_size: usize,
    /// Minimum segment size in bytes (default: 1 byte)
    pub min_segment_size: usize,
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            max_segment_size: 100 * 1024 * 1024, // 100MB
            min_segment_size: 1,
        }
    }
}

/// Service for ingesting data into the ZukLink platform
///
/// This service encapsulates the business rules for data ingestion:
/// - Validates data size constraints
/// - Creates domain entities (Segments)
/// - Coordinates with storage backend via the repository port
/// - Ensures data integrity and business invariants
///
/// ## Static Dispatch
///
/// The service is generic over any `StorageRepository` implementation.
/// The compiler will generate specialized versions for each concrete type,
/// resulting in zero-cost abstractions.
///
pub struct IngestionService<R> {
    repository: R,
    config: IngestionConfig,
}

impl<R> IngestionService<R>
where
    R: StorageRepository,
{
    /// Create a new IngestionService with the given repository and configuration
    pub fn new(repository: R, config: IngestionConfig) -> Self {
        Self { repository, config }
    }

    /// Create a new IngestionService with default configuration
    pub fn with_repository(repository: R) -> Self {
        Self::new(repository, IngestionConfig::default())
    }

    /// Ingest raw data and return the segment ID
    ///
    /// This is the main entry point for data ingestion. It:
    /// 1. Validates the data according to business rules
    /// 2. Creates a Segment entity
    /// 3. Persists the data via the storage repository
    /// 4. Returns the segment ID for tracking
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
    /// - `IngestionError::StorageFailure` if storage operation fails
    ///
    pub async fn ingest_data(&self, data: Vec<u8>) -> Result<SegmentId, IngestionError> {
        // Business rule: Cannot ingest empty data
        if data.is_empty() {
            return Err(IngestionError::EmptySegment);
        }

        // Business rule: Enforce maximum segment size
        if data.len() > self.config.max_segment_size {
            return Err(IngestionError::segment_too_large(
                data.len(),
                self.config.max_segment_size,
            ));
        }

        // Business rule: Enforce minimum segment size
        if data.len() < self.config.min_segment_size {
            return Err(IngestionError::invalid_data(format!(
                "Segment size ({}) is below minimum ({})",
                data.len(),
                self.config.min_segment_size
            )));
        }

        // Create domain entity
        let mut segment = Segment::new(data.clone());

        // Persist via repository (infrastructure concern)
        let storage_key = self.repository.save(&segment, &data).await?;

        // Update segment with storage location
        segment.set_storage_key(storage_key);

        // Return segment ID for tracking
        Ok(*segment.id())
    }

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
    pub async fn get_segment_data(
        &self,
        segment_id: &SegmentId,
    ) -> Result<Vec<u8>, IngestionError> {
        self.repository.get(segment_id).await
    }

    /// Check if a segment exists
    ///
    /// # Arguments
    ///
    /// * `segment_id` - The unique identifier of the segment to check
    ///
    /// # Returns
    ///
    /// `true` if the segment exists, `false` otherwise
    pub async fn segment_exists(&self, segment_id: &SegmentId) -> Result<bool, IngestionError> {
        self.repository.exists(segment_id).await
    }

    /// Delete a segment from storage
    ///
    /// # Arguments
    ///
    /// * `segment_id` - The unique identifier of the segment to delete
    ///
    /// # Errors
    ///
    /// Returns `IngestionError::StorageFailure` if deletion fails
    pub async fn delete_segment(&self, segment_id: &SegmentId) -> Result<(), IngestionError> {
        self.repository.delete(segment_id).await
    }

    /// Get the service configuration
    pub fn config(&self) -> &IngestionConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    // In-memory storage for testing
    struct InMemoryStorage {
        data: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    }

    impl InMemoryStorage {
        fn new() -> Self {
            Self {
                data: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    impl StorageRepository for InMemoryStorage {
        fn save(
            &self,
            segment: &Segment,
            data: &[u8],
        ) -> impl std::future::Future<Output = Result<String, IngestionError>> + Send {
            let key = format!("data/{}.zuk", segment.id());
            let data_clone = data.to_vec();
            let data_map = self.data.clone();

            async move {
                data_map.lock().unwrap().insert(key.clone(), data_clone);
                Ok(key)
            }
        }

        fn get(
            &self,
            segment_id: &SegmentId,
        ) -> impl std::future::Future<Output = Result<Vec<u8>, IngestionError>> + Send {
            let key = format!("data/{}.zuk", segment_id);
            let data_map = self.data.clone();

            async move {
                data_map
                    .lock()
                    .unwrap()
                    .get(&key)
                    .cloned()
                    .ok_or_else(|| IngestionError::storage_failure("Segment not found"))
            }
        }

        fn exists(
            &self,
            segment_id: &SegmentId,
        ) -> impl std::future::Future<Output = Result<bool, IngestionError>> + Send {
            let key = format!("data/{}.zuk", segment_id);
            let data_map = self.data.clone();

            async move { Ok(data_map.lock().unwrap().contains_key(&key)) }
        }

        fn delete(
            &self,
            segment_id: &SegmentId,
        ) -> impl std::future::Future<Output = Result<(), IngestionError>> + Send {
            let key = format!("data/{}.zuk", segment_id);
            let data_map = self.data.clone();

            async move {
                data_map.lock().unwrap().remove(&key);
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn test_ingest_data_success() {
        let storage = InMemoryStorage::new();
        let service = IngestionService::with_repository(storage);

        let data = vec![1, 2, 3, 4, 5];
        let result = service.ingest_data(data).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_ingest_empty_data_fails() {
        let storage = InMemoryStorage::new();
        let service = IngestionService::with_repository(storage);

        let data = vec![];
        let result = service.ingest_data(data).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), IngestionError::EmptySegment));
    }

    #[tokio::test]
    async fn test_ingest_too_large_data_fails() {
        let storage = InMemoryStorage::new();
        let config = IngestionConfig {
            max_segment_size: 10,
            min_segment_size: 1,
        };
        let service = IngestionService::new(storage, config);

        let data = vec![1; 100]; // 100 bytes, exceeds max of 10
        let result = service.ingest_data(data).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            IngestionError::SegmentTooLarge { .. }
        ));
    }

    #[tokio::test]
    async fn test_get_segment_data() {
        let storage = InMemoryStorage::new();
        let service = IngestionService::with_repository(storage);

        let data = vec![1, 2, 3, 4, 5];
        let segment_id = service.ingest_data(data.clone()).await.unwrap();

        let retrieved = service.get_segment_data(&segment_id).await.unwrap();
        assert_eq!(retrieved, data);
    }

    #[tokio::test]
    async fn test_segment_exists() {
        let storage = InMemoryStorage::new();
        let service = IngestionService::with_repository(storage);

        let data = vec![1, 2, 3];
        let segment_id = service.ingest_data(data).await.unwrap();

        let exists = service.segment_exists(&segment_id).await.unwrap();
        assert!(exists);

        let non_existent_id = SegmentId::new();
        let exists = service.segment_exists(&non_existent_id).await.unwrap();
        assert!(!exists);
    }

    #[tokio::test]
    async fn test_delete_segment() {
        let storage = InMemoryStorage::new();
        let service = IngestionService::with_repository(storage);

        let data = vec![1, 2, 3];
        let segment_id = service.ingest_data(data).await.unwrap();

        // Verify it exists
        assert!(service.segment_exists(&segment_id).await.unwrap());

        // Delete it
        service.delete_segment(&segment_id).await.unwrap();

        // Verify it's gone
        assert!(!service.segment_exists(&segment_id).await.unwrap());
    }

    #[tokio::test]
    async fn test_multiple_segments() {
        let storage = InMemoryStorage::new();
        let service = IngestionService::with_repository(storage);

        let segment1_id = service.ingest_data(vec![1, 2, 3]).await.unwrap();
        let segment2_id = service.ingest_data(vec![4, 5, 6]).await.unwrap();

        assert_ne!(segment1_id, segment2_id);

        let data1 = service.get_segment_data(&segment1_id).await.unwrap();
        let data2 = service.get_segment_data(&segment2_id).await.unwrap();

        assert_eq!(data1, vec![1, 2, 3]);
        assert_eq!(data2, vec![4, 5, 6]);
    }
}
