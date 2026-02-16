//! Ingestion domain module
//!
//! This module contains the core business logic and entities for data ingestion.
//! It defines what a Segment is and how data flows through the ingestion pipeline.

mod entity;
mod error;
mod service;

pub use entity::{Segment, SegmentId};
pub use error::{IngestionError, Result};
pub use service::{IngestionConfig, IngestionService};
