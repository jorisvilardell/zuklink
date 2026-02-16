//! Error types for the Yellowpage library

use thiserror::Error;

/// Result type alias for Yellowpage operations
pub type Result<T> = std::result::Result<T, GossipError>;

/// Errors that can occur during Yellowpage operations
#[derive(Error, Debug)]
pub enum GossipError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Node not found in cluster view
    #[error("Node {0} not found in cluster")]
    NodeNotFound(String),

    /// Cluster view is empty
    #[error("Cluster view is empty")]
    EmptyCluster,

    /// Timeout waiting for response
    #[error("Operation timed out after {duration_ms}ms")]
    Timeout { duration_ms: u64 },

    /// Generic error from underlying Chitchat library
    #[error("Chitchat error: {0}")]
    ChitchatError(String),
}

impl GossipError {
    /// Create a config error
    pub fn config_error(msg: impl Into<String>) -> Self {
        Self::ConfigError(msg.into())
    }

    /// Create a timeout error
    pub fn timeout(duration_ms: u64) -> Self {
        Self::Timeout { duration_ms }
    }

    /// Create a node not found error
    pub fn node_not_found(node_id: impl Into<String>) -> Self {
        Self::NodeNotFound(node_id.into())
    }
}
