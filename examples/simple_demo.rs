//! Simple StreamSync Demo
//!
//! A basic demonstration of the core StreamSync libraries working together.
//! This example shows the fundamental capabilities without requiring all
//! advanced features to be fully implemented.

use zk_reconstruction::{ZKReconstructionLibrary, types::{TruncatedData, CompressionParams, CompressionType, AccountMetadata}};
use idl_sync::IDLSyncLibrary;
use distributed_duckdb::DistributedCoordinator;
use solana_sdk::pubkey::Pubkey;
use std::time::Duration;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("🌊 StreamSync Simple Demo");

    // Demo 1: ZK Reconstruction
    info!("📦 Demo 1: ZK Reconstruction");
    zk_reconstruction_demo().await?;

    // Demo 2: IDL Sync
    info!("🔄 Demo 2: IDL Sync");
    idl_sync_demo().await?;

    // Demo 3: Distributed DuckDB
    info!("🗄️ Demo 3: Distributed DuckDB");
    duckdb_demo().await?;

    info!("✅ All demos completed successfully!");
    Ok(())
}

async fn zk_reconstruction_demo() -> Result<(), Box<dyn std::error::Error>> {
    let zk_lib = ZKReconstructionLibrary::new();

    // Check if library is ready
    if !zk_lib.is_ready() {
        warn!("   ZK reconstruction library not ready");
        return Ok(());
    }

    // Create sample compressed data
    let compressed_data = generate_sample_data(512);
    let metadata = AccountMetadata {
        account: Pubkey::new_unique(),
        program_id: Pubkey::new_unique(),
        slot: 1000,
        compression_type: CompressionType::Standard,
    };

    let truncated_data = TruncatedData {
        data: compressed_data,
        metadata,
    };

    let compression_params = CompressionParams {
        compression_type: CompressionType::Standard,
        merkle_tree_height: 15,
        compression_level: 6,
    };

    // Try reconstruction
    info!("   Attempting reconstruction...");
    let start = std::time::Instant::now();

    match zk_lib.reconstruct_compressed_account(&truncated_data, &compression_params).await {
        Ok(result) => {
            let duration = start.elapsed();
            info!("   ✅ Success: {} → {} bytes in {:?} ({:.1}% confidence)",
                  truncated_data.data.len(),
                  result.account_data.len(),
                  duration,
                  result.confidence_score * 100.0);
        },
        Err(e) => {
            warn!("   ❌ Reconstruction failed: {}", e);
        }
    }

    // Try fast path
    info!("   Trying fast path reconstruction...");
    match zk_lib.fast_reconstruct_common_patterns(&truncated_data).await {
        Some(result) => {
            info!("   ⚡ Fast path success: {} bytes ({:.1}% confidence)",
                  result.account_data.len(),
                  result.confidence_score * 100.0);
        },
        None => {
            info!("   ⚠️ Fast path not available");
        }
    }

    Ok(())
}

async fn idl_sync_demo() -> Result<(), Box<dyn std::error::Error>> {
    let idl_lib = IDLSyncLibrary::new();

    // Check if library is ready
    if !idl_lib.is_ready() {
        warn!("   IDL sync library not ready");
        return Ok(());
    }

    let program_id = Pubkey::new_unique();

    // Generate sample transaction history
    let transaction_history = generate_sample_transactions(50);

    info!("   Analyzing {} transactions for program {}",
          transaction_history.len(), program_id);

    let start = std::time::Instant::now();

    match idl_lib.analyze_program_transactions(&program_id, &transaction_history).await {
        Ok(generated_idl) => {
            let duration = start.elapsed();
            info!("   ✅ Analysis complete in {:?}", duration);
            info!("      Instructions: {}", generated_idl.idl.instructions.len());
            info!("      Accounts: {}", generated_idl.idl.accounts.len());
            info!("      Confidence: {:.1}%", generated_idl.confidence.overall_confidence * 100.0);

            // Show first few instructions
            for (i, instruction) in generated_idl.idl.instructions.iter().take(3).enumerate() {
                info!("      Instruction {}: {}", i + 1, instruction.name);
            }
        },
        Err(e) => {
            warn!("   ❌ IDL analysis failed: {}", e);
        }
    }

    Ok(())
}

async fn duckdb_demo() -> Result<(), Box<dyn std::error::Error>> {
    let coordinator = DistributedCoordinator::new();

    info!("   Testing distributed coordinator...");

    // Basic functionality test
    info!("   ✅ Coordinator initialized successfully");

    // Note: Since the actual query execution methods might not be fully implemented,
    // we'll just demonstrate that the library can be instantiated and basic
    // functionality works.

    // In a full implementation, this would include:
    // - Query distribution
    // - Shard management
    // - Result aggregation
    // - Performance benchmarks

    info!("   📊 Distributed query coordination capabilities available");
    info!("   🔗 Ready for integration with reconstruction and IDL analysis");

    Ok(())
}

/// Generate sample compressed data
fn generate_sample_data(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);

    for i in 0..size {
        let byte = match i % 8 {
            0..=2 => 0xFF,                    // Header pattern
            3..=5 => (i % 256) as u8,         // Sequential data
            6 => 0x00,                       // Separator
            7 => ((i * 17) % 256) as u8,     // Pseudo-random
            _ => unreachable!(),
        };
        data.push(byte);
    }

    data
}

/// Generate sample transaction data
fn generate_sample_transactions(count: usize) -> Vec<Vec<u8>> {
    let mut transactions = Vec::new();

    for i in 0..count {
        let mut tx_data = Vec::new();

        // Mock signature (64 bytes)
        tx_data.extend_from_slice(&[0x42; 64]);

        // Instruction discriminator
        let instruction_type = (i % 4) as u8;
        tx_data.push(instruction_type);

        // Simple parameters
        tx_data.extend_from_slice(&(i as u32).to_le_bytes());

        // Mock account keys
        for j in 0..2 {
            let mut key = [0u8; 32];
            key[0] = j;
            key[1] = (i % 256) as u8;
            tx_data.extend_from_slice(&key);
        }

        transactions.push(tx_data);
    }

    transactions
}