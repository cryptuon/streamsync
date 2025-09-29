//! Cross-Library Integration Tests
//!
//! Tests the integration between ZK reconstruction, IDL sync, and distributed DuckDB
//! libraries working together in realistic end-to-end scenarios.

use super::{TestDataGenerator, IntegrationTestConfig, run_with_timeout, TestResults};
use zk_reconstruction::{ZKReconstructionLibrary, types::{TruncatedData, CompressionParams, CompressionType, AccountMetadata}};
use idl_sync::{IDLSyncLibrary, types::IDLAnalysisConfig};
use distributed_duckdb::{DistributedCoordinator, Query};
use solana_sdk::pubkey::Pubkey;
use std::time::{Instant, Duration};
use tracing::{info, debug};

pub struct CrossLibraryIntegrationTests {
    zk_reconstruction: ZKReconstructionLibrary,
    idl_sync: IDLSyncLibrary,
    duckdb_coordinator: DistributedCoordinator,
    config: IntegrationTestConfig,
}

impl CrossLibraryIntegrationTests {
    pub fn new(config: IntegrationTestConfig) -> Self {
        Self {
            zk_reconstruction: ZKReconstructionLibrary::new(),
            idl_sync: IDLSyncLibrary::new(IDLAnalysisConfig::default()),
            duckdb_coordinator: DistributedCoordinator::new(),
            config,
        }
    }

    /// Run all cross-library integration tests
    pub async fn run_all_tests(&self) -> TestResults {
        let mut results = TestResults::default();

        info!("🚀 Starting Cross-Library Integration Tests");

        // Test 1: End-to-end data pipeline
        self.run_test(&mut results, "End-to-End Pipeline", || {
            Box::pin(self.test_end_to_end_pipeline())
        }).await;

        // Test 2: Reconstruction + IDL analysis
        self.run_test(&mut results, "Reconstruction + IDL Analysis", || {
            Box::pin(self.test_reconstruction_idl_analysis())
        }).await;

        // Test 3: IDL sync + analytical queries
        self.run_test(&mut results, "IDL Sync + Analytics", || {
            Box::pin(self.test_idl_sync_analytics())
        }).await;

        // Test 4: Performance optimization pipeline
        self.run_test(&mut results, "Performance Optimization", || {
            Box::pin(self.test_performance_optimization_pipeline())
        }).await;

        // Test 5: Real-time analysis workflow
        self.run_test(&mut results, "Real-time Analysis", || {
            Box::pin(self.test_realtime_analysis_workflow())
        }).await;

        // Test 6: Data consistency across libraries
        self.run_test(&mut results, "Data Consistency", || {
            Box::pin(self.test_data_consistency())
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

    /// Test complete end-to-end data pipeline
    async fn test_end_to_end_pipeline(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing end-to-end data pipeline");

        let program_id = Pubkey::new_unique();

        // Step 1: Generate and reconstruct compressed account data
        let compressed_data = TestDataGenerator::generate_compressed_account_data(512);
        let metadata = AccountMetadata {
            account: Pubkey::new_unique(),
            program_id,
            slot: 1000,
            compression_type: CompressionType::StateCompression,
        };

        let truncated_data = TruncatedData {
            data: compressed_data,
            metadata,
        };

        let compression_params = CompressionParams {
            compression_type: CompressionType::StateCompression,
            merkle_tree_height: 15,
            compression_level: 6,
        };

        let reconstructed = self.zk_reconstruction.reconstruct_compressed_account(
            &truncated_data,
            &compression_params
        ).await?;

        debug!("Step 1 complete: Reconstructed {} bytes", reconstructed.account_data.len());

        // Step 2: Generate transaction history and analyze for IDL
        let transaction_history = (0..50)
            .map(|i| {
                let mut tx_data = TestDataGenerator::generate_transaction_data();
                tx_data[64] = (i % 3) as u8; // Vary instruction types
                tx_data
            })
            .collect::<Vec<_>>();

        let generated_idl = self.idl_sync.analyze_program_transactions(
            &program_id,
            &transaction_history
        ).await?;

        debug!("Step 2 complete: Generated IDL with {} instructions", generated_idl.idl.instructions.len());

        // Step 3: Store and query the data using DuckDB
        let analytical_query = Query {
            sql: format!(
                "SELECT
                    '{}' as program_id,
                    {} as account_size,
                    {} as instruction_count,
                    {:.2} as reconstruction_confidence,
                    {:.2} as idl_confidence
                ",
                program_id,
                reconstructed.account_data.len(),
                generated_idl.idl.instructions.len(),
                reconstructed.confidence_score,
                generated_idl.confidence.overall_confidence
            ),
        };

        let query_result = self.duckdb_coordinator.execute_query(analytical_query).await?;

        debug!("Step 3 complete: Analytical query returned {} rows", query_result.rows.len());

        // Verify end-to-end consistency
        if query_result.rows.is_empty() {
            return Err("End-to-end pipeline produced no analytical results".into());
        }

        if reconstructed.confidence_score < 0.5 {
            return Err("Reconstruction confidence too low for pipeline".into());
        }

        if generated_idl.confidence.overall_confidence < 0.5 {
            return Err("IDL analysis confidence too low for pipeline".into());
        }

        debug!("End-to-end pipeline successful: reconstruction={:.2}%, IDL={:.2}%",
               reconstructed.confidence_score * 100.0,
               generated_idl.confidence.overall_confidence * 100.0);

        Ok(())
    }

    /// Test reconstruction feeding into IDL analysis
    async fn test_reconstruction_idl_analysis(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing reconstruction + IDL analysis integration");

        let program_id = Pubkey::new_unique();

        // Create multiple compressed accounts for the same program
        let mut reconstructed_accounts = Vec::new();

        for i in 0..5 {
            let compressed_data = TestDataGenerator::generate_compressed_account_data(256 + i * 50);
            let metadata = AccountMetadata {
                account: Pubkey::new_unique(),
                program_id,
                slot: 1000 + i as u64,
                compression_type: CompressionType::Standard,
            };

            let truncated_data = TruncatedData {
                data: compressed_data,
                metadata,
            };

            let compression_params = CompressionParams {
                compression_type: CompressionType::Standard,
                merkle_tree_height: 12,
                compression_level: 5,
            };

            let reconstructed = self.zk_reconstruction.reconstruct_compressed_account(
                &truncated_data,
                &compression_params
            ).await?;

            reconstructed_accounts.push(reconstructed);
        }

        // Extract patterns from reconstructed data for IDL analysis
        let mut synthetic_transactions = Vec::new();
        for (i, account) in reconstructed_accounts.iter().enumerate() {
            // Create synthetic transaction data based on reconstructed account structure
            let mut tx_data = TestDataGenerator::generate_transaction_data();

            // Use account data patterns to inform instruction structure
            if account.account_data.len() > 100 {
                tx_data[64] = 0x01; // "Large account" instruction
                tx_data[68] = (account.account_data.len() / 32) as u8; // Size indicator
            } else {
                tx_data[64] = 0x02; // "Small account" instruction
                tx_data[68] = account.account_data.len() as u8; // Direct size
            }

            synthetic_transactions.push(tx_data);
        }

        // Analyze the patterns
        let generated_idl = self.idl_sync.analyze_program_transactions(
            &program_id,
            &synthetic_transactions
        ).await?;

        // Verify integration worked
        if generated_idl.idl.instructions.is_empty() {
            return Err("IDL analysis didn't detect patterns from reconstructed data".into());
        }

        if generated_idl.idl.accounts.is_empty() {
            return Err("IDL analysis didn't infer account structures".into());
        }

        // Should detect different instruction types based on account sizes
        if generated_idl.idl.instructions.len() < 2 {
            return Err("IDL analysis didn't detect expected instruction variety".into());
        }

        debug!("Reconstruction + IDL analysis successful: {} accounts → {} instructions",
               reconstructed_accounts.len(), generated_idl.idl.instructions.len());

        Ok(())
    }

    /// Test IDL sync with analytical queries
    async fn test_idl_sync_analytics(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing IDL sync + analytics integration");

        let program_id = Pubkey::new_unique();

        // Generate transaction history
        let transaction_history = (0..100)
            .map(|i| {
                let mut tx_data = TestDataGenerator::generate_transaction_data();
                tx_data[64] = (i % 4) as u8; // 4 different instruction types
                tx_data
            })
            .collect::<Vec<_>>();

        // Analyze for IDL
        let generated_idl = self.idl_sync.analyze_program_transactions(
            &program_id,
            &transaction_history
        ).await?;

        // Create analytical query based on IDL insights
        let idl_analytics_query = Query {
            sql: format!(
                "SELECT
                    '{}' as program_id,
                    {} as total_instructions,
                    {} as total_accounts,
                    {:.3} as avg_confidence,
                    CASE
                        WHEN {} > 3 THEN 'Complex'
                        WHEN {} > 1 THEN 'Medium'
                        ELSE 'Simple'
                    END as complexity_category
                ",
                program_id,
                generated_idl.idl.instructions.len(),
                generated_idl.idl.accounts.len(),
                generated_idl.confidence.overall_confidence,
                generated_idl.idl.instructions.len(),
                generated_idl.idl.instructions.len()
            ),
        };

        let analytics_result = self.duckdb_coordinator.execute_query(idl_analytics_query).await?;

        // Verify analytics worked
        if analytics_result.rows.is_empty() {
            return Err("IDL analytics query returned no results".into());
        }

        if analytics_result.column_names.len() != 5 {
            return Err("IDL analytics query didn't return expected columns".into());
        }

        // Create program comparison query
        let comparison_query = Query {
            sql: "SELECT
                    complexity_category,
                    COUNT(*) as program_count,
                    AVG(total_instructions) as avg_instructions,
                    AVG(avg_confidence) as avg_confidence
                  FROM program_analysis
                  GROUP BY complexity_category
                  ORDER BY avg_instructions DESC".to_string(),
        };

        let comparison_result = self.duckdb_coordinator.execute_query(comparison_query).await?;

        debug!("IDL sync + analytics successful: {} IDL instructions → {} analytical results",
               generated_idl.idl.instructions.len(), analytics_result.rows.len());

        Ok(())
    }

    /// Test performance optimization pipeline
    async fn test_performance_optimization_pipeline(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing performance optimization pipeline");

        let program_id = Pubkey::new_unique();

        // Measure baseline performance without optimization
        let start_baseline = Instant::now();

        // Step 1: Reconstruct without pattern cache
        let compressed_data = TestDataGenerator::generate_compressed_account_data(1024);
        let metadata = AccountMetadata {
            account: Pubkey::new_unique(),
            program_id,
            slot: 2000,
            compression_type: CompressionType::Standard,
        };

        let truncated_data = TruncatedData {
            data: compressed_data.clone(),
            metadata: metadata.clone(),
        };

        let compression_params = CompressionParams {
            compression_type: CompressionType::Standard,
            merkle_tree_height: 10,
            compression_level: 5,
        };

        let first_reconstruction = self.zk_reconstruction.reconstruct_compressed_account(
            &truncated_data,
            &compression_params
        ).await?;

        let baseline_time = start_baseline.elapsed();

        // Step 2: Use fast path for similar data
        let start_optimized = Instant::now();

        let similar_data = TruncatedData {
            data: compressed_data, // Same data should hit pattern cache
            metadata,
        };

        let second_reconstruction = self.zk_reconstruction.reconstruct_compressed_account(
            &similar_data,
            &compression_params
        ).await?;

        let optimized_time = start_optimized.elapsed();

        // Step 3: Use IDL cache for repeated analysis
        let transaction_history = (0..30)
            .map(|_| TestDataGenerator::generate_transaction_data())
            .collect::<Vec<_>>();

        let start_idl_baseline = Instant::now();
        let first_idl = self.idl_sync.analyze_program_transactions(
            &program_id,
            &transaction_history
        ).await?;
        let idl_baseline_time = start_idl_baseline.elapsed();

        let start_idl_cached = Instant::now();
        let second_idl = self.idl_sync.analyze_program_transactions(
            &program_id,
            &transaction_history
        ).await?;
        let idl_cached_time = start_idl_cached.elapsed();

        // Step 4: Analytics performance
        let query = Query {
            sql: "SELECT COUNT(*) FROM transactions WHERE program_id = 'test'".to_string(),
        };

        let start_query = Instant::now();
        let _query_result = self.duckdb_coordinator.execute_query(query).await?;
        let query_time = start_query.elapsed();

        // Verify optimizations worked
        if optimized_time >= baseline_time {
            return Err("Pattern caching didn't improve reconstruction performance".into());
        }

        if idl_cached_time >= idl_baseline_time {
            return Err("IDL caching didn't improve analysis performance".into());
        }

        // All operations should be reasonably fast
        if query_time > Duration::from_millis(500) {
            return Err("Analytical query performance too slow".into());
        }

        let speedup_factor = baseline_time.as_nanos() / optimized_time.as_nanos().max(1);
        let idl_speedup = idl_baseline_time.as_nanos() / idl_cached_time.as_nanos().max(1);

        debug!("Performance optimization successful: reconstruction {}x faster, IDL {}x faster",
               speedup_factor, idl_speedup);

        Ok(())
    }

    /// Test real-time analysis workflow
    async fn test_realtime_analysis_workflow(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing real-time analysis workflow");

        let program_id = Pubkey::new_unique();

        // Start real-time monitoring
        self.idl_sync.start_monitoring(&program_id).await?;

        // Simulate real-time data stream
        let mut processed_count = 0;
        for batch in 0..5 {
            // Generate new compressed accounts
            for i in 0..3 {
                let compressed_data = TestDataGenerator::generate_compressed_account_data(200 + i * 50);
                let metadata = AccountMetadata {
                    account: Pubkey::new_unique(),
                    program_id,
                    slot: 3000 + (batch * 3 + i) as u64,
                    compression_type: CompressionType::Standard,
                };

                let truncated_data = TruncatedData {
                    data: compressed_data,
                    metadata,
                };

                let compression_params = CompressionParams {
                    compression_type: CompressionType::Standard,
                    merkle_tree_height: 8,
                    compression_level: 4,
                };

                // Reconstruct in real-time
                let reconstructed = self.zk_reconstruction.fast_reconstruct_common_patterns(
                    &truncated_data
                ).await;

                if let Some(result) = reconstructed {
                    // Feed reconstruction results to IDL analysis
                    let synthetic_tx = TestDataGenerator::generate_transaction_data();
                    self.idl_sync.process_new_transaction(&program_id, &synthetic_tx).await?;
                    processed_count += 1;
                }
            }

            // Generate analytical insights
            let progress_query = Query {
                sql: format!(
                    "SELECT
                        {} as batch_number,
                        {} as processed_accounts,
                        CURRENT_TIMESTAMP as analysis_time
                    ",
                    batch, processed_count
                ),
            };

            let _progress_result = self.duckdb_coordinator.execute_query(progress_query).await?;

            // Small delay to simulate real-time processing
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Get final IDL state
        let final_idl = self.idl_sync.get_current_idl(&program_id).await?;

        // Stop monitoring
        self.idl_sync.stop_monitoring(&program_id).await?;

        // Verify real-time processing worked
        if processed_count == 0 {
            return Err("No accounts processed in real-time workflow".into());
        }

        if final_idl.idl.instructions.is_empty() {
            return Err("Real-time IDL analysis produced no instructions".into());
        }

        debug!("Real-time analysis workflow successful: {} accounts processed, {} instructions detected",
               processed_count, final_idl.idl.instructions.len());

        Ok(())
    }

    /// Test data consistency across libraries
    async fn test_data_consistency(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing data consistency across libraries");

        let program_id = Pubkey::new_unique();

        // Create test data that will flow through all systems
        let test_account_data = TestDataGenerator::generate_compressed_account_data(512);
        let test_transactions = (0..20)
            .map(|_| TestDataGenerator::generate_transaction_data())
            .collect::<Vec<_>>();

        // Process through ZK reconstruction
        let metadata = AccountMetadata {
            account: Pubkey::new_unique(),
            program_id,
            slot: 4000,
            compression_type: CompressionType::Standard,
        };

        let truncated_data = TruncatedData {
            data: test_account_data.clone(),
            metadata,
        };

        let compression_params = CompressionParams {
            compression_type: CompressionType::Standard,
            merkle_tree_height: 10,
            compression_level: 5,
        };

        let reconstructed = self.zk_reconstruction.reconstruct_compressed_account(
            &truncated_data,
            &compression_params
        ).await?;

        // Process through IDL sync
        let generated_idl = self.idl_sync.analyze_program_transactions(
            &program_id,
            &test_transactions
        ).await?;

        // Store and verify in DuckDB
        let consistency_query = Query {
            sql: format!(
                "SELECT
                    '{}' as program_id,
                    '{}' as account_hash,
                    {} as original_size,
                    {} as reconstructed_size,
                    {} as instruction_count,
                    CASE
                        WHEN {} = {} THEN 'CONSISTENT'
                        ELSE 'INCONSISTENT'
                    END as size_consistency
                ",
                program_id,
                blake3::hash(&test_account_data).to_hex(),
                test_account_data.len(),
                reconstructed.account_data.len(),
                generated_idl.idl.instructions.len(),
                test_account_data.len(),
                truncated_data.data.len()
            ),
        };

        let consistency_result = self.duckdb_coordinator.execute_query(consistency_query).await?;

        // Verify consistency
        if consistency_result.rows.is_empty() {
            return Err("Consistency query returned no results".into());
        }

        // Check that data flows correctly between systems
        if reconstructed.account_data.is_empty() {
            return Err("Data consistency: reconstruction lost data".into());
        }

        if generated_idl.idl.instructions.is_empty() {
            return Err("Data consistency: IDL analysis lost patterns".into());
        }

        debug!("Data consistency successful: {} bytes → {} bytes, {} instructions",
               test_account_data.len(),
               reconstructed.account_data.len(),
               generated_idl.idl.instructions.len());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integration::init_test_environment;

    #[tokio::test]
    async fn test_cross_library_integration() {
        init_test_environment();

        let config = IntegrationTestConfig::default();
        let test_suite = CrossLibraryIntegrationTests::new(config);

        let results = test_suite.run_all_tests().await;

        // Cross-library integration should have high success rate
        assert!(results.success_rate() >= 0.8,
                "Cross-library integration tests failed with {:.1}% success rate",
                results.success_rate() * 100.0);
    }
}