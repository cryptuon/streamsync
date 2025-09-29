//! Simple test for adaptive verification thresholds
//! Validates that the calibrated verification system works with realistic patterns

use zk_reconstruction::{
    types::{TruncatedData, CompressionParams, TruncationMetadata, CompressionType},
    adaptive_verification::AdaptiveVerifier,
};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use tracing::info;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    info!("🧪 Testing Adaptive Verification Threshold Calibration");

    // Test adaptive verification directly
    test_spl_token_pattern().await?;
    test_state_compression_pattern().await?;

    info!("✅ Adaptive verification threshold tests completed successfully!");
    Ok(())
}

async fn test_spl_token_pattern() -> Result<()> {
    info!("🪙 Testing SPL Token pattern with adaptive thresholds");

    let verifier = AdaptiveVerifier::default();

    // Create SPL Token transfer data
    let token_data = vec![3, 232, 3, 0, 0, 0, 0, 0, 0]; // Transfer with 1000 lamports

    let truncated_data = TruncatedData {
        data: token_data.clone(),
        original_size_hint: Some(3600), // 400x expansion hint
        truncation_point: token_data.len(),
        metadata: TruncationMetadata {
            account: Pubkey::new_unique(),
            program_id: Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")?,
            slot: 250_000_000,
            compression_type: CompressionType::Standard,
            truncation_timestamp: std::time::SystemTime::now(),
        },
    };

    let compression_params = CompressionParams::default();

    // Create a realistic reconstruction result for SPL Token (400x expansion within 500.0 limit)
    let reconstructed = zk_reconstruction::types::ReconstructedAccount {
        account_data: vec![0u8; token_data.len() * 400], // 400x expansion
        confidence_score: 0.8,
        reconstruction_method: zk_reconstruction::types::ReconstructionMethod::PatternMatching {
            pattern_id: "spl_token_transfer".to_string()
        },
        verification_proof: None,
        reconstruction_time: std::time::Duration::from_millis(5),
        cache_hint: zk_reconstruction::types::CacheHint {
            cache_key: "test_key".to_string(),
            ttl: std::time::Duration::from_secs(300),
            pattern_category: Some("spl_token".to_string()),
            reuse_probability: 0.9,
        },
    };

    match verifier.verify_reconstruction(
        &reconstructed,
        &truncated_data,
        &compression_params,
        uuid::Uuid::new_v4(),
    ).await {
        Ok(_) => info!("✅ SPL Token pattern verification PASSED with adaptive thresholds"),
        Err(e) => {
            info!("❌ SPL Token pattern verification failed: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

async fn test_state_compression_pattern() -> Result<()> {
    info!("🗜️ Testing State Compression pattern with adaptive thresholds");

    let verifier = AdaptiveVerifier::default();

    // Create state compression data (very small input)
    let compression_data = vec![0, 1, 2]; // 3 bytes

    let truncated_data = TruncatedData {
        data: compression_data.clone(),
        original_size_hint: Some(4500), // 1500x expansion hint
        truncation_point: compression_data.len(),
        metadata: TruncationMetadata {
            account: Pubkey::new_unique(),
            program_id: Pubkey::new_unique(), // Unknown program for state compression
            slot: 250_000_000,
            compression_type: CompressionType::StateCompression,
            truncation_timestamp: std::time::SystemTime::now(),
        },
    };

    let mut compression_params = CompressionParams::default();
    compression_params.compression_type = CompressionType::StateCompression;

    // Create a realistic reconstruction result for State Compression (1500x expansion within 2000.0 limit)
    let reconstructed = zk_reconstruction::types::ReconstructedAccount {
        account_data: vec![0u8; compression_data.len() * 1500], // 1500x expansion
        confidence_score: 0.9,
        reconstruction_method: zk_reconstruction::types::ReconstructionMethod::Hybrid {
            methods: vec![
                zk_reconstruction::types::ReconstructionMethod::MerkleTreeReconstruction,
                zk_reconstruction::types::ReconstructionMethod::ConstraintSolving,
            ]
        },
        verification_proof: None,
        reconstruction_time: std::time::Duration::from_millis(35),
        cache_hint: zk_reconstruction::types::CacheHint {
            cache_key: "test_key".to_string(),
            ttl: std::time::Duration::from_secs(180),
            pattern_category: Some("state_compression".to_string()),
            reuse_probability: 0.7,
        },
    };

    match verifier.verify_reconstruction(
        &reconstructed,
        &truncated_data,
        &compression_params,
        uuid::Uuid::new_v4(),
    ).await {
        Ok(_) => info!("✅ State Compression pattern verification PASSED with adaptive thresholds"),
        Err(e) => {
            info!("❌ State Compression pattern verification failed: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}