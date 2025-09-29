//! ZK Reconstruction Integration Tests
//!
//! Tests the ZK reconstruction library with realistic scenarios including
//! pattern matching, merkle tree reconstruction, and constraint solving.

use super::{TestDataGenerator, IntegrationTestConfig, run_with_timeout, TestResults};
use zk_reconstruction::{
    ZKReconstructionLibrary,
    types::{TruncatedData, CompressionParams, CompressionType, AccountMetadata},
};
use solana_sdk::pubkey::Pubkey;
use std::time::{Instant, Duration};
use tracing::{info, debug};

pub struct ZKReconstructionIntegrationTests {
    library: ZKReconstructionLibrary,
    config: IntegrationTestConfig,
}

impl ZKReconstructionIntegrationTests {
    pub fn new(config: IntegrationTestConfig) -> Self {
        Self {
            library: ZKReconstructionLibrary::new(),
            config,
        }
    }

    /// Run all ZK reconstruction integration tests
    pub async fn run_all_tests(&self) -> TestResults {
        let mut results = TestResults::default();

        info!("🚀 Starting ZK Reconstruction Integration Tests");

        // Test 1: Basic reconstruction capability
        self.run_test(&mut results, "Basic Reconstruction", || {
            Box::pin(self.test_basic_reconstruction())
        }).await;

        // Test 2: Pattern learning and reuse
        self.run_test(&mut results, "Pattern Learning", || {
            Box::pin(self.test_pattern_learning())
        }).await;

        // Test 3: Large data reconstruction
        self.run_test(&mut results, "Large Data Reconstruction", || {
            Box::pin(self.test_large_data_reconstruction())
        }).await;

        // Test 4: Multiple compression types
        self.run_test(&mut results, "Multi-Compression Types", || {
            Box::pin(self.test_multiple_compression_types())
        }).await;

        // Test 5: Cache efficiency
        self.run_test(&mut results, "Cache Efficiency", || {
            Box::pin(self.test_cache_efficiency())
        }).await;

        // Test 6: Concurrent reconstruction
        self.run_test(&mut results, "Concurrent Reconstruction", || {
            Box::pin(self.test_concurrent_reconstruction())
        }).await;

        results.print_summary();
        results
    }

    async fn run_test<F, Fut>(&self, results: &mut TestResults, name: &str, test_fn: F)
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    {
        let start = Instant::now();

        match run_with_timeout(test_fn(), self.config.test_timeout, name).await {
            Ok(_) => results.add_success(start.elapsed()),
            Err(e) => results.add_failure(e, start.elapsed()),
        }
    }

    /// Test basic reconstruction functionality
    async fn test_basic_reconstruction(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing basic reconstruction with small dataset");

        // Create test data
        let test_data = TestDataGenerator::generate_compressed_account_data(256);
        let metadata = AccountMetadata {
            account: Pubkey::new_unique(),
            program_id: Pubkey::new_unique(),
            slot: 1000,
            compression_type: CompressionType::Standard,
        };

        let truncated_data = TruncatedData {
            data: test_data,
            metadata,
        };

        let compression_params = CompressionParams {
            compression_type: CompressionType::Standard,
            merkle_tree_height: 10,
            compression_level: 6,
        };

        // Attempt reconstruction
        let result = self.library.reconstruct_compressed_account(
            &truncated_data,
            &compression_params
        ).await?;

        // Verify result properties
        if result.account_data.is_empty() {
            return Err("Reconstructed data is empty".into());
        }

        if result.confidence_score <= 0.0 || result.confidence_score > 1.0 {
            return Err("Invalid confidence score".into());
        }

        debug!("Basic reconstruction successful: {} bytes reconstructed with {:.2}% confidence",
               result.account_data.len(), result.confidence_score * 100.0);

        Ok(())
    }

    /// Test pattern learning and reuse
    async fn test_pattern_learning(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing pattern learning and reuse");

        // Create similar test data patterns
        let base_pattern = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let mut test_data1 = base_pattern.clone();
        test_data1.extend_from_slice(&TestDataGenerator::generate_compressed_account_data(100));

        let mut test_data2 = base_pattern.clone();
        test_data2.extend_from_slice(&TestDataGenerator::generate_compressed_account_data(120));

        // First reconstruction - should learn pattern
        let metadata1 = AccountMetadata {
            account: Pubkey::new_unique(),
            program_id: Pubkey::new_unique(),
            slot: 1001,
            compression_type: CompressionType::Standard,
        };

        let truncated_data1 = TruncatedData {
            data: test_data1,
            metadata: metadata1,
        };

        let compression_params = CompressionParams {
            compression_type: CompressionType::Standard,
            merkle_tree_height: 8,
            compression_level: 4,
        };

        let result1 = self.library.reconstruct_compressed_account(
            &truncated_data1,
            &compression_params
        ).await?;

        // Second reconstruction - should reuse pattern
        let metadata2 = AccountMetadata {
            account: Pubkey::new_unique(),
            program_id: metadata1.program_id, // Same program
            slot: 1002,
            compression_type: CompressionType::Standard,
        };

        let truncated_data2 = TruncatedData {
            data: test_data2,
            metadata: metadata2,
        };

        let start = Instant::now();
        let result2 = self.library.reconstruct_compressed_account(
            &truncated_data2,
            &compression_params
        ).await?;
        let second_duration = start.elapsed();

        // Second reconstruction should be faster due to pattern reuse
        if second_duration > Duration::from_millis(100) {
            return Err("Pattern reuse didn't speed up reconstruction".into());
        }

        debug!("Pattern learning successful: second reconstruction took {:?}", second_duration);

        Ok(())
    }

    /// Test reconstruction with large datasets
    async fn test_large_data_reconstruction(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing large data reconstruction");

        // Create large test dataset (10KB)
        let test_data = TestDataGenerator::generate_compressed_account_data(10 * 1024);
        let metadata = AccountMetadata {
            account: Pubkey::new_unique(),
            program_id: Pubkey::new_unique(),
            slot: 2000,
            compression_type: CompressionType::StateCompression,
        };

        let truncated_data = TruncatedData {
            data: test_data,
            metadata,
        };

        let compression_params = CompressionParams {
            compression_type: CompressionType::StateCompression,
            merkle_tree_height: 20,
            compression_level: 8,
        };

        let start = Instant::now();
        let result = self.library.reconstruct_compressed_account(
            &truncated_data,
            &compression_params
        ).await?;
        let duration = start.elapsed();

        // Should complete within reasonable time
        if duration > Duration::from_secs(5) {
            return Err("Large data reconstruction took too long".into());
        }

        debug!("Large data reconstruction successful: {} bytes in {:?}",
               result.account_data.len(), duration);

        Ok(())
    }

    /// Test multiple compression types
    async fn test_multiple_compression_types(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing multiple compression types");

        let compression_types = vec![
            CompressionType::Standard,
            CompressionType::StateCompression,
            CompressionType::Custom("test_custom".to_string()),
        ];

        for compression_type in compression_types {
            let test_data = TestDataGenerator::generate_compressed_account_data(512);
            let metadata = AccountMetadata {
                account: Pubkey::new_unique(),
                program_id: Pubkey::new_unique(),
                slot: 3000,
                compression_type: compression_type.clone(),
            };

            let truncated_data = TruncatedData {
                data: test_data,
                metadata,
            };

            let compression_params = CompressionParams {
                compression_type,
                merkle_tree_height: 15,
                compression_level: 6,
            };

            let result = self.library.reconstruct_compressed_account(
                &truncated_data,
                &compression_params
            ).await?;

            if result.account_data.is_empty() {
                return Err("Reconstruction failed for compression type".into());
            }
        }

        debug!("Multiple compression types test successful");
        Ok(())
    }

    /// Test cache efficiency
    async fn test_cache_efficiency(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing cache efficiency");

        let test_data = TestDataGenerator::generate_compressed_account_data(256);
        let metadata = AccountMetadata {
            account: Pubkey::new_unique(),
            program_id: Pubkey::new_unique(),
            slot: 4000,
            compression_type: CompressionType::Standard,
        };

        let truncated_data = TruncatedData {
            data: test_data,
            metadata,
        };

        let compression_params = CompressionParams {
            compression_type: CompressionType::Standard,
            merkle_tree_height: 10,
            compression_level: 5,
        };

        // First reconstruction - cache miss
        let start1 = Instant::now();
        let result1 = self.library.reconstruct_compressed_account(
            &truncated_data,
            &compression_params
        ).await?;
        let first_duration = start1.elapsed();

        // Second reconstruction - cache hit
        let start2 = Instant::now();
        let result2 = self.library.reconstruct_compressed_account(
            &truncated_data,
            &compression_params
        ).await?;
        let second_duration = start2.elapsed();

        // Cache hit should be significantly faster
        if second_duration >= first_duration {
            return Err("Cache doesn't appear to be working".into());
        }

        debug!("Cache efficiency test successful: first={:?}, second={:?}",
               first_duration, second_duration);

        Ok(())
    }

    /// Test concurrent reconstruction
    async fn test_concurrent_reconstruction(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing concurrent reconstruction");

        use tokio::task::JoinSet;

        let mut join_set = JoinSet::new();

        // Launch multiple concurrent reconstructions
        for i in 0..10 {
            let library = &self.library;
            let test_data = TestDataGenerator::generate_compressed_account_data(128 + i * 10);
            let metadata = AccountMetadata {
                account: Pubkey::new_unique(),
                program_id: Pubkey::new_unique(),
                slot: 5000 + i as u64,
                compression_type: CompressionType::Standard,
            };

            let truncated_data = TruncatedData {
                data: test_data,
                metadata,
            };

            let compression_params = CompressionParams {
                compression_type: CompressionType::Standard,
                merkle_tree_height: 8,
                compression_level: 4,
            };

            join_set.spawn(async move {
                library.reconstruct_compressed_account(&truncated_data, &compression_params).await
            });
        }

        // Wait for all to complete
        let mut results = Vec::new();
        while let Some(result) = join_set.join_next().await {
            match result? {
                Ok(reconstruction) => results.push(reconstruction),
                Err(e) => return Err(format!("Concurrent reconstruction failed: {}", e).into()),
            }
        }

        if results.len() != 10 {
            return Err("Not all concurrent reconstructions completed".into());
        }

        debug!("Concurrent reconstruction test successful: {} reconstructions completed", results.len());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integration::init_test_environment;

    #[tokio::test]
    async fn test_zk_reconstruction_integration() {
        init_test_environment();

        let config = IntegrationTestConfig::default();
        let test_suite = ZKReconstructionIntegrationTests::new(config);

        let results = test_suite.run_all_tests().await;

        // Ensure at least 80% success rate
        assert!(results.success_rate() >= 0.8,
                "ZK reconstruction integration tests failed with {:.1}% success rate",
                results.success_rate() * 100.0);
    }
}