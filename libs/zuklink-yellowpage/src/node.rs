//! Node identification and state management

use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for a node in the cluster
///
/// NodeId is used to identify individual nodes in the distributed system.
/// It must be unique across the cluster and is used for consistent hashing.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    /// Create a new NodeId
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for NodeId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for NodeId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_ordering() {
        let mut ids = vec![
            NodeId::new("node-3"),
            NodeId::new("node-1"),
            NodeId::new("node-2"),
        ];

        ids.sort();

        assert_eq!(ids[0].as_str(), "node-1");
        assert_eq!(ids[1].as_str(), "node-2");
        assert_eq!(ids[2].as_str(), "node-3");
    }

    #[test]
    fn test_node_id_equality() {
        let id1 = NodeId::new("node-1");
        let id2 = NodeId::new("node-1");
        let id3 = NodeId::new("node-2");

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_node_id_display() {
        let id = NodeId::new("receiver-1");
        assert_eq!(format!("{}", id), "receiver-1");
    }
}
