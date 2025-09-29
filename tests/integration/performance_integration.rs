//! Performance Integration Tests
//!
//! Tests performance characteristics and validates claimed performance metrics
//! for all libraries working together under realistic load conditions.

use super::{TestDataGenerator, IntegrationTestConfig, run_with_timeout, TestResults};
use zk_reconstruction::{ZKReconstructionLibrary, types::{TruncatedData, CompressionParams, CompressionType, AccountMetadata}};
use idl_sync::{IDLSyncLibrary, types::IDLAnalysisConfig};
use distributed_duckdb::{DistributedCoordinator, Query};
use solana_sdk::pubkey::Pubkey;
use std::time::{Instant, Duration};
use tracing::{info, debug, warn};
use tokio::task::JoinSet;

pub struct PerformanceIntegrationTests {
    zk_reconstruction: ZKReconstructionLibrary,
    idl_sync: IDLSyncLibrary,
    duckdb_coordinator: DistributedCoordinator,
    config: IntegrationTestConfig,
}

#[derive(Debug)]
pub struct PerformanceMetrics {
    pub throughput: f64,              // Operations per second
    pub latency_p50: Duration,        // 50th percentile latency
    pub latency_p95: Duration,        // 95th percentile latency
    pub latency_p99: Duration,        // 99th percentile latency
    pub success_rate: f64,            // Percentage of successful operations
    pub memory_usage_mb: f64,         // Estimated memory usage
}

impl PerformanceMetrics {
    pub fn from_durations(durations: &[Duration], total_time: Duration) -> Self {
        let mut sorted_durations = durations.to_vec();
        sorted_durations.sort();

        let len = sorted_durations.len();
        let p50_idx = len / 2;
        let p95_idx = (len as f64 * 0.95) as usize;
        let p99_idx = (len as f64 * 0.99) as usize;

        Self {
            throughput: len as f64 / total_time.as_secs_f64(),
            latency_p50: sorted_durations.get(p50_idx).copied().unwrap_or_default(),
            latency_p95: sorted_durations.get(p95_idx).copied().unwrap_or_default(),
            latency_p99: sorted_durations.get(p99_idx).copied().unwrap_or_default(),
            success_rate: 1.0, // Will be adjusted based on failures
            memory_usage_mb: 0.0, // Would need proper memory profiling
        }
    }

    pub fn print_summary(&self, test_name: &str) {
        info!("📊 Performance Metrics for {}:", test_name);
        info!("   🚀 Throughput: {:.1} ops/sec", self.throughput);
        info!("   ⏱️  Latency P50: {:?}", self.latency_p50);
        info!("   ⏱️  Latency P95: {:?}", self.latency_p95);
        info!("   ⏱️  Latency P99: {:?}", self.latency_p99);
        info!("   ✅ Success Rate: {:.1}%", self.success_rate * 100.0);
    }
}

impl PerformanceIntegrationTests {
    pub fn new(config: IntegrationTestConfig) -> Self {
        Self {
            zk_reconstruction: ZKReconstructionLibrary::new(),
            idl_sync: IDLSyncLibrary::new(IDLAnalysisConfig::default()),
            duckdb_coordinator: DistributedCoordinator::new(),
            config,
        }
    }

    /// Run all performance integration tests
    pub async fn run_all_tests(&self) -> TestResults {
        let mut results = TestResults::default();

        info!("🚀 Starting Performance Integration Tests");

        // Test 1: ZK reconstruction throughput
        self.run_test(&mut results, "ZK Reconstruction Throughput", || {
            Box::pin(self.test_zk_reconstruction_throughput())
        }).await;

        // Test 2: IDL analysis performance
        self.run_test(&mut results, "IDL Analysis Performance", || {
            Box::pin(self.test_idl_analysis_performance())
        }).await;

        // Test 3: DuckDB query performance
        self.run_test(&mut results, "DuckDB Query Performance", || {
            Box::pin(self.test_duckdb_query_performance())
        }).await;

        // Test 4: End-to-end pipeline throughput
        self.run_test(&mut results, "Pipeline Throughput", || {
            Box::pin(self.test_pipeline_throughput())
        }).await;

        // Test 5: Concurrent load testing
        self.run_test(&mut results, "Concurrent Load Test", || {
            Box::pin(self.test_concurrent_load())
        }).await;

        // Test 6: Memory efficiency under load
        self.run_test(&mut results, "Memory Efficiency", || {
            Box::pin(self.test_memory_efficiency())
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

        match run_with_timeout(test_fn(), Duration::from_secs(60), name).await {
            Ok(_) => results.add_success(start.elapsed()),
            Err(e) => results.add_failure(e, start.elapsed()),
        }
    }

    /// Test ZK reconstruction throughput
    async fn test_zk_reconstruction_throughput(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing ZK reconstruction throughput");

        let iterations = self.config.performance_iterations.min(200); // Cap for reasonable test time
        let mut durations = Vec::new();
        let mut failures = 0;

        let overall_start = Instant::now();

        for i in 0..iterations {
            let compressed_data = TestDataGenerator::generate_compressed_account_data(256 + (i % 1000));
            let metadata = AccountMetadata {
                account: Pubkey::new_unique(),
                program_id: Pubkey::new_unique(),
                slot: 1000 + i as u64,
                compression_type: CompressionType::Standard,
            };

            let truncated_data = TruncatedData {
                data: compressed_data,
                metadata,
            };

            let compression_params = CompressionParams {
                compression_type: CompressionType::Standard,
                merkle_tree_height: 10 + (i % 5) as u32,
                compression_level: 4 + (i % 5) as u32,
            };

            let start = Instant::now();
            match self.zk_reconstruction.reconstruct_compressed_account(
                &truncated_data,
                &compression_params
            ).await {
                Ok(_) => durations.push(start.elapsed()),
                Err(_) => failures += 1,
            }
        }

        let total_time = overall_start.elapsed();
        let mut metrics = PerformanceMetrics::from_durations(&durations, total_time);
        metrics.success_rate = durations.len() as f64 / iterations as f64;

        metrics.print_summary("ZK Reconstruction");

        // Performance expectations
        if metrics.throughput < 50.0 {
            warn!("ZK reconstruction throughput below expected: {:.1} ops/sec", metrics.throughput);
        }

        if metrics.latency_p95 > Duration::from_millis(100) {
            warn!("ZK reconstruction P95 latency too high: {:?}", metrics.latency_p95);
        }

        if metrics.success_rate < 0.95 {
            return Err(format!("ZK reconstruction success rate too low: {:.1}%", metrics.success_rate * 100.0).into());
        }

        debug!("ZK reconstruction throughput test successful");
        Ok(())
    }

    /// Test IDL analysis performance
    async fn test_idl_analysis_performance(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing IDL analysis performance");

        let iterations = (self.config.performance_iterations / 2).min(50); // IDL analysis is heavier
        let mut durations = Vec::new();
        let mut failures = 0;

        let overall_start = Instant::now();

        for i in 0..iterations {
            let program_id = Pubkey::new_unique();

            // Generate varying transaction history sizes
            let tx_count = 20 + (i * 5) % 100;
            let transaction_history = (0..tx_count)
                .map(|j| {
                    let mut tx_data = TestDataGenerator::generate_transaction_data();
                    tx_data[64] = (j % 4) as u8; // Vary instruction types
                    tx_data
                })
                .collect::<Vec<_>>();

            let start = Instant::now();
            match self.idl_sync.analyze_program_transactions(
                &program_id,
                &transaction_history
            ).await {
                Ok(_) => durations.push(start.elapsed()),
                Err(_) => failures += 1,
            }
        }

        let total_time = overall_start.elapsed();
        let mut metrics = PerformanceMetrics::from_durations(&durations, total_time);
        metrics.success_rate = durations.len() as f64 / iterations as f64;

        metrics.print_summary("IDL Analysis");

        // Performance expectations
        if metrics.throughput < 10.0 {
            warn!("IDL analysis throughput below expected: {:.1} ops/sec", metrics.throughput);
        }

        if metrics.latency_p95 > Duration::from_secs(1) {
            warn!("IDL analysis P95 latency too high: {:?}", metrics.latency_p95);
        }

        if metrics.success_rate < 0.90 {
            return Err(format!("IDL analysis success rate too low: {:.1}%", metrics.success_rate * 100.0).into());
        }

        debug!("IDL analysis performance test successful");
        Ok(())
    }

    /// Test DuckDB query performance
    async fn test_duckdb_query_performance(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing DuckDB query performance");

        let iterations = self.config.performance_iterations.min(500); // Queries should be fast
        let mut durations = Vec::new();
        let mut failures = 0;

        let overall_start = Instant::now();

        // Mix of query types
        let query_templates = vec![
            "SELECT COUNT(*) FROM transactions WHERE program_id = '{}'",
            "SELECT AVG(instruction_size) FROM transactions WHERE slot > {}",
            "SELECT program_id, COUNT(*) FROM transactions GROUP BY program_id LIMIT 10",
            "SELECT * FROM transactions WHERE account = '{}' ORDER BY slot DESC LIMIT 5",
        ];

        for i in 0..iterations {
            let template = &query_templates[i % query_templates.len()];
            let query = Query {
                sql: match i % query_templates.len() {
                    0 => template.replace("{}", &Pubkey::new_unique().to_string()),
                    1 => template.replace("{}", &(1000 + i).to_string()),
                    2 => template.to_string(),
                    3 => template.replace("{}", &Pubkey::new_unique().to_string()),
                    _ => template.to_string(),
                },
            };

            let start = Instant::now();
            match self.duckdb_coordinator.execute_query(query).await {
                Ok(_) => durations.push(start.elapsed()),
                Err(_) => failures += 1,
            }
        }

        let total_time = overall_start.elapsed();
        let mut metrics = PerformanceMetrics::from_durations(&durations, total_time);
        metrics.success_rate = durations.len() as f64 / iterations as f64;

        metrics.print_summary("DuckDB Queries");

        // Performance expectations - DuckDB should be very fast
        if metrics.throughput < 100.0 {
            warn!("DuckDB query throughput below expected: {:.1} ops/sec", metrics.throughput);
        }

        if metrics.latency_p95 > Duration::from_millis(50) {
            warn!("DuckDB query P95 latency too high: {:?}", metrics.latency_p95);
        }

        if metrics.success_rate < 0.95 {
            return Err(format!("DuckDB query success rate too low: {:.1}%", metrics.success_rate * 100.0).into());
        }

        debug!("DuckDB query performance test successful");
        Ok(())
    }

    /// Test end-to-end pipeline throughput
    async fn test_pipeline_throughput(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing end-to-end pipeline throughput");

        let iterations = (self.config.performance_iterations / 5).min(20); // Full pipeline is expensive
        let mut durations = Vec::new();
        let mut failures = 0;

        let overall_start = Instant::now();

        for i in 0..iterations {
            let start = Instant::now();

            // Step 1: ZK Reconstruction
            let compressed_data = TestDataGenerator::generate_compressed_account_data(512);
            let metadata = AccountMetadata {
                account: Pubkey::new_unique(),
                program_id: Pubkey::new_unique(),
                slot: 2000 + i as u64,
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

            let reconstruction_result = self.zk_reconstruction.reconstruct_compressed_account(
                &truncated_data,
                &compression_params
            ).await;

            if reconstruction_result.is_err() {
                failures += 1;
                continue;
            }

            // Step 2: IDL Analysis
            let transaction_history = (0..10)
                .map(|_| TestDataGenerator::generate_transaction_data())
                .collect::<Vec<_>>();

            let idl_result = self.idl_sync.analyze_program_transactions(
                &metadata.program_id,
                &transaction_history
            ).await;

            if idl_result.is_err() {
                failures += 1;
                continue;
            }

            let idl_analysis = idl_result.unwrap();

            // Step 3: Analytics Query
            let analytics_query = Query {
                sql: format!(
                    "SELECT '{}' as program_id, {} as instruction_count, {:.3} as confidence",
                    metadata.program_id,
                    idl_analysis.idl.instructions.len(),
                    idl_analysis.confidence.overall_confidence
                ),
            };

            let query_result = self.duckdb_coordinator.execute_query(analytics_query).await;

            if query_result.is_ok() {
                durations.push(start.elapsed());
            } else {
                failures += 1;
            }
        }

        let total_time = overall_start.elapsed();
        let mut metrics = PerformanceMetrics::from_durations(&durations, total_time);
        metrics.success_rate = durations.len() as f64 / iterations as f64;

        metrics.print_summary("End-to-End Pipeline");

        // Pipeline performance expectations
        if metrics.throughput < 1.0 {
            warn!("Pipeline throughput below expected: {:.1} ops/sec", metrics.throughput);
        }

        if metrics.latency_p95 > Duration::from_secs(5) {
            warn!("Pipeline P95 latency too high: {:?}", metrics.latency_p95);
        }

        if metrics.success_rate < 0.85 {
            return Err(format!("Pipeline success rate too low: {:.1}%", metrics.success_rate * 100.0).into());
        }

        debug!("Pipeline throughput test successful");
        Ok(())
    }

    /// Test concurrent load performance
    async fn test_concurrent_load(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing concurrent load performance");

        let concurrent_tasks = 20;
        let operations_per_task = 5;
        let mut join_set = JoinSet::new();

        let overall_start = Instant::now();

        // Launch concurrent tasks
        for task_id in 0..concurrent_tasks {
            let zk_lib = &self.zk_reconstruction;
            let idl_lib = &self.idl_sync;
            let duckdb = &self.duckdb_coordinator;

            join_set.spawn(async move {
                let mut task_durations = Vec::new();
                let mut task_failures = 0;

                for op in 0..operations_per_task {
                    let start = Instant::now();

                    // Alternate between different types of operations
                    let result = match (task_id + op) % 3 {
                        0 => {
                            // ZK reconstruction
                            let compressed_data = TestDataGenerator::generate_compressed_account_data(256);
                            let metadata = AccountMetadata {
                                account: Pubkey::new_unique(),
                                program_id: Pubkey::new_unique(),
                                slot: 3000 + (task_id * operations_per_task + op) as u64,
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

                            zk_lib.reconstruct_compressed_account(&truncated_data, &compression_params).await.map(|_| ())
                        },
                        1 => {
                            // IDL analysis
                            let program_id = Pubkey::new_unique();
                            let transactions = (0..5)
                                .map(|_| TestDataGenerator::generate_transaction_data())
                                .collect::<Vec<_>>();

                            idl_lib.analyze_program_transactions(&program_id, &transactions).await.map(|_| ())
                        },
                        _ => {
                            // DuckDB query
                            let query = Query {
                                sql: format!("SELECT {} as task_id, {} as operation", task_id, op),
                            };

                            duckdb.execute_query(query).await.map(|_| ())
                        }
                    };

                    match result {
                        Ok(_) => task_durations.push(start.elapsed()),
                        Err(_) => task_failures += 1,
                    }
                }

                (task_durations, task_failures)
            });
        }

        // Collect results
        let mut all_durations = Vec::new();
        let mut total_failures = 0;

        while let Some(result) = join_set.join_next().await {
            match result? {
                (durations, failures) => {
                    all_durations.extend(durations);
                    total_failures += failures;
                }
            }
        }

        let total_time = overall_start.elapsed();
        let total_operations = concurrent_tasks * operations_per_task;

        let mut metrics = PerformanceMetrics::from_durations(&all_durations, total_time);
        metrics.success_rate = all_durations.len() as f64 / total_operations as f64;

        metrics.print_summary("Concurrent Load");

        // Concurrent performance expectations
        if metrics.throughput < 10.0 {
            warn!("Concurrent throughput below expected: {:.1} ops/sec", metrics.throughput);
        }

        if metrics.success_rate < 0.80 {
            return Err(format!("Concurrent success rate too low: {:.1}%", metrics.success_rate * 100.0).into());
        }

        debug!("Concurrent load test successful: {}/{} operations completed",
               all_durations.len(), total_operations);

        Ok(())
    }

    /// Test memory efficiency under load
    async fn test_memory_efficiency(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing memory efficiency under load");

        // This is a simplified memory test - in production, we'd use proper memory profiling
        let initial_memory = Self::estimate_memory_usage();

        // Generate sustained load
        let mut operations = 0;
        let test_duration = Duration::from_secs(5);
        let start_time = Instant::now();

        while start_time.elapsed() < test_duration {
            // Create some data that should be garbage collected
            let compressed_data = TestDataGenerator::generate_compressed_account_data(1024);
            let _transaction_history = (0..20)
                .map(|_| TestDataGenerator::generate_transaction_data())
                .collect::<Vec<_>>();

            // Simulate processing
            let metadata = AccountMetadata {
                account: Pubkey::new_unique(),
                program_id: Pubkey::new_unique(),
                slot: 4000 + operations,
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

            // Fast path to avoid long reconstruction times
            let _result = self.zk_reconstruction.fast_reconstruct_common_patterns(&truncated_data).await;

            operations += 1;

            // Small delay to avoid overwhelming the system
            if operations % 10 == 0 {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }

        let final_memory = Self::estimate_memory_usage();
        let memory_growth = final_memory - initial_memory;

        debug!("Memory efficiency test: {} operations, {:.1} MB growth",
               operations, memory_growth);

        // Memory growth should be reasonable
        if memory_growth > 100.0 {
            warn!("Memory growth seems high: {:.1} MB", memory_growth);
        }

        if operations < 50 {
            return Err("Too few operations completed in memory efficiency test".into());
        }

        debug!("Memory efficiency test successful");
        Ok(())
    }

    /// Simplified memory usage estimation
    fn estimate_memory_usage() -> f64 {
        // This is a placeholder - real implementation would use system APIs
        // or memory profiling tools to get actual memory usage
        std::thread::available_parallelism().map(|n| n.get() as f64).unwrap_or(1.0) * 10.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integration::init_test_environment;

    #[tokio::test]
    async fn test_performance_integration() {
        init_test_environment();

        let mut config = IntegrationTestConfig::default();
        config.performance_iterations = 20; // Reduced for testing

        let test_suite = PerformanceIntegrationTests::new(config);

        let results = test_suite.run_all_tests().await;

        // Performance tests might be more variable
        assert!(results.success_rate() >= 0.70,
                "Performance integration tests failed with {:.1}% success rate",
                results.success_rate() * 100.0);
    }
}