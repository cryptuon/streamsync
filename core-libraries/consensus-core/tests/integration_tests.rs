//! Integration tests for consensus-core

use consensus_core::{ConsensusEngine, Config, Proposal, Result};
use consensus_core::transport::{InMemoryTransport, MockTransport};
use consensus_core::types::NodeId;
use uuid::Uuid;

type InMemoryState = std::collections::HashMap<NodeId, Vec<consensus_core::transport::Message>>;

/// Test basic consensus with 4 nodes
#[tokio::test]
async fn test_four_node_consensus() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    // Create 4 nodes
    let node_ids: Vec<NodeId> = (0..4).map(|_| Uuid::new_v4()).collect();
    let mut engines = Vec::new();

    // Create in-memory transports
    let transports = InMemoryTransport::create_network(node_ids.clone());

    // Create engines
    for &node_id in &node_ids {
        let config = Config::new(node_id, node_ids.clone())
            .with_request_timeout(std::time::Duration::from_secs(10))
            .with_debug_logs(true);

        let transport = transports.get(&node_id).unwrap();
        let engine = ConsensusEngine::new(config, transport.clone()).await?;
        engines.push(engine);
    }

    // Start all engines
    for engine in &engines {
        engine.start().await?;
    }

    // Wait for startup - needs enough time for all message loops to be ready
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Subscribe to results
    let mut receivers = Vec::new();
    for engine in &engines {
        receivers.push(engine.subscribe());
    }

    // Primary (first node) proposes
    let proposal = Proposal::new("test_proposal".to_string(), b"test_data".to_vec());
    let primary_engine = &engines[0];

    // Propose and wait for result
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        primary_engine.propose(proposal.clone())
    ).await.expect("Timeout").expect("Proposal failed");

    assert_eq!(result.proposal.id, "test_proposal");
    assert_eq!(result.sequence, 1);
    assert_eq!(result.view, 0);

    // All nodes should receive the result
    for receiver in &mut receivers {
        let received_result = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            receiver.recv()
        ).await.expect("Timeout").expect("Receiver failed");

        assert_eq!(received_result.proposal.id, "test_proposal");
        assert_eq!(received_result.sequence, 1);
    }

    // Stop all engines
    for engine in &engines {
        engine.stop().await?;
    }

    Ok(())
}

/// Test view change scenario
#[tokio::test]
async fn test_view_change() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    // Create 4 nodes
    let node_ids: Vec<NodeId> = (0..4).map(|_| Uuid::new_v4()).collect();
    let mut engines = Vec::new();

    // Create in-memory transports
    let transports = InMemoryTransport::create_network(node_ids.clone());

    // Create engines
    for &node_id in &node_ids {
        let config = Config::new(node_id, node_ids.clone())
            .with_view_change_timeout(std::time::Duration::from_millis(500))
            .with_debug_logs(true);

        let transport = transports.get(&node_id).unwrap();
        let engine = ConsensusEngine::new(config, transport.clone()).await?;
        engines.push(engine);
    }

    // Start all engines
    for engine in &engines {
        engine.start().await?;
    }

    // Wait for startup - needs enough time for all message loops to be ready
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Check initial view
    assert_eq!(engines[0].current_view().await, 0);

    // Trigger view change from backup node
    engines[1].trigger_view_change().await?;

    // Wait for view change to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // All nodes should be in new view
    for engine in &engines {
        assert_eq!(engine.current_view().await, 1);
    }

    // Stop all engines
    for engine in &engines {
        engine.stop().await?;
    }

    Ok(())
}

/// Test multiple proposals in sequence
#[tokio::test]
async fn test_multiple_proposals() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    // Create 4 nodes
    let node_ids: Vec<NodeId> = (0..4).map(|_| Uuid::new_v4()).collect();
    let mut engines = Vec::new();

    // Create in-memory transports
    let transports = InMemoryTransport::create_network(node_ids.clone());

    // Create engines
    for &node_id in &node_ids {
        let config = Config::new(node_id, node_ids.clone())
            .with_request_timeout(std::time::Duration::from_secs(10))
            .with_debug_logs(true);

        let transport = transports.get(&node_id).unwrap();
        let engine = ConsensusEngine::new(config, transport.clone()).await?;
        engines.push(engine);
    }

    // Start all engines
    for engine in &engines {
        engine.start().await?;
    }

    // Wait for startup - needs enough time for all message loops to be ready
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let primary_engine = &engines[0];

    // Propose multiple values in sequence
    for i in 1..=5 {
        let proposal = Proposal::new(format!("proposal_{}", i), format!("data_{}", i).into_bytes());

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            primary_engine.propose(proposal)
        ).await.expect("Timeout").expect("Proposal failed");

        assert_eq!(result.sequence, i);
        assert_eq!(result.view, 0);
    }

    // Check that all proposals were committed
    assert_eq!(primary_engine.last_committed().await, 5);

    // Stop all engines
    for engine in &engines {
        engine.stop().await?;
    }

    Ok(())
}

/// Test Byzantine fault tolerance with malicious node
#[tokio::test]
async fn test_byzantine_fault_tolerance() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    // Create 4 nodes (can tolerate 1 Byzantine fault)
    let node_ids: Vec<NodeId> = (0..4).map(|_| Uuid::new_v4()).collect();
    let mut honest_engines = Vec::new();

    // Create in-memory transports
    let transports = InMemoryTransport::create_network(node_ids.clone());

    // Create honest engines (skip one to simulate Byzantine node)
    for (i, &node_id) in node_ids.iter().enumerate() {
        if i == 3 { continue; } // Skip last node (Byzantine)

        let config = Config::new(node_id, node_ids.clone())
            .with_request_timeout(std::time::Duration::from_secs(10))
            .with_debug_logs(true);

        let transport = transports.get(&node_id).unwrap();
        let engine = ConsensusEngine::new(config, transport.clone()).await?;
        honest_engines.push(engine);
    }

    // Start honest engines
    for engine in &honest_engines {
        engine.start().await?;
    }

    // Wait for startup - needs enough time for all message loops to be ready
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Primary proposes (should still work with 3/4 nodes)
    let proposal = Proposal::new("byzantine_test".to_string(), b"test_data".to_vec());
    let primary_engine = &honest_engines[0];

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        primary_engine.propose(proposal)
    ).await.expect("Timeout").expect("Proposal failed");

    assert_eq!(result.proposal.id, "byzantine_test");
    assert_eq!(result.sequence, 1);

    // Stop engines
    for engine in &honest_engines {
        engine.stop().await?;
    }

    Ok(())
}

/// Test checkpointing functionality
#[tokio::test]
async fn test_checkpointing() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    // Create 4 nodes with small checkpoint interval
    let node_ids: Vec<NodeId> = (0..4).map(|_| Uuid::new_v4()).collect();
    let mut engines = Vec::new();

    // Create in-memory transports
    let transports = InMemoryTransport::create_network(node_ids.clone());

    // Create engines with small checkpoint interval
    for &node_id in &node_ids {
        let config = Config::new(node_id, node_ids.clone())
            .with_checkpoint_interval(3) // Checkpoint every 3 proposals
            .with_request_timeout(std::time::Duration::from_secs(10))
            .with_debug_logs(true);

        let transport = transports.get(&node_id).unwrap();
        let engine = ConsensusEngine::new(config, transport.clone()).await?;
        engines.push(engine);
    }

    // Start all engines
    for engine in &engines {
        engine.start().await?;
    }

    // Wait for startup - needs enough time for all message loops to be ready
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let primary_engine = &engines[0];

    // Propose 5 values to trigger checkpoint
    for i in 1..=5 {
        let proposal = Proposal::new(format!("checkpoint_test_{}", i), format!("data_{}", i).into_bytes());

        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            primary_engine.propose(proposal)
        ).await.expect("Timeout").expect("Proposal failed");
    }

    // Force checkpoint
    primary_engine.create_checkpoint().await?;

    // Wait for checkpoint to propagate
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Check statistics
    let stats = primary_engine.stats().await;
    assert_eq!(stats.total_committed, 5);
    assert!(stats.last_checkpoint > 0);

    // Stop all engines
    for engine in &engines {
        engine.stop().await?;
    }

    Ok(())
}

/// Test engine statistics
#[tokio::test]
async fn test_statistics() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    let node_ids: Vec<NodeId> = (0..4).map(|_| Uuid::new_v4()).collect();
    let config = Config::new(node_ids[0], node_ids.clone())
        .with_debug_logs(true);

    let transport = MockTransport::new(config.node_id, config.participants.clone());
    let engine = ConsensusEngine::new(config, transport).await?;

    engine.start().await?;

    // Initial stats
    let stats = engine.stats().await;
    assert_eq!(stats.current_view, 0);
    assert_eq!(stats.current_sequence, 0);
    assert_eq!(stats.total_committed, 0);
    assert_eq!(stats.participant_count, 4);
    assert_eq!(stats.success_rate, 1.0);

    engine.stop().await?;

    Ok(())
}

/// Test concurrent proposals (should be serialized)
#[tokio::test]
async fn test_concurrent_proposals() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    // Create 4 nodes
    let node_ids: Vec<NodeId> = (0..4).map(|_| Uuid::new_v4()).collect();
    let mut engines = Vec::new();

    // Create in-memory transports
    let transports = InMemoryTransport::create_network(node_ids.clone());

    // Create engines wrapped in Arc for sharing
    for &node_id in &node_ids {
        let config = Config::new(node_id, node_ids.clone())
            .with_request_timeout(std::time::Duration::from_secs(10))
            .with_debug_logs(true);

        let transport = transports.get(&node_id).unwrap();
        let engine = std::sync::Arc::new(ConsensusEngine::new(config, transport.clone()).await?);
        engines.push(engine);
    }

    // Start all engines
    for engine in &engines {
        engine.start().await?;
    }

    // Wait for startup - needs enough time for all message loops to be ready
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Launch concurrent proposals
    let mut tasks = Vec::new();
    for i in 1..=3 {
        let engine = engines[0].clone();
        let proposal = Proposal::new(format!("concurrent_{}", i), format!("data_{}", i).into_bytes());

        let task = tokio::spawn(async move {
            engine.propose(proposal).await
        });
        tasks.push(task);
    }

    // Wait for all to complete
    let mut results = Vec::new();
    for task in tasks {
        let result = task.await.expect("Task join failed").expect("Task failed");
        results.push(result);
    }

    // All should succeed with different sequence numbers
    assert_eq!(results.len(), 3);
    let mut sequences: Vec<_> = results.iter().map(|r| r.sequence).collect();
    sequences.sort();
    assert_eq!(sequences, vec![1, 2, 3]);

    // Stop all engines
    for engine in &engines {
        engine.stop().await?;
    }

    Ok(())
}

/// Test error conditions
#[tokio::test]
async fn test_error_conditions() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    // Test with insufficient nodes
    let node_ids: Vec<NodeId> = (0..2).map(|_| Uuid::new_v4()).collect(); // Only 2 nodes
    let config = Config::new(node_ids[0], node_ids.clone());

    // Should fail validation
    assert!(config.validate().is_err());

    // Test with valid config but not primary
    let node_ids: Vec<NodeId> = (0..4).map(|_| Uuid::new_v4()).collect();
    let mut config = Config::new(node_ids[1], node_ids.clone()); // Not primary for view 0
    config.node_id = node_ids[1];

    let transport = MockTransport::new(config.node_id, config.participants.clone());
    let engine = ConsensusEngine::new(config, transport).await?;

    engine.start().await?;

    let proposal = Proposal::new("test".to_string(), b"data".to_vec());

    // Should fail because not primary
    match engine.propose(proposal).await {
        Err(consensus_core::ConsensusError::NotPrimary { .. }) => {} // Expected
        _ => panic!("Should have failed with NotPrimary"),
    }

    engine.stop().await?;

    Ok(())
}