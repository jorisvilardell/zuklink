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
//! ```rust
//! use zuklink_domain::ingestion::{Segment, IngestionService};
//! use zuklink_domain::ports::StorageRepository;
//!
//! // The service is generic over any StorageRepository implementation
//! async fn example<R: StorageRepository>(service: IngestionService<R>) {
//!     let data = vec![1, 2, 3, 4];
//!     let segment_id = service.ingest_data(data).await.unwrap();
//!     println!("Ingested segment: {}", segment_id);
//! }
//! ```

pub mod ingestion;
pub mod ports;

// Re-export commonly used types
pub use ingestion::{Segment, SegmentId, IngestionService};
pub use ports::StorageRepository;
