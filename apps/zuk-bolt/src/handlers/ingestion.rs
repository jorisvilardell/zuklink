//! Ingestion handler

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use tracing::{error, info};
use zuklink_domain::ingestion::error::IngestionError;

use crate::{
    dto::ingestion::{ErrorResponse, IngestRequest, IngestResponse},
    AppState,
};

/// Handle ingestion requests
#[utoipa::path(
    post,
    path = "/ingest",
    request_body = IngestRequest,
    responses(
        (status = 201, description = "Segment ingested successfully", body = IngestResponse),
        (status = 400, description = "Bad request - empty or invalid data", body = ErrorResponse),
        (status = 409, description = "Conflict - segment already exists", body = ErrorResponse),
        (status = 413, description = "Payload too large", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "ingestion"
)]
pub async fn ingest_handler(
    State(state): State<AppState>,
    Json(payload): Json<IngestRequest>,
) -> impl IntoResponse {
    info!(data_size = payload.data.len(), "Received ingest request");

    match state.ingestion_service.ingest_data(payload.data).await {
        Ok(segment_id) => {
            info!(segment_id = %segment_id, "Successfully ingested segment");
            (
                StatusCode::CREATED,
                Json(IngestResponse {
                    segment_id: segment_id.to_string(),
                    message: "Segment ingested successfully".to_string(),
                }),
            )
                .into_response()
        }
        Err(err) => {
            error!(error = ?err, "Failed to ingest segment");
            let (status, message) = match err {
                IngestionError::EmptySegment => {
                    (StatusCode::BAD_REQUEST, "Data cannot be empty".to_string())
                }
                IngestionError::SegmentTooLarge { size, max } => (
                    StatusCode::PAYLOAD_TOO_LARGE,
                    format!(
                        "Segment size ({} bytes) exceeds maximum ({} bytes)",
                        size, max
                    ),
                ),
                IngestionError::InvalidData(msg) => (StatusCode::BAD_REQUEST, msg),
                IngestionError::StorageFailure(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
                IngestionError::SegmentAlreadyExists(msg) => (StatusCode::CONFLICT, msg),
                IngestionError::ConfigError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
                IngestionError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            };

            (status, Json(ErrorResponse { error: message })).into_response()
        }
    }
}
