//! StreamSync Core Libraries Integration Tests
//!
//! Comprehensive integration test suite that validates the interaction between
//! ZK reconstruction, IDL sync, and distributed DuckDB libraries under realistic
//! scenarios and performance conditions.

mod integration;

use integration::{
    IntegrationTestConfig, TestResults, init_test_environment,
    zk_reconstruction_integration::ZKReconstructionIntegrationTests,
    idl_sync_integration::IDLSyncIntegrationTests,
    distributed_duckdb_integration::DistributedDuckDBIntegrationTests,
    cross_library_integration::CrossLibraryIntegrationTests,
    performance_integration::PerformanceIntegrationTests,
};
use std::time::Duration;
use tracing::{info, error};

/// Main integration test suite runner
pub async fn run_full_integration_test_suite() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_environment();

    info!("🚀 Starting StreamSync Full Integration Test Suite");
    info!("Testing ZK Reconstruction, IDL Sync, and Distributed DuckDB libraries");

    let config = IntegrationTestConfig {
        test_timeout: Duration::from_secs(30),
        performance_iterations: 50,
        test_data_size: 1024,
        enable_logging: true,
    };

    let mut overall_results = TestResults::default();
    let mut suite_failures = Vec::new();

    // Test Suite 1: ZK Reconstruction Integration
    info!("📦 Running ZK Reconstruction Integration Tests");
    let zk_tests = ZKReconstructionIntegrationTests::new(config.clone());
    let zk_results = zk_tests.run_all_tests().await;
    merge_results(&mut overall_results, zk_results, "ZK Reconstruction", &mut suite_failures);

    // Test Suite 2: IDL Sync Integration
    info!("🔄 Running IDL Sync Integration Tests");
    let idl_tests = IDLSyncIntegrationTests::new(config.clone());
    let idl_results = idl_tests.run_all_tests().await;
    merge_results(&mut overall_results, idl_results, "IDL Sync", &mut suite_failures);

    // Test Suite 3: Distributed DuckDB Integration
    info!("🗄️ Running Distributed DuckDB Integration Tests");
    let duckdb_tests = DistributedDuckDBIntegrationTests::new(config.clone());
    let duckdb_results = duckdb_tests.run_all_tests().await;
    merge_results(&mut overall_results, duckdb_results, "Distributed DuckDB", &mut suite_failures);

    // Test Suite 4: Cross-Library Integration
    info!("🔗 Running Cross-Library Integration Tests");
    let cross_tests = CrossLibraryIntegrationTests::new(config.clone());
    let cross_results = cross_tests.run_all_tests().await;
    merge_results(&mut overall_results, cross_results, "Cross-Library", &mut suite_failures);

    // Test Suite 5: Performance Integration
    info!("⚡ Running Performance Integration Tests");
    let perf_tests = PerformanceIntegrationTests::new(config.clone());
    let perf_results = perf_tests.run_all_tests().await;
    merge_results(&mut overall_results, perf_results, "Performance", &mut suite_failures);

    // Print final summary
    info!("🎯 Full Integration Test Suite Complete");
    overall_results.print_summary();

    if !suite_failures.is_empty() {
        error!("❌ Test Suite Failures:");
        for failure in &suite_failures {
            error!("   - {}", failure);
        }
    }

    // Determine overall success
    let overall_success_rate = overall_results.success_rate();
    if overall_success_rate < 0.80 {
        return Err(format!(
            "Integration test suite failed with {:.1}% success rate (minimum 80% required)",
            overall_success_rate * 100.0
        ).into());
    }

    info!("✅ Integration test suite passed with {:.1}% success rate", overall_success_rate * 100.0);
    Ok(())
}

/// Merge individual test suite results into overall results
fn merge_results(
    overall: &mut TestResults,
    suite_results: TestResults,
    suite_name: &str,
    failures: &mut Vec<String>,
) {
    overall.passed += suite_results.passed;
    overall.failed += suite_results.failed;
    overall.total_time += suite_results.total_time;

    // Add suite-specific failure information
    for failure in &suite_results.failures {
        failures.push(format!("{}: {}", suite_name, failure));
    }

    overall.failures.extend(suite_results.failures);

    let suite_success_rate = suite_results.success_rate();
    if suite_success_rate < 0.75 {
        failures.push(format!(
            "{} suite has low success rate: {:.1}%",
            suite_name,
            suite_success_rate * 100.0
        ));
    }

    info!("📊 {} Suite Results: {:.1}% success rate ({}/{} tests)",
          suite_name,
          suite_success_rate * 100.0,
          suite_results.passed,
          suite_results.passed + suite_results.failed);
}

/// Quick smoke test for basic functionality
pub async fn run_smoke_tests() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_environment();

    info!("🔥 Running Smoke Tests");

    let config = IntegrationTestConfig {
        test_timeout: Duration::from_secs(10),
        performance_iterations: 5,
        test_data_size: 256,
        enable_logging: false,
    };

    // Quick test of each library
    let zk_tests = ZKReconstructionIntegrationTests::new(config.clone());
    let idl_tests = IDLSyncIntegrationTests::new(config.clone());
    let duckdb_tests = DistributedDuckDBIntegrationTests::new(config.clone());

    // Run one test from each suite
    let mut results = TestResults::default();

    // Test ZK reconstruction
    match zk_tests.test_basic_reconstruction().await {
        Ok(_) => {
            results.add_success(Duration::from_millis(100));
            info!("✅ ZK Reconstruction: Basic test passed");
        },
        Err(e) => {
            results.add_failure(format!("ZK Reconstruction: {}", e), Duration::from_millis(100));
            error!("❌ ZK Reconstruction: Basic test failed");
        }
    }

    // Test IDL sync
    match idl_tests.test_transaction_analysis().await {
        Ok(_) => {
            results.add_success(Duration::from_millis(200));
            info!("✅ IDL Sync: Transaction analysis passed");
        },
        Err(e) => {
            results.add_failure(format!("IDL Sync: {}", e), Duration::from_millis(200));
            error!("❌ IDL Sync: Transaction analysis failed");
        }
    }

    // Test DuckDB
    match duckdb_tests.test_basic_query_execution().await {
        Ok(_) => {
            results.add_success(Duration::from_millis(50));
            info!("✅ DuckDB: Basic query passed");
        },
        Err(e) => {
            results.add_failure(format!("DuckDB: {}", e), Duration::from_millis(50));
            error!("❌ DuckDB: Basic query failed");
        }
    }

    results.print_summary();

    if results.success_rate() < 1.0 {
        return Err("Smoke tests failed - basic functionality not working".into());
    }

    info!("🔥 Smoke tests passed - all libraries functional");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn smoke_test() {
        run_smoke_tests().await.expect("Smoke tests should pass");
    }

    #[tokio::test]
    #[ignore] // This test takes a long time - run explicitly with `cargo test --ignored`
    async fn full_integration_test() {
        run_full_integration_test_suite().await.expect("Full integration tests should pass");
    }

    #[tokio::test]
    async fn quick_integration_test() {
        init_test_environment();

        // Run a subset of tests quickly
        let config = IntegrationTestConfig {
            test_timeout: Duration::from_secs(15),
            performance_iterations: 10,
            test_data_size: 512,
            enable_logging: true,
        };

        let cross_tests = CrossLibraryIntegrationTests::new(config);
        let results = cross_tests.run_all_tests().await;

        assert!(results.success_rate() >= 0.75,
                "Quick integration tests failed with {:.1}% success rate",
                results.success_rate() * 100.0);
    }
}