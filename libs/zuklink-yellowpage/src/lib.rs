mod error;
mod node;

pub use error::{GossipError, Result};
pub use node::NodeId;

use chitchat::transport::UdpTransport;
use chitchat::{spawn_chitchat, ChitchatConfig, ChitchatHandle, ChitchatId, FailureDetectorConfig};
use std::net::SocketAddr;
use std::time::Duration;
use tracing::info;

/// Main entry point for cluster coordination
///
/// Wraps Chitchat to provide a simplified API for ZukLink's needs:
/// - Discovery of live nodes
/// - Consistent ordering for sharding
/// - Metadata storage (role, load, etc.)
pub struct Yellowpage {
    /// Handle to the Chitchat instance
    handle: ChitchatHandle,
    /// This node's unique identifier
    node_id: NodeId,
    /// Cluster identifier
    cluster_id: String,
}

impl Yellowpage {
    /// Create a new Yellowpage instance
    ///
    /// # Arguments
    ///
    /// * `node_id` - Unique identifier for this node (e.g., "receiver-1")
    /// * `listen_addr` - Socket address to bind to for gossip protocol
    /// * `seeds` - List of seed nodes to bootstrap the cluster (e.g., ["receiver-1:7000"])
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Cannot bind to the listen address
    /// - Invalid seed node addresses
    /// - Chitchat initialization fails
    pub async fn new(node_id: String, listen_addr: SocketAddr, seeds: Vec<String>) -> Result<Self> {
        Self::with_cluster_id(node_id, "zuklink-cluster".to_string(), listen_addr, seeds).await
    }

    /// Create a new Yellowpage instance with a custom cluster ID
    ///
    /// Useful for testing or running multiple independent clusters.
    pub async fn with_cluster_id(
        node_id: String,
        cluster_id: String,
        listen_addr: SocketAddr,
        seeds: Vec<String>,
    ) -> Result<Self> {
        info!(
            node_id = %node_id,
            cluster_id = %cluster_id,
            listen_addr = %listen_addr,
            seeds = ?seeds,
            "Initializing Yellowpage"
        );

        // Create ChitchatId with node_id, generation (using current timestamp), and advertise addr
        let generation_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let chitchat_id = ChitchatId::new(node_id.clone(), generation_id, listen_addr);

        let config = ChitchatConfig {
            chitchat_id,
            cluster_id: cluster_id.clone(),
            gossip_interval: Duration::from_millis(500),
            listen_addr,
            seed_nodes: seeds.clone(),
            failure_detector_config: FailureDetectorConfig::default(),
            marked_for_deletion_grace_period: Duration::from_secs(60),
            catchup_callback: None,
            extra_liveness_predicate: None,
        };

        // Create UDP transport
        let transport = UdpTransport;

        // Spawn Chitchat in background
        let handle = spawn_chitchat(config, Vec::new(), &transport)
            .await
            .map_err(|e| GossipError::config_error(format!("Failed to spawn chitchat: {}", e)))?;

        info!(
            node_id = %node_id,
            "Yellowpage initialized successfully"
        );

        Ok(Self {
            handle,
            node_id: NodeId(node_id),
            cluster_id,
        })
    }

    /// Get the sorted list of live node IDs
    ///
    /// This is the core method used by receivers for consistent hashing.
    /// The list is always sorted to ensure all nodes agree on the same ordering.
    ///
    /// # Returns
    ///
    /// A vector of node IDs, sorted lexicographically.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use zuklink_yellowpage::Yellowpage;
    /// # async fn example(yellowpage: &Yellowpage) {
    /// let live_nodes = yellowpage.get_live_nodes().await;
    /// let my_index = live_nodes.iter().position(|id| id == yellowpage.node_id());
    /// let cluster_size = live_nodes.len();
    /// // Use my_index and cluster_size for consistent hashing
    /// # }
    /// ```
    pub async fn get_live_nodes(&self) -> Vec<NodeId> {
        let chitchat = self.handle.chitchat();
        let chitchat_guard = chitchat.lock().await;

        let mut nodes: Vec<NodeId> = chitchat_guard
            .live_nodes()
            .map(|chitchat_id| NodeId(chitchat_id.node_id.clone()))
            .collect();

        // CRITICAL: Sort to ensure consistent ordering across all nodes
        nodes.sort();

        nodes
    }

    /// Get the number of live nodes in the cluster
    pub async fn cluster_size(&self) -> usize {
        let chitchat = self.handle.chitchat();
        let chitchat_guard = chitchat.lock().await;
        chitchat_guard.live_nodes().count()
    }

    /// Get this node's position in the sorted cluster view
    ///
    /// Returns `None` if this node is not in the live nodes list
    /// (which shouldn't happen under normal circumstances).
    pub async fn my_index(&self) -> Option<usize> {
        let live_nodes = self.get_live_nodes().await;
        live_nodes.iter().position(|id| id == &self.node_id)
    }

    /// Set a metadata key-value pair for this node
    ///
    /// Metadata is propagated to all nodes in the cluster via gossip.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use zuklink_yellowpage::Yellowpage;
    /// # async fn example(yellowpage: &Yellowpage) {
    /// // Mark this node's role
    /// yellowpage.set_metadata("role", "receiver").await;
    ///
    /// // Report current load (for future load balancing)
    /// yellowpage.set_metadata("cpu_load", "0.75").await;
    /// # }
    /// ```
    pub async fn set_metadata(&self, key: &str, value: &str) {
        let chitchat = self.handle.chitchat();
        let mut chitchat_guard = chitchat.lock().await;

        chitchat_guard
            .self_node_state()
            .set(key.to_string(), value.to_string());

        info!(
            node_id = %self.node_id,
            key = key,
            value = value,
            "Metadata set"
        );
    }

    /// Get metadata for a specific node
    ///
    /// Returns `None` if the node doesn't exist or the key is not set.
    pub async fn get_metadata(&self, node_id: &NodeId, key: &str) -> Option<String> {
        let chitchat = self.handle.chitchat();
        let chitchat_guard = chitchat.lock().await;

        // Find the ChitchatId for this node_id
        let chitchat_id = chitchat_guard
            .live_nodes()
            .find(|chitchat_id| chitchat_id.node_id == node_id.0)
            .cloned();

        if let Some(id) = chitchat_id {
            chitchat_guard
                .node_state(&id)
                .and_then(|state| state.get(key).map(|v| v.to_string()))
        } else {
            None
        }
    }

    /// Get this node's ID
    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    /// Get the cluster ID
    pub fn cluster_id(&self) -> &str {
        &self.cluster_id
    }

    /// Gracefully shutdown the Yellowpage instance
    pub async fn shutdown(self) {
        info!(node_id = %self.node_id, "Shutting down Yellowpage");
        let _ = self.handle.shutdown().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_node_id_sorting() {
        let mut nodes = vec![
            NodeId("node-3".to_string()),
            NodeId("node-1".to_string()),
            NodeId("node-2".to_string()),
        ];

        nodes.sort();

        assert_eq!(nodes[0].0, "node-1");
        assert_eq!(nodes[1].0, "node-2");
        assert_eq!(nodes[2].0, "node-3");
    }
}
