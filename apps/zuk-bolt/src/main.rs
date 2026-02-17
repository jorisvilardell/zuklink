//! ZukBolt - Stateless Sender Service
//!
//! HTTP service for ingesting data into ZukLink distributed streaming platform.
//! Follows the "Flat Storage" pattern: writes to S3 without coordination.

mod dto;
mod handlers;
mod routes;

use anyhow::Result;
use std::sync::Arc;
use tracing::info;
use zuklink_domain::ingestion::service::IngestionService;
use zuklink_s3::infrastructure::S3StorageRepository;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub ingestion_service: Arc<IngestionService<S3StorageRepository>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting ZukBolt sender service");

    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize AWS S3 client with MinIO-compatible configuration
    let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;

    // Configure S3 client with path-style addressing for MinIO compatibility
    let s3_config = aws_sdk_s3::config::Builder::from(&aws_config)
        .force_path_style(true) // Required for MinIO
        .build();

    let s3_client = aws_sdk_s3::Client::from_conf(s3_config);

    // Get bucket name from environment
    let bucket = std::env::var("ZUKLINK_BUCKET").unwrap_or_else(|_| {
        info!("ZUKLINK_BUCKET not set, using default: zuklink");
        "zuklink".to_string()
    });

    info!(bucket = %bucket, "Initializing S3 storage repository");

    // Create S3 repository
    let repository = S3StorageRepository::new(s3_client, bucket);

    // Create ingestion service
    let service = IngestionService::with_repository(repository);

    // Create shared application state
    let state = AppState {
        ingestion_service: Arc::new(service),
    };

    // Build HTTP router
    let app = routes::create_router(state);

    // Get bind address from environment
    let host = std::env::var("BOLT_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("BOLT_PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("{}:{}", host, port);

    info!(addr = %addr, "Starting HTTP server");

    // Start server
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
