//! # ZukLink Domain Layer
//!
//! This crate contains the pure business logic and domain models for the ZukLink
//! distributed streaming platform. It follows hexagonal architecture principles:
//!
//! - **Entities**: Core domain models (Segment)
//! - **Ports**: Trait definitions for external dependencies (StorageRepository)
//! - **Services**: Business logic orchestration
//!
//! ## Architecture
//!
//! This layer has NO dependencies on infrastructure concerns (AWS, S3, HTTP, etc.).
//! All external dependencies are expressed as traits (ports) that will be implemented
//! by adapter layers.
//!
//! ## Example
//!
/// ```rust
/// use zuklink_domain::ingestion::entity::Segment;
/// use zuklink_domain::ingestion::service::IngestionService;
/// use zuklink_domain::ports::StorageRepository;
/// use zuklink_domain::ingestion::error::IngestionError;
/// use std::future::Future;
///
/// // Mock implementation for doc-test
/// struct MockRepo;
/// impl StorageRepository for MockRepo {
///     fn save(&self, segment: &Segment, _data: &[u8]) -> impl Future<Output = Result<String, IngestionError>> + Send {
///         let key = format!("mock/{}", segment.id());
///         async move { Ok(key) }
///     }
///     fn get(&self, _id: &zuklink_domain::ingestion::ids::SegmentId) -> impl Future<Output = Result<Vec<u8>, IngestionError>> + Send {
///         async { Ok(vec![]) }
///     }
///     fn exists(&self, _id: &zuklink_domain::ingestion::ids::SegmentId) -> impl Future<Output = Result<bool, IngestionError>> + Send {
///         async { Ok(true) }
///     }
///     fn delete(&self, _id: &zuklink_domain::ingestion::ids::SegmentId) -> impl Future<Output = Result<(), IngestionError>> + Send {
///         async { Ok(()) }
///     }
/// }
///
/// // The service is generic over any StorageRepository implementation
/// async fn example() {
///     let repo = MockRepo;
///     let service = IngestionService::with_repository(repo);
///     let data = vec![1, 2, 3, 4];
///     let segment_id = service.ingest_data(data).await.unwrap();
///     println!("Ingested segment: {}", segment_id);
/// }
/// ```
pub mod ingestion;
pub mod ports;
pub mod storage;
