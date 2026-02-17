//! Ingestion routes

use axum::{routing::post, Router};

use crate::{handlers::ingestion::ingest_handler, AppState};

/// Create ingestion routes
pub fn routes() -> Router<AppState> {
    Router::new().route("/ingest", post(ingest_handler))
}
