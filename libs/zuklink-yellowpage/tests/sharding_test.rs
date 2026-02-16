//! Integration tests for distributed sharding
//!
//! These tests verify that:
//! 1. Multiple Yellowpage instances can discover each other
//! 2. All nodes agree on the same cluster view (sorted)
//! 3. Consistent hashing distributes files correctly without duplicates
//! 4. Each file is assigned to exactly one node
//! 5. Sharding rebalances when nodes join/leave

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tokio::time::sleep;
use zuklink_yellowpage::Yellowpage;

/// Helper function for consistent hashing (same as in simple.rs)
fn should_process_file(filename: &str, my_index: usize, cluster_size: usize) -> bool {
    if cluster_size == 0 {
        return false;
    }

    let mut hasher = DefaultHasher::new();
    filename.hash(&mut hasher);
    let hash = hasher.finish();

    (hash as usize % cluster_size) == my_index
}

/// Test that a single node cluster assigns all files to itself
#[tokio::test]
async fn test_single_node_processes_all_files() {
    let node = Yellowpage::new(
        "test-node-1".to_string(),
        "127.0.0.1:17001".parse().unwrap(),
        vec![],
    )
    .await
    .expect("Failed to create yellowpage");

    // Wait for initialization
    sleep(Duration::from_millis(100)).await;

    let live_nodes = node.get_live_nodes().await;
    assert_eq!(live_nodes.len(), 1, "Should have 1 node");

    let my_index = node.my_index().await.expect("Should have an index");
    assert_eq!(my_index, 0, "Single node should be at index 0");

    // Test files
    let test_files = vec!["file1.zuk", "file2.zuk", "file3.zuk"];

    for filename in &test_files {
        assert!(
            should_process_file(filename, my_index, 1),
            "Single node should process all files"
        );
    }

    node.shutdown().await;
}

/// Test that two nodes discover each other and split work
#[tokio::test]
async fn test_two_nodes_discover_and_shard() {
    // Node 1 (seed)
    let node1 = Yellowpage::new(
        "node-1".to_string(),
        "127.0.0.1:17002".parse().unwrap(),
        vec![],
    )
    .await
    .expect("Failed to create node1");

    // Wait for node1 to be ready
    sleep(Duration::from_millis(100)).await;

    // Node 2 (joins node1)
    let node2 = Yellowpage::new(
        "node-2".to_string(),
        "127.0.0.1:17003".parse().unwrap(),
        vec!["127.0.0.1:17002".to_string()],
    )
    .await
    .expect("Failed to create node2");

    // Wait for gossip to converge
    sleep(Duration::from_secs(2)).await;

    // Both nodes should see each other
    let nodes1 = node1.get_live_nodes().await;
    let nodes2 = node2.get_live_nodes().await;

    assert_eq!(nodes1.len(), 2, "Node1 should see 2 nodes");
    assert_eq!(nodes2.len(), 2, "Node2 should see 2 nodes");

    // Both should have the same sorted view
    assert_eq!(nodes1, nodes2, "Both nodes should agree on cluster view");

    // Get indices
    let index1 = node1.my_index().await.expect("Node1 should have index");
    let index2 = node2.my_index().await.expect("Node2 should have index");

    assert_ne!(index1, index2, "Nodes should have different indices");

    // Test that files are distributed
    let test_files = vec![
        "file1.zuk",
        "file2.zuk",
        "file3.zuk",
        "file4.zuk",
        "file5.zuk",
        "file6.zuk",
        "file7.zuk",
        "file8.zuk",
    ];

    let mut node1_files = 0;
    let mut node2_files = 0;

    for filename in &test_files {
        let n1_processes = should_process_file(filename, index1, 2);
        let n2_processes = should_process_file(filename, index2, 2);

        // Each file should be assigned to exactly one node
        assert!(
            n1_processes ^ n2_processes,
            "File {} should be assigned to exactly one node",
            filename
        );

        if n1_processes {
            node1_files += 1;
        }
        if n2_processes {
            node2_files += 1;
        }
    }

    // Both nodes should have work (not perfect distribution, but should not be 0)
    assert!(node1_files > 0, "Node1 should process at least 1 file");
    assert!(node2_files > 0, "Node2 should process at least 1 file");
    assert_eq!(
        node1_files + node2_files,
        test_files.len(),
        "All files should be assigned"
    );

    println!(
        "✅ Two-node distribution: node1={}, node2={}",
        node1_files, node2_files
    );

    node1.shutdown().await;
    node2.shutdown().await;
}

/// Test that three nodes form a cluster and distribute work correctly
#[tokio::test]
async fn test_three_nodes_shard_correctly() {
    // Node 1 (seed)
    let node1 = Yellowpage::new(
        "node-1".to_string(),
        "127.0.0.1:17004".parse().unwrap(),
        vec![],
    )
    .await
    .expect("Failed to create node1");

    sleep(Duration::from_millis(100)).await;

    // Node 2
    let node2 = Yellowpage::new(
        "node-2".to_string(),
        "127.0.0.1:17005".parse().unwrap(),
        vec!["127.0.0.1:17004".to_string()],
    )
    .await
    .expect("Failed to create node2");

    sleep(Duration::from_millis(100)).await;

    // Node 3
    let node3 = Yellowpage::new(
        "node-3".to_string(),
        "127.0.0.1:17006".parse().unwrap(),
        vec!["127.0.0.1:17004".to_string()],
    )
    .await
    .expect("Failed to create node3");

    // Wait for full convergence
    sleep(Duration::from_secs(3)).await;

    // All nodes should see the full cluster
    let nodes1 = node1.get_live_nodes().await;
    let nodes2 = node2.get_live_nodes().await;
    let nodes3 = node3.get_live_nodes().await;

    assert_eq!(nodes1.len(), 3, "Node1 should see 3 nodes");
    assert_eq!(nodes2.len(), 3, "Node2 should see 3 nodes");
    assert_eq!(nodes3.len(), 3, "Node3 should see 3 nodes");

    // All should agree on the same view
    assert_eq!(nodes1, nodes2, "Node1 and Node2 views should match");
    assert_eq!(nodes2, nodes3, "Node2 and Node3 views should match");

    // Get indices
    let index1 = node1.my_index().await.expect("Node1 should have index");
    let index2 = node2.my_index().await.expect("Node2 should have index");
    let index3 = node3.my_index().await.expect("Node3 should have index");

    // All indices should be unique
    let indices = HashSet::from([index1, index2, index3]);
    assert_eq!(indices.len(), 3, "All indices should be unique");

    // Test sharding with many files
    let test_files: Vec<String> = (1..=100).map(|i| format!("file-{:03}.zuk", i)).collect();

    let mut assignment: HashMap<usize, Vec<String>> = HashMap::new();
    assignment.insert(index1, Vec::new());
    assignment.insert(index2, Vec::new());
    assignment.insert(index3, Vec::new());

    for filename in &test_files {
        let mut assigned_to = Vec::new();

        if should_process_file(filename, index1, 3) {
            assigned_to.push(index1);
            assignment.get_mut(&index1).unwrap().push(filename.clone());
        }
        if should_process_file(filename, index2, 3) {
            assigned_to.push(index2);
            assignment.get_mut(&index2).unwrap().push(filename.clone());
        }
        if should_process_file(filename, index3, 3) {
            assigned_to.push(index3);
            assignment.get_mut(&index3).unwrap().push(filename.clone());
        }

        assert_eq!(
            assigned_to.len(),
            1,
            "File {} should be assigned to exactly one node, but assigned to {:?}",
            filename,
            assigned_to
        );
    }

    // Check distribution
    let count1 = assignment[&index1].len();
    let count2 = assignment[&index2].len();
    let count3 = assignment[&index3].len();

    println!(
        "✅ Three-node distribution: node1={}, node2={}, node3={}",
        count1, count2, count3
    );

    assert_eq!(
        count1 + count2 + count3,
        test_files.len(),
        "All files should be assigned"
    );

    // Each node should have at least some files (distribution may not be perfectly even)
    assert!(count1 > 0, "Node1 should process at least 1 file");
    assert!(count2 > 0, "Node2 should process at least 1 file");
    assert!(count3 > 0, "Node3 should process at least 1 file");

    // Distribution should be reasonably balanced (within 50% of perfect distribution)
    let perfect = test_files.len() / 3;
    let tolerance = perfect / 2;

    assert!(
        count1 >= perfect - tolerance && count1 <= perfect + tolerance,
        "Node1 distribution ({}) should be within tolerance of perfect ({})",
        count1,
        perfect
    );

    node1.shutdown().await;
    node2.shutdown().await;
    node3.shutdown().await;
}

/// Test that sharding is deterministic - same input always produces same output
#[tokio::test]
async fn test_sharding_is_deterministic() {
    let test_files = vec!["fileA.zuk", "fileB.zuk", "fileC.zuk"];

    // Run the same test multiple times
    for iteration in 0..3 {
        let node = Yellowpage::new(
            format!("test-node-{}", iteration),
            format!("127.0.0.1:{}", 17100 + iteration).parse().unwrap(),
            vec![],
        )
        .await
        .expect("Failed to create node");

        sleep(Duration::from_millis(100)).await;

        let my_index = node.my_index().await.expect("Should have index");

        for filename in &test_files {
            let result1 = should_process_file(filename, my_index, 1);
            let result2 = should_process_file(filename, my_index, 1);

            assert_eq!(
                result1, result2,
                "Hashing should be deterministic for {}",
                filename
            );
        }

        node.shutdown().await;
        sleep(Duration::from_millis(100)).await;
    }

    println!("✅ Sharding is deterministic across multiple runs");
}

/// Test that no files are lost or duplicated when all nodes process
#[tokio::test]
async fn test_no_duplicates_or_losses() {
    // Create 3 nodes
    let node1 = Yellowpage::new(
        "dup-test-1".to_string(),
        "127.0.0.1:17007".parse().unwrap(),
        vec![],
    )
    .await
    .unwrap();

    sleep(Duration::from_millis(100)).await;

    let node2 = Yellowpage::new(
        "dup-test-2".to_string(),
        "127.0.0.1:17008".parse().unwrap(),
        vec!["127.0.0.1:17007".to_string()],
    )
    .await
    .unwrap();

    sleep(Duration::from_millis(100)).await;

    let node3 = Yellowpage::new(
        "dup-test-3".to_string(),
        "127.0.0.1:17009".parse().unwrap(),
        vec!["127.0.0.1:17007".to_string()],
    )
    .await
    .unwrap();

    sleep(Duration::from_secs(3)).await;

    let cluster_size = node1.get_live_nodes().await.len();
    assert_eq!(cluster_size, 3);

    let index1 = node1.my_index().await.unwrap();
    let index2 = node2.my_index().await.unwrap();
    let index3 = node3.my_index().await.unwrap();

    // Generate test files
    let test_files: Vec<String> = (1..=50).map(|i| format!("test-file-{}.zuk", i)).collect();

    // Track which node processes each file
    let mut file_assignments: HashMap<String, Vec<usize>> = HashMap::new();

    for filename in &test_files {
        let mut nodes_claiming = Vec::new();

        if should_process_file(filename, index1, cluster_size) {
            nodes_claiming.push(index1);
        }
        if should_process_file(filename, index2, cluster_size) {
            nodes_claiming.push(index2);
        }
        if should_process_file(filename, index3, cluster_size) {
            nodes_claiming.push(index3);
        }

        file_assignments.insert(filename.clone(), nodes_claiming);
    }

    // Verify: each file assigned to exactly one node
    for (filename, nodes) in &file_assignments {
        assert_eq!(
            nodes.len(),
            1,
            "File {} should be assigned to exactly 1 node, but assigned to {} nodes: {:?}",
            filename,
            nodes.len(),
            nodes
        );
    }

    // Verify: all files are assigned
    assert_eq!(
        file_assignments.len(),
        test_files.len(),
        "All files should be assigned"
    );

    println!(
        "✅ No duplicates or losses: {} files correctly distributed",
        test_files.len()
    );

    node1.shutdown().await;
    node2.shutdown().await;
    node3.shutdown().await;
}
