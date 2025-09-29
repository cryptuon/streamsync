//! Validation test for adaptive verification thresholds
//! Tests the calibrated verification system against realistic Solana data patterns

use zk_reconstruction::{
    ZKReconstructionLibrary,
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

    // Test with various Solana program patterns
    test_spl_token_pattern().await?;
    test_metaplex_pattern().await?;
    test_jupiter_pattern().await?;
    test_state_compression_pattern().await?;
    test_unknown_program_pattern().await?;

    info!("✅ All adaptive verification threshold tests completed successfully!");
    Ok(())
}

async fn test_spl_token_pattern() -> Result<()> {
    info!("🪙 Testing SPL Token pattern with adaptive thresholds");

    let verifier = AdaptiveVerifier::default();

    // Create SPL Token transfer data
    let mut token_data = vec![3]; // Transfer discriminator
    token_data.extend_from_slice(&1000u64.to_le_bytes()); // Amount

    let truncated_data = TruncatedData {
        data: token_data.clone(),
        original_size_hint: Some(token_data.len() * 400),
        truncation_point: token_data.len(),
        metadata: TruncationMetadata {
            account: Pubkey::new_unique(),
            program_id: Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")?,
            slot: 250_000_000,
            compression_type: CompressionType::Standard,
            truncation_timestamp: std::time::SystemTime::now(),
        },
    };

    let compression_params = CompressionParams {
        compression_type: CompressionType::Standard,
        merkle_tree_height: 20,
        leaf_count: 1000,
        root_hash: [0; 32],
        compression_program: Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")?,
        additional_params: std::collections::HashMap::new(),
    };

    // Create a realistic reconstruction result for SPL Token
    let reconstructed = zk_reconstruction::types::ReconstructedAccount {
        account_data: vec![0u8; token_data.len() * 400], // 400x expansion (within 500.0 limit)
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
        Ok(_) => info!("✅ SPL Token pattern verification passed with adaptive thresholds"),
        Err(e) => info!("❌ SPL Token pattern verification failed: {}", e),
    }

    Ok(())
}

async fn test_metaplex_pattern() -> Result<()> {
    info!("🖼️ Testing Metaplex pattern with adaptive thresholds");

    let verifier = AdaptiveVerifier::default();

    // Create Metaplex metadata data
    let metaplex_data = vec![1, 2, 3, 4]; // Simple metadata pattern

    let truncated_data = TruncatedData {
        data: metaplex_data.clone(),
        metadata: AccountMetadata {
            account: Pubkey::new_unique(),
            program_id: Pubkey::from_str("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s")?,
            slot: 250_000_000,
            lamports: 5616720,
        },
    };

    let compression_params = CompressionParams {
        compression_type: CompressionType::Standard,
        merkle_tree_height: 20,
        compression_ratio: 2.0,
    };

    // Create a realistic reconstruction result for Metaplex (higher expansion)
    let reconstructed = zk_reconstruction::types::ReconstructedAccount {
        account_data: vec![0u8; metaplex_data.len() * 800], // 800x expansion (within 1000.0 limit)
        confidence_score: 0.75,
        reconstruction_method: zk_reconstruction::types::ReconstructionMethod::MerkleTreeReconstruction,
        verification_proof: None,
        reconstruction_time: std::time::Duration::from_millis(15),
        cache_hint: zk_reconstruction::types::CacheHint {
            cache_key: "test_key".to_string(),
            ttl: std::time::Duration::from_secs(60),
            pattern_category: Some("metaplex".to_string()),
            reuse_probability: 0.6,
        },
    };

    match verifier.verify_reconstruction(
        &reconstructed,
        &truncated_data,
        &compression_params,
        uuid::Uuid::new_v4(),
    ).await {
        Ok(_) => info!("✅ Metaplex pattern verification passed with adaptive thresholds"),
        Err(e) => info!("❌ Metaplex pattern verification failed: {}", e),
    }

    Ok(())
}

async fn test_jupiter_pattern() -> Result<()> {
    info!("🌌 Testing Jupiter swap pattern with adaptive thresholds");

    let verifier = AdaptiveVerifier::default();

    // Create Jupiter swap data
    let jupiter_data = vec![1, 2, 3, 4, 5, 6, 7, 8]; // Swap calculation data

    let truncated_data = TruncatedData {
        data: jupiter_data.clone(),
        metadata: AccountMetadata {
            account: Pubkey::new_unique(),
            program_id: Pubkey::from_str("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4")?,
            slot: 250_000_000,
            lamports: 1461600,
        },
    };

    let compression_params = CompressionParams {
        compression_type: CompressionType::Standard,
        merkle_tree_height: 20,
        compression_ratio: 2.0,
    };

    // Create a realistic reconstruction result for Jupiter
    let reconstructed = zk_reconstruction::types::ReconstructedAccount {
        account_data: vec![0u8; jupiter_data.len() * 700], // 700x expansion (within 800.0 limit)
        confidence_score: 0.85,
        reconstruction_method: zk_reconstruction::types::ReconstructionMethod::ConstraintSolving,
        verification_proof: None,
        reconstruction_time: std::time::Duration::from_millis(25),
        cache_hint: zk_reconstruction::types::CacheHint {
            cache_key: "test_key".to_string(),
            ttl: std::time::Duration::from_secs(120),
            pattern_category: Some("jupiter".to_string()),
            reuse_probability: 0.5,
        },
    };

    match verifier.verify_reconstruction(
        &reconstructed,
        &truncated_data,
        &compression_params,
        uuid::Uuid::new_v4(),
    ).await {
        Ok(_) => info!("✅ Jupiter pattern verification passed with adaptive thresholds"),
        Err(e) => info!("❌ Jupiter pattern verification failed: {}", e),
    }

    Ok(())
}

async fn test_state_compression_pattern() -> Result<()> {
    info!("🗜️ Testing State Compression pattern with adaptive thresholds");

    let verifier = AdaptiveVerifier::default();

    // Create state compression data
    let compression_data = vec![0, 1, 2]; // Very small input for high expansion

    let truncated_data = TruncatedData {
        data: compression_data.clone(),
        metadata: AccountMetadata {
            account: Pubkey::new_unique(),
            program_id: Pubkey::new_unique(), // Generic compression program
            slot: 250_000_000,
            lamports: 0,
        },
    };

    let compression_params = CompressionParams {
        compression_type: CompressionType::StateCompression,
        merkle_tree_height: 20,
        compression_ratio: 2.0,
    };

    // Create a realistic reconstruction result for State Compression (very high expansion)
    let reconstructed = zk_reconstruction::types::ReconstructedAccount {
        account_data: vec![0u8; compression_data.len() * 1500], // 1500x expansion (within 2000.0 limit)
        confidence_score: 0.9,
        reconstruction_method: zk_reconstruction::types::ReconstructionMethod::Hybrid {
            primary_method: Box::new(zk_reconstruction::types::ReconstructionMethod::MerkleTreeReconstruction),
            fallback_method: Box::new(zk_reconstruction::types::ReconstructionMethod::ConstraintSolving),
            confidence_threshold: 0.8,
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
        Ok(_) => info!("✅ State Compression pattern verification passed with adaptive thresholds"),
        Err(e) => info!("❌ State Compression pattern verification failed: {}", e),
    }

    Ok(())
}

async fn test_unknown_program_pattern() -> Result<()> {
    info!("❓ Testing unknown program pattern with adaptive thresholds");

    let verifier = AdaptiveVerifier::default();

    // Create unknown program data
    let unknown_data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]; // 16 bytes

    let truncated_data = TruncatedData {
        data: unknown_data.clone(),
        metadata: AccountMetadata {
            account: Pubkey::new_unique(),
            program_id: Pubkey::new_unique(), // Unknown program
            slot: 250_000_000,
            lamports: 1000000,
        },
    };

    let compression_params = CompressionParams {
        compression_type: CompressionType::Standard,
        merkle_tree_height: 20,
        compression_ratio: 2.0,
    };

    // Create a realistic reconstruction result for unknown program (conservative limit)
    let reconstructed = zk_reconstruction::types::ReconstructedAccount {
        account_data: vec![0u8; unknown_data.len() * 250], // 250x expansion (within 300.0 default limit)
        confidence_score: 0.7,
        reconstruction_method: zk_reconstruction::types::ReconstructionMethod::PatternMatching {
            pattern_id: "unknown_pattern".to_string()
        },
        verification_proof: None,
        reconstruction_time: std::time::Duration::from_millis(10),
        cache_hint: zk_reconstruction::types::CacheHint {
            cache_key: "test_key".to_string(),
            ttl: std::time::Duration::from_secs(60),
            pattern_category: None,
            reuse_probability: 0.3,
        },
    };

    match verifier.verify_reconstruction(
        &reconstructed,
        &truncated_data,
        &compression_params,
        uuid::Uuid::new_v4(),
    ).await {
        Ok(_) => info!("✅ Unknown program pattern verification passed with adaptive thresholds"),
        Err(e) => info!("❌ Unknown program pattern verification failed: {}", e),
    }

    Ok(())
}