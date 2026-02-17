//! DTOs for ingestion endpoints

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request body for ingestion endpoint
#[derive(Debug, Deserialize, ToSchema)]
pub struct IngestRequest {
    /// Raw binary data to ingest (array of bytes representing "Hello")
    #[schema(example = json!([72, 101, 108, 108, 111]))]
    pub data: Vec<u8>,
}

/// Response body for successful ingestion
#[derive(Debug, Serialize, ToSchema)]
pub struct IngestResponse {
    /// Unique identifier of the ingested segment
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub segment_id: String,
    /// Success message
    #[schema(example = "Segment ingested successfully")]
    pub message: String,
}

/// Error response body
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    /// Error description
    #[schema(example = "Data cannot be empty")]
    pub error: String,
}
