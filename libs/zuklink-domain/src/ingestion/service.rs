//! Ingestion service - Business logic orchestration
//!
//! This module contains the core business logic for data ingestion.
//! The service coordinates between the domain entities and the storage port.

use std::future::Future;

use crate::{
    ingestion::{
        entity::Segment, error::IngestionError, ids::SegmentId, ports::IngestionServicePort,
    },
    ports::StorageRepository,
};

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

// Implement the IngestionServicePort trait for IngestionService
impl<R> IngestionServicePort for IngestionService<R>
where
    R: StorageRepository,
{
    fn ingest_data(
        &self,
        data: Vec<u8>,
    ) -> impl Future<Output = Result<SegmentId, IngestionError>> + Send {
        self.ingest_data(data)
    }

    fn get_segment_data(
        &self,
        segment_id: &SegmentId,
    ) -> impl Future<Output = Result<Vec<u8>, IngestionError>> + Send {
        self.get_segment_data(segment_id)
    }

    fn segment_exists(
        &self,
        segment_id: &SegmentId,
    ) -> impl Future<Output = Result<bool, IngestionError>> + Send {
        self.segment_exists(segment_id)
    }

    fn delete_segment(
        &self,
        segment_id: &SegmentId,
    ) -> impl Future<Output = Result<(), IngestionError>> + Send {
        self.delete_segment(segment_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::future::Future;
    use std::sync::{Arc, Mutex};

    /// Mock StorageRepository using builder pattern for testing
    /// Compatible with RPITIT (Return Position Impl Trait In Trait)
    #[derive(Clone)]
    struct MockStorageRepo {
        save_fn: Arc<dyn Fn(&Segment, &[u8]) -> Result<String, IngestionError> + Send + Sync>,
        get_fn: Arc<dyn Fn(&SegmentId) -> Result<Vec<u8>, IngestionError> + Send + Sync>,
        exists_fn: Arc<dyn Fn(&SegmentId) -> Result<bool, IngestionError> + Send + Sync>,
        delete_fn: Arc<dyn Fn(&SegmentId) -> Result<(), IngestionError> + Send + Sync>,
    }

    impl MockStorageRepo {
        fn new() -> Self {
            Self {
                save_fn: Arc::new(|_, _| {
                    Err(IngestionError::storage_failure("No expectation set"))
                }),
                get_fn: Arc::new(|_| Err(IngestionError::storage_failure("No expectation set"))),
                exists_fn: Arc::new(|_| Err(IngestionError::storage_failure("No expectation set"))),
                delete_fn: Arc::new(|_| Err(IngestionError::storage_failure("No expectation set"))),
            }
        }

        fn with_save<F>(mut self, f: F) -> Self
        where
            F: Fn(&Segment, &[u8]) -> Result<String, IngestionError> + Send + Sync + 'static,
        {
            self.save_fn = Arc::new(f);
            self
        }

        fn with_get<F>(mut self, f: F) -> Self
        where
            F: Fn(&SegmentId) -> Result<Vec<u8>, IngestionError> + Send + Sync + 'static,
        {
            self.get_fn = Arc::new(f);
            self
        }

        fn with_exists<F>(mut self, f: F) -> Self
        where
            F: Fn(&SegmentId) -> Result<bool, IngestionError> + Send + Sync + 'static,
        {
            self.exists_fn = Arc::new(f);
            self
        }

        fn with_delete<F>(mut self, f: F) -> Self
        where
            F: Fn(&SegmentId) -> Result<(), IngestionError> + Send + Sync + 'static,
        {
            self.delete_fn = Arc::new(f);
            self
        }
    }

    impl StorageRepository for MockStorageRepo {
        fn save(
            &self,
            segment: &Segment,
            data: &[u8],
        ) -> impl Future<Output = Result<String, IngestionError>> + Send {
            let result = (self.save_fn)(segment, data);
            async move { result }
        }

        fn get(
            &self,
            segment_id: &SegmentId,
        ) -> impl Future<Output = Result<Vec<u8>, IngestionError>> + Send {
            let result = (self.get_fn)(segment_id);
            async move { result }
        }

        fn exists(
            &self,
            segment_id: &SegmentId,
        ) -> impl Future<Output = Result<bool, IngestionError>> + Send {
            let result = (self.exists_fn)(segment_id);
            async move { result }
        }

        fn delete(
            &self,
            segment_id: &SegmentId,
        ) -> impl Future<Output = Result<(), IngestionError>> + Send {
            let result = (self.delete_fn)(segment_id);
            async move { result }
        }
    }

    /// Test Builder Pattern for IngestionService tests
    /// Inspired by Ferriskey's test architecture
    struct IngestionServiceTestBuilder {
        storage: MockStorageRepo,
        config: Option<IngestionConfig>,
    }

    impl IngestionServiceTestBuilder {
        fn new() -> Self {
            Self {
                storage: MockStorageRepo::new(),
                config: None,
            }
        }

        fn with_successful_save(mut self) -> Self {
            self.storage = self
                .storage
                .with_save(|seg, _| Ok(format!("data/{}.zuk", seg.id())));
            self
        }

        fn with_failed_save(mut self, error_msg: &'static str) -> Self {
            self.storage = self
                .storage
                .with_save(move |_, _| Err(IngestionError::storage_failure(error_msg)));
            self
        }

        fn with_successful_get(mut self, expected_data: Vec<u8>) -> Self {
            self.storage = self.storage.with_get(move |_| Ok(expected_data.clone()));
            self
        }

        fn with_failed_get(mut self, error_msg: &'static str) -> Self {
            self.storage = self
                .storage
                .with_get(move |_| Err(IngestionError::storage_failure(error_msg)));
            self
        }

        fn with_exists(mut self, exists: bool) -> Self {
            self.storage = self.storage.with_exists(move |_| Ok(exists));
            self
        }

        fn with_successful_delete(mut self) -> Self {
            self.storage = self.storage.with_delete(|_| Ok(()));
            self
        }

        fn with_failed_delete(mut self, error_msg: &'static str) -> Self {
            self.storage = self
                .storage
                .with_delete(move |_| Err(IngestionError::storage_failure(error_msg)));
            self
        }

        fn with_in_memory_storage(mut self) -> Self {
            let data: Arc<Mutex<HashMap<String, Vec<u8>>>> = Arc::new(Mutex::new(HashMap::new()));

            let data_clone = data.clone();
            self.storage = self.storage.with_save(move |seg, bytes| {
                let key = format!("data/{}.zuk", seg.id());
                data_clone
                    .lock()
                    .unwrap()
                    .insert(key.clone(), bytes.to_vec());
                Ok(key)
            });

            let data_clone = data.clone();
            self.storage = self.storage.with_get(move |seg_id| {
                let key = format!("data/{}.zuk", seg_id);
                data_clone
                    .lock()
                    .unwrap()
                    .get(&key)
                    .cloned()
                    .ok_or_else(|| IngestionError::storage_failure("Segment not found"))
            });

            let data_clone = data.clone();
            self.storage = self.storage.with_exists(move |seg_id| {
                let key = format!("data/{}.zuk", seg_id);
                Ok(data_clone.lock().unwrap().contains_key(&key))
            });

            let data_clone = data.clone();
            self.storage = self.storage.with_delete(move |seg_id| {
                let key = format!("data/{}.zuk", seg_id);
                data_clone.lock().unwrap().remove(&key);
                Ok(())
            });

            self
        }

        fn with_config(mut self, config: IngestionConfig) -> Self {
            self.config = Some(config);
            self
        }

        fn with_max_segment_size(mut self, max_size: usize) -> Self {
            let mut config = self.config.take().unwrap_or_default();
            config.max_segment_size = max_size;
            self.config = Some(config);
            self
        }

        fn with_min_segment_size(mut self, min_size: usize) -> Self {
            let mut config = self.config.take().unwrap_or_default();
            config.min_segment_size = min_size;
            self.config = Some(config);
            self
        }

        fn build(self) -> IngestionService<MockStorageRepo> {
            if let Some(config) = self.config {
                IngestionService::new(self.storage, config)
            } else {
                IngestionService::with_repository(self.storage)
            }
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_ingest_data_success() {
        let service = IngestionServiceTestBuilder::new()
            .with_successful_save()
            .build();

        let data = vec![1, 2, 3, 4, 5];
        let result = service.ingest_data(data).await;

        assert!(result.is_ok());
        let segment_id = result.unwrap();
        assert_ne!(segment_id.to_string(), "");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_ingest_empty_data_fails() {
        let service = IngestionServiceTestBuilder::new()
            .with_successful_save()
            .build();

        let data = vec![];
        let result = service.ingest_data(data).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), IngestionError::EmptySegment));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_ingest_too_large_data_fails() {
        let service = IngestionServiceTestBuilder::new()
            .with_successful_save()
            .with_max_segment_size(10)
            .build();

        let data = vec![1; 100]; // 100 bytes, exceeds max of 10
        let result = service.ingest_data(data).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            IngestionError::SegmentTooLarge { .. }
        ));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_ingest_below_min_size_fails() {
        let service = IngestionServiceTestBuilder::new()
            .with_successful_save()
            .with_min_segment_size(10)
            .build();

        let data = vec![1, 2, 3]; // 3 bytes, below min of 10
        let result = service.ingest_data(data).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            IngestionError::InvalidData(_)
        ));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_ingest_with_storage_failure() {
        let service = IngestionServiceTestBuilder::new()
            .with_failed_save("S3 unavailable")
            .build();

        let data = vec![1, 2, 3, 4, 5];
        let result = service.ingest_data(data).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            IngestionError::StorageFailure(_)
        ));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_segment_data_success() {
        let expected_data = vec![1, 2, 3, 4, 5];
        let service = IngestionServiceTestBuilder::new()
            .with_successful_get(expected_data.clone())
            .build();

        let segment_id = SegmentId::new();
        let result = service.get_segment_data(&segment_id).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_data);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_segment_data_not_found() {
        let service = IngestionServiceTestBuilder::new()
            .with_failed_get("Segment not found")
            .build();

        let segment_id = SegmentId::new();
        let result = service.get_segment_data(&segment_id).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            IngestionError::StorageFailure(_)
        ));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_segment_exists_true() {
        let service = IngestionServiceTestBuilder::new().with_exists(true).build();

        let segment_id = SegmentId::new();
        let result = service.segment_exists(&segment_id).await;

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_segment_exists_false() {
        let service = IngestionServiceTestBuilder::new()
            .with_exists(false)
            .build();

        let segment_id = SegmentId::new();
        let result = service.segment_exists(&segment_id).await;

        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_delete_segment_success() {
        let service = IngestionServiceTestBuilder::new()
            .with_successful_delete()
            .build();

        let segment_id = SegmentId::new();
        let result = service.delete_segment(&segment_id).await;

        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_delete_segment_failure() {
        let service = IngestionServiceTestBuilder::new()
            .with_failed_delete("Permission denied")
            .build();

        let segment_id = SegmentId::new();
        let result = service.delete_segment(&segment_id).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            IngestionError::StorageFailure(_)
        ));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_full_lifecycle_with_in_memory_storage() {
        let service = IngestionServiceTestBuilder::new()
            .with_in_memory_storage()
            .build();

        // Ingest data
        let data = vec![1, 2, 3, 4, 5];
        let segment_id = service.ingest_data(data.clone()).await.unwrap();

        // Verify it exists
        let exists = service.segment_exists(&segment_id).await.unwrap();
        assert!(exists);

        // Retrieve the data
        let retrieved = service.get_segment_data(&segment_id).await.unwrap();
        assert_eq!(retrieved, data);

        // Delete it
        service.delete_segment(&segment_id).await.unwrap();

        // Verify it's gone
        let exists = service.segment_exists(&segment_id).await.unwrap();
        assert!(!exists);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_multiple_segments_isolation() {
        let service = IngestionServiceTestBuilder::new()
            .with_in_memory_storage()
            .build();

        let data1 = vec![1, 2, 3];
        let data2 = vec![4, 5, 6];
        let data3 = vec![7, 8, 9];

        // Ingest multiple segments
        let segment1_id = service.ingest_data(data1.clone()).await.unwrap();
        let segment2_id = service.ingest_data(data2.clone()).await.unwrap();
        let segment3_id = service.ingest_data(data3.clone()).await.unwrap();

        // Verify all IDs are unique
        assert_ne!(segment1_id, segment2_id);
        assert_ne!(segment2_id, segment3_id);
        assert_ne!(segment1_id, segment3_id);

        // Verify all data is correctly isolated
        assert_eq!(service.get_segment_data(&segment1_id).await.unwrap(), data1);
        assert_eq!(service.get_segment_data(&segment2_id).await.unwrap(), data2);
        assert_eq!(service.get_segment_data(&segment3_id).await.unwrap(), data3);

        // Delete one segment
        service.delete_segment(&segment2_id).await.unwrap();

        // Verify only the deleted segment is gone
        assert!(service.segment_exists(&segment1_id).await.unwrap());
        assert!(!service.segment_exists(&segment2_id).await.unwrap());
        assert!(service.segment_exists(&segment3_id).await.unwrap());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_config_access() {
        let config = IngestionConfig {
            max_segment_size: 1024,
            min_segment_size: 10,
        };

        let service = IngestionServiceTestBuilder::new()
            .with_successful_save()
            .with_config(config.clone())
            .build();

        assert_eq!(service.config().max_segment_size, 1024);
        assert_eq!(service.config().min_segment_size, 10);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_edge_case_min_size_boundary() {
        let service = IngestionServiceTestBuilder::new()
            .with_successful_save()
            .with_min_segment_size(5)
            .build();

        // Exactly at minimum should succeed
        let data_at_min = vec![1; 5];
        let result = service.ingest_data(data_at_min).await;
        assert!(result.is_ok());

        // One byte below minimum should fail
        let data_below_min = vec![1; 4];
        let result = service.ingest_data(data_below_min).await;
        assert!(result.is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_edge_case_max_size_boundary() {
        let service = IngestionServiceTestBuilder::new()
            .with_successful_save()
            .with_max_segment_size(10)
            .build();

        // Exactly at maximum should succeed
        let data_at_max = vec![1; 10];
        let result = service.ingest_data(data_at_max).await;
        assert!(result.is_ok());

        // One byte above maximum should fail
        let data_above_max = vec![1; 11];
        let result = service.ingest_data(data_above_max).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            IngestionError::SegmentTooLarge { .. }
        ));
    }
}
