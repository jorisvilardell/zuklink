//! Simple example demonstrating Yellowpage usage
//!
//! This example shows how to:
//! - Initialize a Yellowpage instance
//! - Set node metadata
//! - Monitor cluster membership
//! - Compute consistent hashing for file distribution
//!
//! Run with:
//! ```bash
//! cargo run --example simple
//! ```

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use zuklink_yellowpage::Yellowpage;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🟡 Starting Yellowpage Example\n");

    // Configuration
    let node_id = std::env::var("NODE_ID").unwrap_or_else(|_| "example-node".to_string());
    let listen_addr = "127.0.0.1:7000".parse()?;
    let seeds = std::env::var("SEEDS")
        .ok()
        .map(|s| s.split(',').map(String::from).collect())
        .unwrap_or_default();

    println!("Configuration:");
    println!("  Node ID: {}", node_id);
    println!("  Listen: {}", listen_addr);
    println!("  Seeds: {:?}\n", seeds);

    // Initialize Yellowpage
    let yellowpage = Yellowpage::new(node_id.clone(), listen_addr, seeds).await?;

    // Set node metadata
    yellowpage.set_metadata("role", "receiver").await;
    yellowpage.set_metadata("status", "ready").await;
    yellowpage.set_metadata("version", "0.1.0").await;

    println!("✅ Yellowpage initialized\n");

    // Monitor cluster membership
    println!("Monitoring cluster for 30 seconds...\n");

    let mut interval = tokio::time::interval(Duration::from_secs(5));
    let mut iterations = 0;

    loop {
        tokio::select! {
            _ = interval.tick() => {
                print_cluster_status(&yellowpage).await;
                demonstrate_sharding(&yellowpage).await;
                println!();

                iterations += 1;
                if iterations >= 6 {
                    break;
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\n🛑 Received shutdown signal");
                break;
            }
        }
    }

    // Graceful shutdown
    println!("Shutting down...");
    yellowpage.shutdown().await;
    println!("✅ Shutdown complete");

    Ok(())
}

/// Print current cluster status
async fn print_cluster_status(yellowpage: &Yellowpage) {
    let live_nodes = yellowpage.get_live_nodes().await;
    let cluster_size = live_nodes.len();
    let my_index = yellowpage.my_index().await;

    println!("📊 Cluster Status:");
    println!("  Cluster size: {}", cluster_size);
    println!("  My index: {:?}", my_index);
    println!("  Live nodes:");

    for (idx, node) in live_nodes.iter().enumerate() {
        let is_me = Some(idx) == my_index;
        let marker = if is_me { "👉" } else { "  " };

        // Try to fetch node metadata
        let role = yellowpage
            .get_metadata(node, "role")
            .await
            .unwrap_or_else(|| "unknown".to_string());

        let status = yellowpage
            .get_metadata(node, "status")
            .await
            .unwrap_or_else(|| "unknown".to_string());

        println!(
            "  {} [{}] {} (role: {}, status: {})",
            marker, idx, node, role, status
        );
    }
}

/// Demonstrate consistent hashing for file distribution
async fn demonstrate_sharding(yellowpage: &Yellowpage) {
    let live_nodes = yellowpage.get_live_nodes().await;
    let cluster_size = live_nodes.len();

    if cluster_size == 0 {
        println!("⚠️  No nodes in cluster - skipping sharding demo");
        return;
    }

    let my_index = match yellowpage.my_index().await {
        Some(idx) => idx,
        None => {
            println!("⚠️  Cannot determine my index - skipping sharding demo");
            return;
        }
    };

    println!("🔀 Sharding Demo:");

    // Simulate some files
    let test_files = vec![
        "data-001.zuk",
        "data-002.zuk",
        "data-003.zuk",
        "data-004.zuk",
        "data-005.zuk",
        "data-006.zuk",
    ];

    let mut my_files = Vec::new();
    let mut other_files = Vec::new();

    for filename in &test_files {
        if should_process_file(filename, my_index, cluster_size) {
            my_files.push(*filename);
        } else {
            other_files.push(*filename);
        }
    }

    println!(
        "  My files ({}/{}): {:?}",
        my_files.len(),
        test_files.len(),
        my_files
    );
    if !other_files.is_empty() {
        println!("  Other nodes' files: {:?}", other_files);
    }
}

/// Determine if this node should process a file based on consistent hashing
fn should_process_file(filename: &str, my_index: usize, cluster_size: usize) -> bool {
    if cluster_size == 0 {
        return false;
    }

    let mut hasher = DefaultHasher::new();
    filename.hash(&mut hasher);
    let hash = hasher.finish();

    (hash as usize % cluster_size) == my_index
}
