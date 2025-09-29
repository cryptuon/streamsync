//! Basic ZK Reconstruction Example
//!
//! This example demonstrates how to use the ZK reconstruction library
//! to reconstruct compressed account data with various strategies.

use zk_reconstruction::{
    ZKReconstructionLibrary,
    types::{TruncatedData, CompressionParams, CompressionType, AccountMetadata, ReconstructionConfig},
};
use solana_sdk::pubkey::Pubkey;
use std::time::Duration;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("🚀 Basic ZK Reconstruction Example");

    // Create reconstruction library with custom configuration
    let config = ReconstructionConfig {
        cache_size: 1000,
        max_reconstruction_time: Duration::from_secs(10),
        enable_pattern_learning: true,
        confidence_threshold: 0.7,
    };

    let zk_lib = ZKReconstructionLibrary::with_config(config);

    // Example 1: Basic reconstruction
    info!("📦 Example 1: Basic Reconstruction");
    basic_reconstruction_example(&zk_lib).await?;

    // Example 2: Pattern learning and reuse
    info!("🧠 Example 2: Pattern Learning");
    pattern_learning_example(&zk_lib).await?;

    // Example 3: Fast path reconstruction
    info!("⚡ Example 3: Fast Path Reconstruction");
    fast_path_example(&zk_lib).await?;

    // Example 4: Different compression types
    info!("🔧 Example 4: Multiple Compression Types");
    multiple_compression_types_example(&zk_lib).await?;

    info!("✅ All examples completed successfully!");
    Ok(())
}

async fn basic_reconstruction_example(
    zk_lib: &ZKReconstructionLibrary
) -> Result<(), Box<dyn std::error::Error>> {

    // Simulate compressed account data
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

    // Perform reconstruction
    let start = std::time::Instant::now();
    let result = zk_lib.reconstruct_compressed_account(
        &truncated_data,
        &compression_params
    ).await?;

    let duration = start.elapsed();

    info!("✅ Reconstruction completed in {:?}", duration);
    info!("   📊 Original size: {} bytes", truncated_data.data.len());
    info!("   📊 Reconstructed size: {} bytes", result.account_data.len());
    info!("   📊 Confidence: {:.2}%", result.confidence_score * 100.0);
    info!("   📊 Method: {:?}", result.reconstruction_method);

    Ok(())
}

async fn pattern_learning_example(
    zk_lib: &ZKReconstructionLibrary
) -> Result<(), Box<dyn std::error::Error>> {

    let program_id = Pubkey::new_unique();

    // Create multiple similar accounts to establish patterns
    for i in 0..5 {
        let mut compressed_data = generate_sample_data(256);

        // Add a recognizable pattern
        compressed_data[0..4].copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
        compressed_data.extend_from_slice(&format!("pattern_{}", i).as_bytes());

        let metadata = AccountMetadata {
            account: Pubkey::new_unique(),
            program_id, // Same program to build patterns
            slot: 2000 + i as u64,
            compression_type: CompressionType::StateCompression,
        };

        let truncated_data = TruncatedData {
            data: compressed_data,
            metadata,
        };

        let compression_params = CompressionParams {
            compression_type: CompressionType::StateCompression,
            merkle_tree_height: 12,
            compression_level: 5,
        };

        let start = std::time::Instant::now();
        let result = zk_lib.reconstruct_compressed_account(
            &truncated_data,
            &compression_params
        ).await?;

        let duration = start.elapsed();

        info!("   Account {}: {:?}, confidence: {:.2}%, method: {:?}",
              i + 1, duration, result.confidence_score * 100.0, result.reconstruction_method);

        // Pattern learning should make subsequent reconstructions faster
        if i > 2 && duration > Duration::from_millis(50) {
            warn!("   Pattern learning may not be working optimally");
        }
    }

    Ok(())
}

async fn fast_path_example(
    zk_lib: &ZKReconstructionLibrary
) -> Result<(), Box<dyn std::error::Error>> {

    // Create data that should match existing patterns
    let compressed_data = generate_sample_data(256);
    let metadata = AccountMetadata {
        account: Pubkey::new_unique(),
        program_id: Pubkey::new_unique(),
        slot: 3000,
        compression_type: CompressionType::Standard,
    };

    let truncated_data = TruncatedData {
        data: compressed_data,
        metadata,
    };

    // Try fast path first
    let start = std::time::Instant::now();
    match zk_lib.fast_reconstruct_common_patterns(&truncated_data).await {
        Some(result) => {
            let duration = start.elapsed();
            info!("✅ Fast path successful in {:?}", duration);
            info!("   📊 Reconstructed {} bytes with {:.2}% confidence",
                  result.account_data.len(), result.confidence_score * 100.0);
        },
        None => {
            info!("⚠️  Fast path not available, would use full reconstruction");

            // Fall back to full reconstruction
            let compression_params = CompressionParams {
                compression_type: CompressionType::Standard,
                merkle_tree_height: 10,
                compression_level: 4,
            };

            let result = zk_lib.reconstruct_compressed_account(
                &truncated_data,
                &compression_params
            ).await?;

            let duration = start.elapsed();
            info!("✅ Full reconstruction completed in {:?}", duration);
            info!("   📊 Confidence: {:.2}%", result.confidence_score * 100.0);
        }
    }

    Ok(())
}

async fn multiple_compression_types_example(
    zk_lib: &ZKReconstructionLibrary
) -> Result<(), Box<dyn std::error::Error>> {

    let compression_types = vec![
        ("Standard", CompressionType::Standard),
        ("State Compression", CompressionType::StateCompression),
        ("Custom Algorithm", CompressionType::Custom("lz4_optimized".to_string())),
    ];

    for (name, compression_type) in compression_types {
        info!("   Testing {} compression...", name);

        let compressed_data = generate_sample_data(384);
        let metadata = AccountMetadata {
            account: Pubkey::new_unique(),
            program_id: Pubkey::new_unique(),
            slot: 4000,
            compression_type: compression_type.clone(),
        };

        let truncated_data = TruncatedData {
            data: compressed_data,
            metadata,
        };

        let compression_params = CompressionParams {
            compression_type,
            merkle_tree_height: 14,
            compression_level: 6,
        };

        match zk_lib.reconstruct_compressed_account(
            &truncated_data,
            &compression_params
        ).await {
            Ok(result) => {
                info!("     ✅ Success: {} bytes, {:.2}% confidence",
                      result.account_data.len(), result.confidence_score * 100.0);
            },
            Err(e) => {
                warn!("     ❌ Failed: {}", e);
            }
        }
    }

    Ok(())
}

/// Generate sample compressed data for testing
fn generate_sample_data(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);

    // Create realistic-looking compressed data patterns
    for i in 0..size {
        let byte = match i % 16 {
            0..=3 => 0xFF,                    // Common header pattern
            4..=7 => (i % 256) as u8,         // Sequential data
            8..=11 => 0x00,                   // Zero padding
            12..=15 => ((i * 37) % 256) as u8, // Pseudo-random content
            _ => unreachable!(),
        };
        data.push(byte);
    }

    // Add some entropy at the end
    data.extend_from_slice(b"ENTROPY");
    data.truncate(size);

    data
}