//! Quick Pipeline Test - Test complete ZK reconstruction with known working patterns

use zk_reconstruction::{
    ZKReconstructionLibrary,
    types::{TruncatedData, CompressionParams, TruncationMetadata, CompressionType},
};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::time::SystemTime;
use tracing::info;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    info!("🚀 Quick ZK Reconstruction Pipeline Test");

    let reconstructor = ZKReconstructionLibrary::new();

    // Test 1: Small SPL Token pattern (should use fast pattern matching)
    test_spl_token_small(&reconstructor).await?;

    // Test 2: Metaplex pattern with medium complexity
    test_metaplex_pattern(&reconstructor).await?;

    info!("🎉 Quick pipeline tests completed!");
    Ok(())
}

async fn test_spl_token_small(reconstructor: &ZKReconstructionLibrary) -> Result<()> {
    info!("🪙 Testing SPL Token (small pattern - should trigger fast path)");

    // Create minimal SPL Token data that should trigger pattern matching
    let token_data = vec![3, 232, 3, 0, 0, 0, 0, 0, 0]; // 9 bytes: Transfer + amount

    let truncated_data = TruncatedData {
        data: token_data.clone(),
        original_size_hint: Some(165), // Small expansion: 165 bytes
        truncation_point: token_data.len(),
        metadata: TruncationMetadata {
            account: Pubkey::new_unique(),
            program_id: Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")?,
            slot: 250_000_000,
            compression_type: CompressionType::Standard,
            truncation_timestamp: SystemTime::now(),
        },
    };

    let compression_params = CompressionParams::default();

    info!("   📊 Input: {} bytes", token_data.len());
    info!("   🎯 Expected: Pattern matching with ~18x expansion");

    match reconstructor.reconstruct_compressed_account(&truncated_data, &compression_params).await {
        Ok(result) => {
            let expansion = result.account_data.len() as f64 / token_data.len() as f64;
            info!("   ✅ SUCCESS: {} bytes reconstructed ({:.1}x expansion)", result.account_data.len(), expansion);
            info!("   🎯 Confidence: {:.3}", result.confidence_score);
            info!("   🔧 Method: {:?}", result.reconstruction_method);
            info!("   ⏱️ Time: {:?}", result.reconstruction_time);
        }
        Err(e) => {
            info!("   ❌ Failed: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

async fn test_metaplex_pattern(reconstructor: &ZKReconstructionLibrary) -> Result<()> {
    info!("🖼️ Testing Metaplex pattern (medium complexity)");

    // Create Metaplex-style data (slightly larger)
    let metaplex_data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]; // 16 bytes

    let truncated_data = TruncatedData {
        data: metaplex_data.clone(),
        original_size_hint: Some(800), // 50x expansion
        truncation_point: metaplex_data.len(),
        metadata: TruncationMetadata {
            account: Pubkey::new_unique(),
            program_id: Pubkey::from_str("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s")?,
            slot: 250_000_000,
            compression_type: CompressionType::Standard,
            truncation_timestamp: SystemTime::now(),
        },
    };

    let compression_params = CompressionParams::default();

    info!("   📊 Input: {} bytes", metaplex_data.len());
    info!("   🎯 Expected: Metaplex reconstruction with ~50x expansion");

    match reconstructor.reconstruct_compressed_account(&truncated_data, &compression_params).await {
        Ok(result) => {
            let expansion = result.account_data.len() as f64 / metaplex_data.len() as f64;
            info!("   ✅ SUCCESS: {} bytes reconstructed ({:.1}x expansion)", result.account_data.len(), expansion);
            info!("   🎯 Confidence: {:.3}", result.confidence_score);
            info!("   🔧 Method: {:?}", result.reconstruction_method);
            info!("   ⏱️ Time: {:?}", result.reconstruction_time);
        }
        Err(e) => {
            info!("   ❌ Failed: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}