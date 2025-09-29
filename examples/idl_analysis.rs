//! IDL Analysis Example
//!
//! This example demonstrates how to use the IDL sync library to analyze
//! Solana program transactions and generate Interface Definition Language
//! specifications automatically.

use idl_sync::{
    IDLSyncLibrary,
    types::{IDLAnalysisConfig, IDLPattern},
};
use solana_sdk::pubkey::Pubkey;
use std::time::Duration;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("🔄 IDL Analysis Example");

    // Create IDL sync library with custom configuration
    let config = IDLAnalysisConfig {
        min_confidence_threshold: 0.8,
        analysis_window_size: 1000,
        pattern_detection_sensitivity: 0.7,
        consensus_required_nodes: 3,
    };

    let idl_lib = IDLSyncLibrary::new(config);

    // Example 1: Basic transaction analysis
    info!("📊 Example 1: Basic Transaction Analysis");
    basic_analysis_example(&idl_lib).await?;

    // Example 2: Real-time monitoring
    info!("⏱️ Example 2: Real-time Monitoring");
    realtime_monitoring_example(&idl_lib).await?;

    // Example 3: Network consensus simulation
    info!("🌐 Example 3: Network Consensus");
    network_consensus_example(&idl_lib).await?;

    // Example 4: Pattern evolution tracking
    info!("🧬 Example 4: Pattern Evolution");
    pattern_evolution_example(&idl_lib).await?;

    info!("✅ All IDL analysis examples completed!");
    Ok(())
}

async fn basic_analysis_example(
    idl_lib: &IDLSyncLibrary
) -> Result<(), Box<dyn std::error::Error>> {

    let program_id = Pubkey::new_unique();

    // Generate realistic transaction history
    let transaction_history = generate_realistic_transactions(100);

    info!("   Analyzing {} transactions for program {}",
          transaction_history.len(), program_id);

    let start = std::time::Instant::now();
    let generated_idl = idl_lib.analyze_program_transactions(
        &program_id,
        &transaction_history
    ).await?;

    let duration = start.elapsed();

    info!("✅ Analysis completed in {:?}", duration);
    info!("   📊 Instructions detected: {}", generated_idl.idl.instructions.len());
    info!("   📊 Account types detected: {}", generated_idl.idl.accounts.len());
    info!("   📊 Overall confidence: {:.2}%", generated_idl.confidence.overall_confidence * 100.0);

    // Display detailed instruction information
    for (i, instruction) in generated_idl.idl.instructions.iter().enumerate() {
        info!("   🔧 Instruction {}: {} (confidence: {:.2}%)",
              i + 1, instruction.name, instruction.confidence_score * 100.0);

        for (j, account) in instruction.accounts.iter().enumerate() {
            info!("     📝 Account {}: {} ({})",
                  j + 1, account.name, if account.is_mut { "mutable" } else { "read-only" });
        }
    }

    // Display account structures
    for (i, account) in generated_idl.idl.accounts.iter().enumerate() {
        info!("   🏛️ Account type {}: {} ({} fields)",
              i + 1, account.name, account.fields.len());
    }

    Ok(())
}

async fn realtime_monitoring_example(
    idl_lib: &IDLSyncLibrary
) -> Result<(), Box<dyn std::error::Error>> {

    let program_id = Pubkey::new_unique();

    // Start monitoring
    info!("   Starting real-time monitoring for program {}", program_id);
    idl_lib.start_monitoring(&program_id).await?;

    // Simulate incoming transactions over time
    for batch in 0..5 {
        info!("   Processing batch {} of transactions...", batch + 1);

        // Generate new transactions for this batch
        let new_transactions = generate_batch_transactions(batch, 20);

        // Process each transaction
        for (i, transaction) in new_transactions.iter().enumerate() {
            idl_lib.process_new_transaction(&program_id, transaction).await?;

            // Simulate processing delay
            if i % 5 == 0 {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        // Get current IDL state
        let current_idl = idl_lib.get_current_idl(&program_id).await?;
        info!("     Current IDL: {} instructions, {:.2}% confidence",
              current_idl.idl.instructions.len(),
              current_idl.confidence.overall_confidence * 100.0);

        // Small delay between batches
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    // Get final IDL state
    let final_idl = idl_lib.get_current_idl(&program_id).await?;
    info!("✅ Final IDL state: {} instructions, {} accounts",
          final_idl.idl.instructions.len(),
          final_idl.idl.accounts.len());

    // Stop monitoring
    idl_lib.stop_monitoring(&program_id).await?;
    info!("   Monitoring stopped");

    Ok(())
}

async fn network_consensus_example(
    idl_lib: &IDLSyncLibrary
) -> Result<(), Box<dyn std::error::Error>> {

    let program_id = Pubkey::new_unique();

    // Simulate multiple nodes analyzing the same program
    let mut node_analyses = Vec::new();

    for node_id in 0..5 {
        info!("   Node {} analyzing transactions...", node_id + 1);

        // Each node sees slightly different transaction sets
        let mut transactions = generate_realistic_transactions(80);

        // Add some node-specific variations
        if node_id > 0 {
            // Some nodes might miss recent transactions
            transactions.truncate(75 + node_id * 2);

            // Add some unique transactions
            let unique_txs = generate_batch_transactions(node_id, 5);
            transactions.extend(unique_txs);
        }

        let analysis = idl_lib.analyze_program_transactions(
            &program_id,
            &transactions
        ).await?;

        info!("     Node {}: {} instructions, {:.2}% confidence",
              node_id + 1,
              analysis.idl.instructions.len(),
              analysis.confidence.overall_confidence * 100.0);

        node_analyses.push(analysis);
    }

    // Reach network consensus
    info!("   Reaching network consensus...");
    let consensus_result = idl_lib.reach_network_consensus(
        &program_id,
        node_analyses
    ).await?;

    info!("✅ Consensus reached:");
    info!("   📊 Agreement level: {:.2}%", consensus_result.consensus_confidence * 100.0);
    info!("   📊 Agreed instructions: {}", consensus_result.agreed_idl.instructions.len());
    info!("   📊 Agreed accounts: {}", consensus_result.agreed_idl.accounts.len());
    info!("   📊 Participating nodes: {}", consensus_result.participating_nodes);

    // Show consensus details
    if consensus_result.consensus_confidence < 0.8 {
        warn!("   ⚠️ Low consensus - may need more analysis");

        for discrepancy in &consensus_result.discrepancies {
            warn!("     Discrepancy: {}", discrepancy);
        }
    }

    Ok(())
}

async fn pattern_evolution_example(
    idl_lib: &IDLSyncLibrary
) -> Result<(), Box<dyn std::error::Error>> {

    let program_id = Pubkey::new_unique();

    // Simulate program evolution over time
    let evolution_phases = vec![
        ("Initial Release", generate_v1_transactions()),
        ("Feature Update", generate_v2_transactions()),
        ("Major Upgrade", generate_v3_transactions()),
    ];

    let mut cumulative_transactions = Vec::new();

    for (phase_name, new_transactions) in evolution_phases {
        info!("   Phase: {}", phase_name);

        // Add new transactions to cumulative set
        cumulative_transactions.extend(new_transactions);

        // Analyze current state
        let current_analysis = idl_lib.analyze_program_transactions(
            &program_id,
            &cumulative_transactions
        ).await?;

        info!("     Instructions: {}", current_analysis.idl.instructions.len());
        info!("     Confidence: {:.2}%", current_analysis.confidence.overall_confidence * 100.0);

        // Check for new patterns
        let detected_patterns = detect_new_patterns(&current_analysis);
        if !detected_patterns.is_empty() {
            info!("     🆕 New patterns detected:");
            for pattern in detected_patterns {
                info!("       - {}", pattern.description);
            }
        }

        // Simulate time passing
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    // Generate evolution report
    let final_analysis = idl_lib.analyze_program_transactions(
        &program_id,
        &cumulative_transactions
    ).await?;

    info!("✅ Evolution tracking complete:");
    info!("   📈 Total transactions analyzed: {}", cumulative_transactions.len());
    info!("   📈 Final instruction count: {}", final_analysis.idl.instructions.len());
    info!("   📈 Final confidence: {:.2}%", final_analysis.confidence.overall_confidence * 100.0);

    Ok(())
}

/// Generate realistic transaction data with recognizable patterns
fn generate_realistic_transactions(count: usize) -> Vec<Vec<u8>> {
    let mut transactions = Vec::new();

    for i in 0..count {
        let mut tx_data = Vec::new();

        // Mock transaction signature (64 bytes)
        tx_data.extend_from_slice(&[0x42; 64]);

        // Instruction discriminator (varies by transaction type)
        let instruction_type = match i % 4 {
            0 => 0x00, // Initialize
            1 => 0x01, // Transfer
            2 => 0x02, // Update
            3 => 0x03, // Close
            _ => unreachable!(),
        };
        tx_data.push(instruction_type);

        // Instruction parameters (varies by type)
        match instruction_type {
            0x00 => { // Initialize
                tx_data.extend_from_slice(&[0x20]); // 32-byte parameter
                tx_data.extend_from_slice(&[0xFF; 32]); // Account seed
            },
            0x01 => { // Transfer
                tx_data.extend_from_slice(&[0x08]); // 8-byte amount
                tx_data.extend_from_slice(&(i as u64 * 1000).to_le_bytes());
            },
            0x02 => { // Update
                tx_data.extend_from_slice(&[0x04]); // 4-byte flags
                tx_data.extend_from_slice(&(i as u32).to_le_bytes());
            },
            0x03 => { // Close
                tx_data.extend_from_slice(&[0x01]); // 1-byte confirmation
                tx_data.push(0x01);
            },
            _ => {}
        }

        // Mock account keys (32 bytes each)
        for j in 0..3 {
            let mut key = [0u8; 32];
            key[0] = j;
            key[1] = (i % 256) as u8;
            tx_data.extend_from_slice(&key);
        }

        transactions.push(tx_data);
    }

    transactions
}

/// Generate batch-specific transactions for simulation
fn generate_batch_transactions(batch_id: usize, count: usize) -> Vec<Vec<u8>> {
    let mut transactions = Vec::new();

    for i in 0..count {
        let mut tx_data = Vec::new();

        // Signature
        tx_data.extend_from_slice(&[0x42; 64]);

        // Batch-specific instruction pattern
        let instruction_type = 0x10 + (batch_id % 4) as u8;
        tx_data.push(instruction_type);

        // Batch-specific parameters
        tx_data.extend_from_slice(&batch_id.to_le_bytes());
        tx_data.extend_from_slice(&i.to_le_bytes());

        // Account keys
        for j in 0..2 {
            let mut key = [0u8; 32];
            key[0] = (batch_id % 256) as u8;
            key[1] = j;
            tx_data.extend_from_slice(&key);
        }

        transactions.push(tx_data);
    }

    transactions
}

/// Generate v1 transaction patterns
fn generate_v1_transactions() -> Vec<Vec<u8>> {
    let mut transactions = Vec::new();

    for i in 0..30 {
        let mut tx_data = Vec::new();
        tx_data.extend_from_slice(&[0x42; 64]); // Signature
        tx_data.push(0x01); // V1 instruction
        tx_data.extend_from_slice(&(i as u32).to_le_bytes());

        // Simple account structure
        let mut key = [0u8; 32];
        key[0] = 0x01; // V1 marker
        tx_data.extend_from_slice(&key);

        transactions.push(tx_data);
    }

    transactions
}

/// Generate v2 transaction patterns (extended functionality)
fn generate_v2_transactions() -> Vec<Vec<u8>> {
    let mut transactions = Vec::new();

    for i in 0..25 {
        let mut tx_data = Vec::new();
        tx_data.extend_from_slice(&[0x42; 64]); // Signature
        tx_data.push(0x02); // V2 instruction
        tx_data.extend_from_slice(&(i as u64).to_le_bytes()); // Extended parameter

        // More complex account structure
        for j in 0..2 {
            let mut key = [0u8; 32];
            key[0] = 0x02; // V2 marker
            key[1] = j;
            tx_data.extend_from_slice(&key);
        }

        transactions.push(tx_data);
    }

    transactions
}

/// Generate v3 transaction patterns (major upgrade)
fn generate_v3_transactions() -> Vec<Vec<u8>> {
    let mut transactions = Vec::new();

    for i in 0..20 {
        let mut tx_data = Vec::new();
        tx_data.extend_from_slice(&[0x42; 64]); // Signature
        tx_data.push(0x03); // V3 instruction
        tx_data.push(0x04); // V3 sub-instruction
        tx_data.extend_from_slice(&(i as u128).to_le_bytes()); // Very extended parameter

        // Complex account structure
        for j in 0..4 {
            let mut key = [0u8; 32];
            key[0] = 0x03; // V3 marker
            key[1] = j;
            key[2] = (i % 256) as u8;
            tx_data.extend_from_slice(&key);
        }

        transactions.push(tx_data);
    }

    transactions
}

/// Detect new patterns in IDL analysis (simplified)
fn detect_new_patterns(analysis: &idl_sync::types::GeneratedIDL) -> Vec<IDLPattern> {
    let mut patterns = Vec::new();

    // Simple pattern detection based on instruction count
    if analysis.idl.instructions.len() > 5 {
        patterns.push(IDLPattern {
            pattern_type: "ComplexProgram".to_string(),
            description: "Program with multiple instruction types detected".to_string(),
            confidence: 0.8,
            occurrences: analysis.idl.instructions.len(),
        });
    }

    if analysis.idl.accounts.len() > 3 {
        patterns.push(IDLPattern {
            pattern_type: "MultiAccountProgram".to_string(),
            description: "Program using multiple account types detected".to_string(),
            confidence: 0.9,
            occurrences: analysis.idl.accounts.len(),
        });
    }

    patterns
}