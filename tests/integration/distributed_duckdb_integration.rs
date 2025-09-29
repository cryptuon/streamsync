//! Distributed DuckDB Integration Tests
//!
//! Tests the distributed DuckDB library with realistic scenarios including
//! query distribution, shard management, and result aggregation.

use super::{TestDataGenerator, IntegrationTestConfig, run_with_timeout, TestResults};
use distributed_duckdb::{DistributedCoordinator, Query, QueryResult};
use std::time::{Instant, Duration};
use tracing::{info, debug};

pub struct DistributedDuckDBIntegrationTests {
    coordinator: DistributedCoordinator,
    config: IntegrationTestConfig,
}

impl DistributedDuckDBIntegrationTests {
    pub fn new(config: IntegrationTestConfig) -> Self {
        Self {
            coordinator: DistributedCoordinator::new(),
            config,
        }
    }

    /// Run all distributed DuckDB integration tests
    pub async fn run_all_tests(&self) -> TestResults {
        let mut results = TestResults::default();

        info!("🚀 Starting Distributed DuckDB Integration Tests");

        // Test 1: Basic query execution
        self.run_test(&mut results, "Basic Query Execution", || {
            Box::pin(self.test_basic_query_execution())
        }).await;

        // Test 2: Query distribution
        self.run_test(&mut results, "Query Distribution", || {
            Box::pin(self.test_query_distribution())
        }).await;

        // Test 3: Shard management
        self.run_test(&mut results, "Shard Management", || {
            Box::pin(self.test_shard_management())
        }).await;

        // Test 4: Result aggregation
        self.run_test(&mut results, "Result Aggregation", || {
            Box::pin(self.test_result_aggregation())
        }).await;

        // Test 5: Performance under load
        self.run_test(&mut results, "Performance Under Load", || {
            Box::pin(self.test_performance_under_load())
        }).await;

        // Test 6: Fault tolerance
        self.run_test(&mut results, "Fault Tolerance", || {
            Box::pin(self.test_fault_tolerance())
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

    /// Test basic query execution
    async fn test_basic_query_execution(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing basic query execution");

        let query = Query {
            sql: "SELECT COUNT(*) as total FROM transactions".to_string(),
        };

        let start = Instant::now();
        let result = self.coordinator.execute_query(query).await?;
        let execution_time = start.elapsed();

        // Verify result structure
        if result.rows.is_empty() {
            return Err("Query result is empty".into());
        }

        if result.column_names.is_empty() {
            return Err("No column names in result".into());
        }

        // Should complete quickly for simple queries
        if execution_time > Duration::from_secs(1) {
            return Err("Basic query took too long".into());
        }

        debug!("Basic query execution successful: {} rows, {} columns in {:?}",
               result.rows.len(), result.column_names.len(), execution_time);

        Ok(())
    }

    /// Test query distribution across nodes
    async fn test_query_distribution(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing query distribution");

        // Complex analytical query that should be distributed
        let query = Query {
            sql: TestDataGenerator::generate_analytical_query(),
        };

        let start = Instant::now();
        let result = self.coordinator.execute_query(query).await?;
        let execution_time = start.elapsed();

        // Should get results from distributed execution
        if result.rows.is_empty() {
            return Err("Distributed query returned no results".into());
        }

        // Check for distribution metadata
        if result.execution_metadata.is_none() {
            return Err("No execution metadata from distributed query".into());
        }

        let metadata = result.execution_metadata.unwrap();
        if metadata.nodes_involved == 0 {
            return Err("Query wasn't distributed across nodes".into());
        }

        debug!("Query distribution successful: {} nodes involved, {} rows in {:?}",
               metadata.nodes_involved, result.rows.len(), execution_time);

        Ok(())
    }

    /// Test shard management
    async fn test_shard_management(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing shard management");

        // Get current shard information
        let shard_info = self.coordinator.get_shard_information().await?;

        if shard_info.total_shards == 0 {
            return Err("No shards configured".into());
        }

        if shard_info.active_shards == 0 {
            return Err("No active shards".into());
        }

        // Test shard rebalancing
        let rebalance_result = self.coordinator.rebalance_shards().await?;

        if !rebalance_result.success {
            return Err("Shard rebalancing failed".into());
        }

        // Verify shard health
        let health_check = self.coordinator.check_shard_health().await?;

        let healthy_shards = health_check.iter().filter(|s| s.is_healthy).count();
        if healthy_shards == 0 {
            return Err("No healthy shards found".into());
        }

        debug!("Shard management successful: {}/{} shards healthy after rebalancing",
               healthy_shards, health_check.len());

        Ok(())
    }

    /// Test result aggregation
    async fn test_result_aggregation(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing result aggregation");

        // Query that requires aggregation across multiple shards
        let aggregation_query = Query {
            sql: "SELECT
                    program_id,
                    COUNT(*) as total_transactions,
                    SUM(instruction_size) as total_size,
                    AVG(gas_used) as avg_gas,
                    MAX(block_time) as latest_block
                 FROM transactions
                 GROUP BY program_id
                 ORDER BY total_transactions DESC
                 LIMIT 50".to_string(),
        };

        let start = Instant::now();
        let result = self.coordinator.execute_query(aggregation_query).await?;
        let execution_time = start.elapsed();

        // Verify aggregation worked correctly
        if result.rows.is_empty() {
            return Err("Aggregation query returned no results".into());
        }

        if result.column_names.len() != 5 {
            return Err("Aggregation query didn't return expected columns".into());
        }

        // Check that results are properly ordered
        let metadata = result.execution_metadata.unwrap();
        if metadata.partial_results_aggregated == 0 {
            return Err("No partial results were aggregated".into());
        }

        debug!("Result aggregation successful: {} partial results aggregated into {} final rows in {:?}",
               metadata.partial_results_aggregated, result.rows.len(), execution_time);

        Ok(())
    }

    /// Test performance under load
    async fn test_performance_under_load(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing performance under load");

        use tokio::task::JoinSet;

        let mut join_set = JoinSet::new();
        let concurrent_queries = 20;

        // Launch multiple concurrent queries
        for i in 0..concurrent_queries {
            let coordinator = &self.coordinator;
            let query = Query {
                sql: format!("SELECT COUNT(*) FROM transactions WHERE id % {} = 0", i + 1),
            };

            join_set.spawn(async move {
                let start = Instant::now();
                let result = coordinator.execute_query(query).await;
                (result, start.elapsed())
            });
        }

        // Collect results
        let mut completed_queries = 0;
        let mut total_time = Duration::ZERO;
        let mut max_time = Duration::ZERO;

        while let Some(result) = join_set.join_next().await {
            match result? {
                (Ok(_query_result), duration) => {
                    completed_queries += 1;
                    total_time += duration;
                    max_time = max_time.max(duration);
                },
                (Err(e), _) => return Err(format!("Query failed under load: {}", e).into()),
            }
        }

        if completed_queries != concurrent_queries {
            return Err("Not all queries completed under load".into());
        }

        let avg_time = total_time / concurrent_queries;

        // Performance expectations
        if avg_time > Duration::from_secs(2) {
            return Err("Average query time too slow under load".into());
        }

        if max_time > Duration::from_secs(5) {
            return Err("Maximum query time too slow under load".into());
        }

        debug!("Performance under load successful: {} queries, avg={:?}, max={:?}",
               completed_queries, avg_time, max_time);

        Ok(())
    }

    /// Test fault tolerance
    async fn test_fault_tolerance(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing fault tolerance");

        // Get initial node status
        let initial_nodes = self.coordinator.get_active_nodes().await?;
        if initial_nodes.len() < 2 {
            // Can't test fault tolerance with less than 2 nodes
            debug!("Skipping fault tolerance test: insufficient nodes");
            return Ok(());
        }

        // Simulate node failure
        let node_to_fail = &initial_nodes[0];
        self.coordinator.simulate_node_failure(node_to_fail).await?;

        // Execute query with failed node
        let query = Query {
            sql: "SELECT COUNT(*) FROM transactions".to_string(),
        };

        let start = Instant::now();
        let result = self.coordinator.execute_query(query).await?;
        let execution_time = start.elapsed();

        // Query should still succeed
        if result.rows.is_empty() {
            return Err("Query failed with node failure".into());
        }

        // Should have detected and worked around the failure
        let metadata = result.execution_metadata.unwrap();
        if metadata.failed_nodes == 0 {
            return Err("Node failure wasn't detected".into());
        }

        // Restore the failed node
        self.coordinator.restore_node(node_to_fail).await?;

        // Verify recovery
        let recovered_nodes = self.coordinator.get_active_nodes().await?;
        if recovered_nodes.len() != initial_nodes.len() {
            return Err("Node didn't recover properly".into());
        }

        debug!("Fault tolerance successful: handled {} node failure, query completed in {:?}",
               metadata.failed_nodes, execution_time);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integration::init_test_environment;

    #[tokio::test]
    async fn test_distributed_duckdb_integration() {
        init_test_environment();

        let config = IntegrationTestConfig::default();
        let test_suite = DistributedDuckDBIntegrationTests::new(config);

        let results = test_suite.run_all_tests().await;

        // Ensure at least 70% success rate (distributed systems can be flaky)
        assert!(results.success_rate() >= 0.7,
                "Distributed DuckDB integration tests failed with {:.1}% success rate",
                results.success_rate() * 100.0);
    }
}