//! API routes

pub mod ingestion;

use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    dto::ingestion::{ErrorResponse, IngestRequest, IngestResponse},
    handlers, AppState,
};

/// OpenAPI documentation structure
#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::ingestion::ingest_handler,
        health_handler
    ),
    components(
        schemas(IngestRequest, IngestResponse, ErrorResponse)
    ),
    tags(
        (name = "ingestion", description = "Data ingestion endpoints"),
        (name = "health", description = "Health check endpoints")
    ),
    info(
        title = "ZukBolt API",
        version = "0.1.0",
        description = "Stateless sender service for ZukLink distributed streaming platform",
        contact(
            name = "ZukLink Team"
        )
    )
)]
pub struct ApiDoc;

/// Create the main application router
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(ingestion::routes())
        .route("/health", axum::routing::get(health_handler))
        .with_state(state)
}

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = String)
    ),
    tag = "health"
)]
async fn health_handler() -> &'static str {
    "OK"
}
