//! S3 Storage Repository Implementation
//!
//! This module implements the `StorageRepository` trait using AWS S3 as the backend.
//! It handles all S3 operations and converts AWS errors to domain errors.

use aws_sdk_s3::{primitives::ByteStream, Client};
use bytes::Bytes;
use tracing::{debug, error, info, instrument, warn};
use zuklink_domain::{
    ingestion::{entity::Segment, error::IngestionError, ids::SegmentId},
    ports::StorageRepository,
};

/// S3-based implementation of the StorageRepository port
///
/// This adapter translates domain storage operations into AWS S3 API calls.
/// It follows the "Flat Storage" pattern - all files are stored at the root
/// of the bucket with UUID-based names.
///
/// ## Configuration
///
/// The repository requires:
/// - An S3 bucket name
/// - An AWS SDK S3 Client (configured with region, credentials, endpoint)
///
/// ## Error Handling
///
/// All AWS SDK errors are converted to `IngestionError::StorageFailure` with
/// descriptive error messages for debugging.
#[derive(Clone)]
pub struct S3StorageRepository {
    client: Client,
    bucket: String,
}

impl S3StorageRepository {
    /// Create a new S3 storage repository
    ///
    /// # Arguments
    ///
    /// * `client` - Configured AWS S3 client
    /// * `bucket` - Name of the S3 bucket to use
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use aws_sdk_s3::Client;
    /// use zuklink_s3::infrastructure::S3StorageRepository;
    ///
    /// # async fn example() {
    /// let config = aws_config::load_from_env().await;
    /// let s3_client = Client::new(&config);
    /// let repo = S3StorageRepository::new(s3_client, "my-bucket".to_string());
    /// # }
    /// ```
    pub fn new(client: Client, bucket: String) -> Self {
        info!(bucket = %bucket, "Initializing S3StorageRepository");
        Self { client, bucket }
    }

    /// Get the bucket name
    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    /// Generate the S3 key for a segment
    ///
    /// Follows the flat storage pattern: just the segment UUID with .zuk extension
    fn generate_key(segment_id: &SegmentId) -> String {
        format!("{}.zuk", segment_id)
    }
}

impl StorageRepository for S3StorageRepository {
    #[instrument(skip(self, segment, data), fields(segment_id = %segment.id(), data_size = data.len()))]
    fn save(
        &self,
        segment: &Segment,
        data: &[u8],
    ) -> impl std::future::Future<Output = Result<String, IngestionError>> + Send {
        let client = self.client.clone();
        let bucket = self.bucket.clone();
        let key = Self::generate_key(segment.id());
        let data = Bytes::copy_from_slice(data);

        async move {
            debug!(key = %key, bucket = %bucket, "Saving segment to S3");

            let body = ByteStream::from(data);

            match client
                .put_object()
                .bucket(&bucket)
                .key(&key)
                .body(body)
                .send()
                .await
            {
                Ok(_) => {
                    info!(key = %key, "Successfully saved segment to S3");
                    Ok(key)
                }
                Err(err) => {
                    error!(key = %key, error = ?err, "Failed to save segment to S3");
                    Err(IngestionError::StorageFailure(format!(
                        "S3 put_object failed for key '{}': {}",
                        key, err
                    )))
                }
            }
        }
    }

    #[instrument(skip(self), fields(segment_id = %segment_id))]
    fn get(
        &self,
        segment_id: &SegmentId,
    ) -> impl std::future::Future<Output = Result<Vec<u8>, IngestionError>> + Send {
        let client = self.client.clone();
        let bucket = self.bucket.clone();
        let key = Self::generate_key(segment_id);

        async move {
            debug!(key = %key, bucket = %bucket, "Retrieving segment from S3");

            match client.get_object().bucket(&bucket).key(&key).send().await {
                Ok(output) => match output.body.collect().await {
                    Ok(data) => {
                        let bytes = data.into_bytes().to_vec();
                        info!(key = %key, size = bytes.len(), "Successfully retrieved segment from S3");
                        Ok(bytes)
                    }
                    Err(err) => {
                        error!(key = %key, error = ?err, "Failed to read S3 object body");
                        Err(IngestionError::StorageFailure(format!(
                            "Failed to read S3 object body for key '{}': {}",
                            key, err
                        )))
                    }
                },
                Err(err) => {
                    warn!(key = %key, error = ?err, "Failed to retrieve segment from S3");
                    Err(IngestionError::StorageFailure(format!(
                        "S3 get_object failed for key '{}': {}",
                        key, err
                    )))
                }
            }
        }
    }

    #[instrument(skip(self), fields(segment_id = %segment_id))]
    fn exists(
        &self,
        segment_id: &SegmentId,
    ) -> impl std::future::Future<Output = Result<bool, IngestionError>> + Send {
        let client = self.client.clone();
        let bucket = self.bucket.clone();
        let key = Self::generate_key(segment_id);

        async move {
            debug!(key = %key, bucket = %bucket, "Checking if segment exists in S3");

            match client.head_object().bucket(&bucket).key(&key).send().await {
                Ok(_) => {
                    debug!(key = %key, "Segment exists in S3");
                    Ok(true)
                }
                Err(err) => {
                    // Check if it's a "Not Found" error (404)
                    let err_str = err.to_string();
                    if err_str.contains("NotFound") || err_str.contains("404") {
                        debug!(key = %key, "Segment does not exist in S3");
                        Ok(false)
                    } else {
                        error!(key = %key, error = ?err, "Failed to check segment existence in S3");
                        Err(IngestionError::StorageFailure(format!(
                            "S3 head_object failed for key '{}': {}",
                            key, err
                        )))
                    }
                }
            }
        }
    }

    #[instrument(skip(self), fields(segment_id = %segment_id))]
    fn delete(
        &self,
        segment_id: &SegmentId,
    ) -> impl std::future::Future<Output = Result<(), IngestionError>> + Send {
        let client = self.client.clone();
        let bucket = self.bucket.clone();
        let key = Self::generate_key(segment_id);

        async move {
            debug!(key = %key, bucket = %bucket, "Deleting segment from S3");

            match client
                .delete_object()
                .bucket(&bucket)
                .key(&key)
                .send()
                .await
            {
                Ok(_) => {
                    info!(key = %key, "Successfully deleted segment from S3");
                    Ok(())
                }
                Err(err) => {
                    error!(key = %key, error = ?err, "Failed to delete segment from S3");
                    Err(IngestionError::StorageFailure(format!(
                        "S3 delete_object failed for key '{}': {}",
                        key, err
                    )))
                }
            }
        }
    }
}
