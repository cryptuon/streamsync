//! Integration Test Framework for StreamSync Core Libraries
//!
//! This module provides comprehensive integration testing for the interaction
//! between ZK reconstruction, IDL sync, and distributed DuckDB libraries.

pub mod zk_reconstruction_integration;
pub mod idl_sync_integration;
pub mod distributed_duckdb_integration;
pub mod cross_library_integration;
pub mod performance_integration;

use std::time::Duration;
use tokio::time::timeout;
use tracing::{info, warn};

/// Test configuration for integration tests
#[derive(Debug, Clone)]
pub struct IntegrationTestConfig {
    pub test_timeout: Duration,
    pub performance_iterations: usize,
    pub test_data_size: usize,
    pub enable_logging: bool,
}

impl Default for IntegrationTestConfig {
    fn default() -> Self {
        Self {
            test_timeout: Duration::from_secs(30),
            performance_iterations: 100,
            test_data_size: 1024,
            enable_logging: true,
        }
    }
}

/// Initialize test environment with logging
pub fn init_test_environment() {
    use tracing_subscriber::{fmt, EnvFilter};

    let _ = fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .try_init();
}

/// Generate test data for various scenarios
pub struct TestDataGenerator;

impl TestDataGenerator {
    /// Generate mock compressed account data
    pub fn generate_compressed_account_data(size: usize) -> Vec<u8> {
        // Generate realistic-looking compressed data
        let mut data = Vec::with_capacity(size);

        // Add some recognizable patterns that compression algorithms might create
        for i in 0..size {
            match i % 16 {
                0..=3 => data.push(0xFF), // Common pattern
                4..=7 => data.push((i % 256) as u8), // Sequential
                8..=11 => data.push(0x00), // Zeros
                _ => data.push(((i * 37) % 256) as u8), // Pseudo-random
            }
        }

        data
    }

    /// Generate mock Solana transaction data
    pub fn generate_transaction_data() -> Vec<u8> {
        // Simulate Solana transaction structure
        let mut tx_data = Vec::new();

        // Mock transaction signature (64 bytes)
        tx_data.extend_from_slice(&[0x42; 64]);

        // Mock instruction data
        tx_data.extend_from_slice(&[0x01, 0x02, 0x03, 0x04]); // Instruction discriminator
        tx_data.extend_from_slice(&[0x10, 0x20, 0x30, 0x40]); // Parameters

        // Mock account keys (32 bytes each)
        for i in 0..3 {
            let mut key = [0u8; 32];
            key[0] = i;
            tx_data.extend_from_slice(&key);
        }

        tx_data
    }

    /// Generate mock DuckDB query
    pub fn generate_analytical_query() -> String {
        "SELECT
            program_id,
            COUNT(*) as transaction_count,
            AVG(instruction_size) as avg_size,
            SUM(gas_used) as total_gas
        FROM transactions
        WHERE block_time > NOW() - INTERVAL '1 hour'
        GROUP BY program_id
        ORDER BY transaction_count DESC
        LIMIT 100".to_string()
    }
}

/// Utility for running tests with timeout
pub async fn run_with_timeout<F, T>(
    future: F,
    timeout_duration: Duration,
    test_name: &str,
) -> Result<T, String>
where
    F: std::future::Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>>,
{
    match timeout(timeout_duration, future).await {
        Ok(Ok(result)) => {
            info!("✅ {} completed successfully", test_name);
            Ok(result)
        },
        Ok(Err(e)) => {
            warn!("❌ {} failed: {}", test_name, e);
            Err(format!("{} failed: {}", test_name, e))
        },
        Err(_) => {
            warn!("⏰ {} timed out after {:?}", test_name, timeout_duration);
            Err(format!("{} timed out", test_name))
        }
    }
}

/// Test result aggregator
#[derive(Debug, Default)]
pub struct TestResults {
    pub passed: u32,
    pub failed: u32,
    pub total_time: Duration,
    pub failures: Vec<String>,
}

impl TestResults {
    pub fn add_success(&mut self, duration: Duration) {
        self.passed += 1;
        self.total_time += duration;
    }

    pub fn add_failure(&mut self, error: String, duration: Duration) {
        self.failed += 1;
        self.total_time += duration;
        self.failures.push(error);
    }

    pub fn success_rate(&self) -> f64 {
        if self.passed + self.failed == 0 {
            0.0
        } else {
            self.passed as f64 / (self.passed + self.failed) as f64
        }
    }

    pub fn average_time(&self) -> Duration {
        if self.passed + self.failed == 0 {
            Duration::ZERO
        } else {
            self.total_time / (self.passed + self.failed)
        }
    }

    pub fn print_summary(&self) {
        info!("📊 Test Results Summary:");
        info!("   ✅ Passed: {}", self.passed);
        info!("   ❌ Failed: {}", self.failed);
        info!("   📈 Success Rate: {:.1}%", self.success_rate() * 100.0);
        info!("   ⏱️  Average Time: {:?}", self.average_time());

        if !self.failures.is_empty() {
            info!("   🔍 Failures:");
            for failure in &self.failures {
                info!("      - {}", failure);
            }
        }
    }
}