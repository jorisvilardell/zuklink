# ZukLink Yellowpage

Cluster coordination library for ZukLink distributed streaming platform.

## Overview

ZukLink Yellowpage provides distributed coordination for the ZukLink streaming platform using [Chitchat](https://github.com/quickwit-oss/chitchat), a robust Scuttlebutt-based gossip protocol from Quickwit.

### Key Features

- **Peer Discovery**: Automatic discovery of cluster members via gossip protocol
- **Failure Detection**: Phi Accrual failure detector for reliable node health monitoring
- **Consistent Ordering**: Sorted cluster view for deterministic consistent hashing
- **Metadata Store**: Distributed key-value store for node metadata propagation
- **No Central Coordinator**: Fully decentralized, eventually consistent architecture

## Why Chitchat?

Instead of implementing a naive UDP gossip protocol, we leverage Chitchat for:

1. **Battle-Tested Algorithm**: Scuttlebutt protocol with state reconciliation
2. **Production-Ready**: Used by Quickwit in production distributed search
3. **Phi Accrual Failure Detection**: Probabilistic, adaptive failure detection
4. **Built-in KV Store**: Share metadata (load, status, etc.) across the cluster

## Usage

### Basic Example

```rust
use zuklink_yellowpage::Yellowpage;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listen_addr: SocketAddr = "0.0.0.0:7000".parse()?;
    let seeds = vec!["receiver-1:7000".to_string()];

    let yellowpage = Yellowpage::new(
        "my-node-id".to_string(),
        listen_addr,
        seeds,
    ).await?;

    // Mark this node's role
    yellowpage.set_metadata("role", "receiver").await;

    // Get sorted list of live nodes
    let live_nodes = yellowpage.get_live_nodes().await;
    println!("Live nodes: {:?}", live_nodes);

    Ok(())
}
```

### Consistent Hashing

The primary use case is enabling receivers to deterministically shard work:

```rust
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

async fn should_process_file(
    yellowpage: &Yellowpage,
    filename: &str,
) -> bool {
    let live_nodes = yellowpage.get_live_nodes().await;
    let cluster_size = live_nodes.len();
    
    let my_index = yellowpage.my_index().await.unwrap();
    
    let mut hasher = DefaultHasher::new();
    filename.hash(&mut hasher);
    let hash = hasher.finish();
    
    (hash as usize % cluster_size) == my_index
}
```

### Metadata Management

Share arbitrary metadata across the cluster:

```rust
// Set metadata on this node
yellowpage.set_metadata("cpu_load", "0.75").await;
yellowpage.set_metadata("status", "ready").await;

// Read metadata from another node
let node_id = NodeId::new("receiver-1");
if let Some(load) = yellowpage.get_metadata(&node_id, "cpu_load").await {
    println!("Node {} load: {}", node_id, load);
}
```

## Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Receiver 1 │◄───►│  Receiver 2 │◄───►│  Receiver 3 │
│ (Yellowpage)│     │ (Yellowpage)│     │ (Yellowpage)│
└─────────────┘     └─────────────┘     └─────────────┘
       │                   │                   │
       └───────────────────┴───────────────────┘
              Gossip Protocol (UDP)
              Scuttlebutt State Sync
```

Each node:
1. Joins the cluster via seed nodes
2. Exchanges gossip messages with peers
3. Maintains an eventually consistent view
4. Detects failures via Phi Accrual
5. Propagates metadata via KV store

## Configuration

Yellowpage is configured at initialization:

```rust
let config = YellowpageConfig {
    node_id: "receiver-1".to_string(),
    listen_addr: "0.0.0.0:7000".parse()?,
    seeds: vec!["receiver-2:7000".to_string()],
};
```

### Environment Variables (for apps)

- `ZUK_NODE_ID`: Unique node identifier
- `ZUK_GOSSIP_PORT`: Port for gossip protocol (default: 7000)
- `ZUK_SEEDS`: Comma-separated list of seed nodes

## Running the Example

A complete example demonstrating cluster monitoring and consistent hashing is provided:

```bash
# Run a single node
cargo run --example simple -p zuklink-yellowpage

# Run multiple nodes in different terminals
# Terminal 1 (seed node)
NODE_ID=node-1 cargo run --example simple -p zuklink-yellowpage

# Terminal 2 (joins node-1)
NODE_ID=node-2 SEEDS=127.0.0.1:7000 cargo run --example simple -p zuklink-yellowpage -- --listen 127.0.0.1:7001

# Terminal 3 (joins node-1)
NODE_ID=node-3 SEEDS=127.0.0.1:7000 cargo run --example simple -p zuklink-yellowpage -- --listen 127.0.0.1:7002
```

The example will:
- Initialize a Yellowpage instance
- Set node metadata (role, status, version)
- Monitor cluster membership every 5 seconds
- Demonstrate consistent hashing by showing which files each node would process
- Run for 30 seconds or until Ctrl+C

### Example Output

```
🟡 Starting Yellowpage Example

Configuration:
  Node ID: node-1
  Listen: 127.0.0.1:7000
  Seeds: []

✅ Yellowpage initialized

Monitoring cluster for 30 seconds...

📊 Cluster Status:
  Cluster size: 1
  My index: Some(0)
  Live nodes:
  👉 [0] node-1 (role: receiver, status: ready)

🔀 Sharding Demo:
  My files (6/6): ["data-001.zuk", "data-002.zuk", "data-003.zuk", "data-004.zuk", "data-005.zuk", "data-006.zuk"]
```

## Testing

```bash
# Run unit tests
cargo test -p zuklink-yellowpage

# Run with logging
RUST_LOG=zuklink_yellowpage=debug cargo test -p zuklink-yellowpage -- --nocapture
```

## Design Principles

### 1. Shared Nothing

Yellowpage does NOT require:
- Redis or any external database
- Centralized coordinator (Zookeeper, etcd, Consul)
- Shared filesystem

The only shared state is:
- Network connectivity between nodes
- Gossip messages exchanged via UDP

### 2. Eventually Consistent

The cluster view is eventually consistent. During network partitions or node failures:
- Different nodes may temporarily have different views
- The system continues to operate (AP in CAP theorem)
- Views converge once network heals

### 3. Deterministic Ordering

The `get_live_nodes()` method always returns a **sorted** list. This ensures all nodes with the same membership agree on node ordering, which is critical for consistent hashing.

## Comparison to Alternatives

| Feature | Yellowpage | Redis | Zookeeper | Consul |
|---------|------------|-------|-----------|--------|
| Shared Nothing | ✅ | ❌ | ❌ | ❌ |
| Embedded Library | ✅ | ❌ | ❌ | ❌ |
| Consistency | Eventual | Strong | Strong | Strong/Eventual |
| Failure Detection | Phi Accrual | Timeout | Heartbeat | Gossip |
| Operational Overhead | Low | Medium | High | Medium |

## Performance

- **Gossip Interval**: 500ms (configurable)
- **Network Protocol**: UDP (low overhead)
- **Message Size**: < 1KB per gossip message
- **Convergence Time**: O(log N) rounds for N nodes

## Limitations

1. **At-Least-Once Processing**: During topology changes, a file may be processed by two nodes temporarily
2. **Not for Strong Consistency**: Don't use for distributed locks or leader election
3. **UDP Requirements**: Requires UDP connectivity between all nodes

## License

Apache-2.0

## Credits

Built on top of [Chitchat](https://github.com/quickwit-oss/chitchat) by the Quickwit team.