//! IDL Sync Integration Tests
//!
//! Tests the IDL synchronization library with realistic scenarios including
//! behavioral analysis, consensus mechanisms, and real-time monitoring.

use super::{TestDataGenerator, IntegrationTestConfig, run_with_timeout, TestResults};
use idl_sync::{IDLSyncLibrary, types::IDLAnalysisConfig};
use solana_sdk::pubkey::Pubkey;
use std::time::{Instant, Duration};
use tracing::{info, debug};

pub struct IDLSyncIntegrationTests {
    library: IDLSyncLibrary,
    config: IntegrationTestConfig,
}

impl IDLSyncIntegrationTests {
    pub fn new(config: IntegrationTestConfig) -> Self {
        Self {
            library: IDLSyncLibrary::new(IDLAnalysisConfig::default()),
            config,
        }
    }

    /// Run all IDL sync integration tests
    pub async fn run_all_tests(&self) -> TestResults {
        let mut results = TestResults::default();

        info!("🚀 Starting IDL Sync Integration Tests");

        // Test 1: Basic transaction analysis
        self.run_test(&mut results, "Transaction Analysis", || {
            Box::pin(self.test_transaction_analysis())
        }).await;

        // Test 2: Program behavior learning
        self.run_test(&mut results, "Behavior Learning", || {
            Box::pin(self.test_behavior_learning())
        }).await;

        // Test 3: IDL generation from patterns
        self.run_test(&mut results, "IDL Generation", || {
            Box::pin(self.test_idl_generation())
        }).await;

        // Test 4: Real-time monitoring
        self.run_test(&mut results, "Real-time Monitoring", || {
            Box::pin(self.test_realtime_monitoring())
        }).await;

        // Test 5: Network consensus simulation
        self.run_test(&mut results, "Network Consensus", || {
            Box::pin(self.test_network_consensus())
        }).await;

        // Test 6: Cache performance
        self.run_test(&mut results, "Cache Performance", || {
            Box::pin(self.test_cache_performance())
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

    /// Test basic transaction analysis
    async fn test_transaction_analysis(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing basic transaction analysis");

        // Generate mock transaction history
        let mut transaction_history = Vec::new();
        for i in 0..50 {
            let mut tx_data = TestDataGenerator::generate_transaction_data();
            // Add some variation to simulate different instruction types
            tx_data[64] = (i % 4) as u8; // Vary instruction discriminator
            transaction_history.push(tx_data);
        }

        let program_id = Pubkey::new_unique();

        let start = Instant::now();
        let generated_idl = self.library.analyze_program_transactions(
            &program_id,
            &transaction_history
        ).await?;
        let analysis_time = start.elapsed();

        // Verify results
        if generated_idl.idl.instructions.is_empty() {
            return Err("No instructions detected in transaction analysis".into());
        }

        if generated_idl.confidence.overall_confidence <= 0.0 {
            return Err("Invalid confidence score".into());
        }

        // Should complete quickly
        if analysis_time > Duration::from_secs(2) {
            return Err("Transaction analysis took too long".into());
        }

        debug!("Transaction analysis successful: {} instructions found with {:.2}% confidence in {:?}",
               generated_idl.idl.instructions.len(),
               generated_idl.confidence.overall_confidence * 100.0,
               analysis_time);

        Ok(())
    }

    /// Test program behavior learning over time
    async fn test_behavior_learning(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing behavior learning over time");

        let program_id = Pubkey::new_unique();

        // First batch of transactions
        let mut first_batch = Vec::new();
        for i in 0..20 {
            let mut tx_data = TestDataGenerator::generate_transaction_data();
            tx_data[64] = 0x01; // All use instruction type 1
            first_batch.push(tx_data);
        }

        let first_analysis = self.library.analyze_program_transactions(
            &program_id,
            &first_batch
        ).await?;

        // Second batch with new patterns
        let mut second_batch = first_batch.clone();
        for i in 0..30 {
            let mut tx_data = TestDataGenerator::generate_transaction_data();
            tx_data[64] = 0x02; // New instruction type
            second_batch.push(tx_data);
        }

        let second_analysis = self.library.analyze_program_transactions(
            &program_id,
            &second_batch
        ).await?;

        // Should detect the new instruction type
        if second_analysis.idl.instructions.len() <= first_analysis.idl.instructions.len() {
            return Err("Behavior learning didn't detect new patterns".into());
        }

        // Confidence should improve with more data
        if second_analysis.confidence.overall_confidence <= first_analysis.confidence.overall_confidence {
            return Err("Confidence didn't improve with more data".into());
        }

        debug!("Behavior learning successful: {} → {} instructions, confidence {:.2}% → {:.2}%",
               first_analysis.idl.instructions.len(),
               second_analysis.idl.instructions.len(),
               first_analysis.confidence.overall_confidence * 100.0,
               second_analysis.confidence.overall_confidence * 100.0);

        Ok(())
    }

    /// Test IDL generation from patterns
    async fn test_idl_generation(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing IDL generation from patterns");

        // Create structured transaction data that should generate clear patterns
        let mut transaction_history = Vec::new();

        // Simulate "initialize" instruction pattern
        for _ in 0..15 {
            let mut tx_data = TestDataGenerator::generate_transaction_data();
            tx_data[64] = 0x00; // Initialize discriminator
            tx_data[68] = 0x20; // 32-byte account parameter
            transaction_history.push(tx_data);
        }

        // Simulate "transfer" instruction pattern
        for _ in 0..25 {
            let mut tx_data = TestDataGenerator::generate_transaction_data();
            tx_data[64] = 0x01; // Transfer discriminator
            tx_data[68] = 0x08; // 8-byte amount parameter
            transaction_history.push(tx_data);
        }

        let program_id = Pubkey::new_unique();
        let generated_idl = self.library.analyze_program_transactions(
            &program_id,
            &transaction_history
        ).await?;

        // Should detect both instruction types
        if generated_idl.idl.instructions.len() < 2 {
            return Err("IDL generation didn't detect expected instruction patterns".into());
        }

        // Should have reasonable confidence
        if generated_idl.confidence.overall_confidence < 0.7 {
            return Err("IDL generation confidence too low".into());
        }

        // Should detect account structures
        if generated_idl.idl.accounts.is_empty() {
            return Err("IDL generation didn't detect account structures".into());
        }

        debug!("IDL generation successful: {} instructions, {} accounts with {:.2}% confidence",
               generated_idl.idl.instructions.len(),
               generated_idl.idl.accounts.len(),
               generated_idl.confidence.overall_confidence * 100.0);

        Ok(())
    }

    /// Test real-time monitoring capabilities
    async fn test_realtime_monitoring(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing real-time monitoring");

        let program_id = Pubkey::new_unique();

        // Start monitoring
        self.library.start_monitoring(&program_id).await?;

        // Simulate incoming transactions over time
        let mut total_analyzed = 0;
        for batch in 0..5 {
            // Generate new transactions
            let mut new_transactions = Vec::new();
            for i in 0..10 {
                let mut tx_data = TestDataGenerator::generate_transaction_data();
                tx_data[64] = (batch % 3) as u8; // Vary instruction types
                new_transactions.push(tx_data);
            }

            // Process new transactions
            for tx in &new_transactions {
                self.library.process_new_transaction(&program_id, tx).await?;
                total_analyzed += 1;
            }

            // Small delay to simulate real-time processing
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        // Get current IDL state
        let current_idl = self.library.get_current_idl(&program_id).await?;

        // Should have processed all transactions
        if current_idl.idl.instructions.is_empty() {
            return Err("Real-time monitoring didn't generate IDL".into());
        }

        // Stop monitoring
        self.library.stop_monitoring(&program_id).await?;

        debug!("Real-time monitoring successful: processed {} transactions, generated {} instructions",
               total_analyzed, current_idl.idl.instructions.len());

        Ok(())
    }

    /// Test network consensus simulation
    async fn test_network_consensus(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing network consensus simulation");

        let program_id = Pubkey::new_unique();

        // Generate base transaction data
        let transaction_history = (0..30)
            .map(|i| {
                let mut tx_data = TestDataGenerator::generate_transaction_data();
                tx_data[64] = (i % 2) as u8; // Two instruction types
                tx_data
            })
            .collect::<Vec<_>>();

        // Simulate analysis from multiple nodes
        let mut node_analyses = Vec::new();
        for node_id in 0..5 {
            // Each node sees slightly different transaction sets (simulating network delays)
            let mut node_transactions = transaction_history.clone();
            if node_id > 0 {
                // Remove some transactions to simulate partial views
                node_transactions.truncate(25 + node_id);
            }

            let analysis = self.library.analyze_program_transactions(
                &program_id,
                &node_transactions
            ).await?;

            node_analyses.push(analysis);
        }

        // Simulate consensus process
        let consensus_result = self.library.reach_network_consensus(
            &program_id,
            node_analyses
        ).await?;

        // Consensus should have reasonable confidence
        if consensus_result.consensus_confidence < 0.6 {
            return Err("Network consensus confidence too low".into());
        }

        // Should agree on basic instruction structure
        if consensus_result.agreed_idl.instructions.is_empty() {
            return Err("Network consensus didn't agree on any instructions".into());
        }

        debug!("Network consensus successful: {:.2}% agreement on {} instructions",
               consensus_result.consensus_confidence * 100.0,
               consensus_result.agreed_idl.instructions.len());

        Ok(())
    }

    /// Test cache performance
    async fn test_cache_performance(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Testing cache performance");

        let program_id = Pubkey::new_unique();
        let transaction_history = (0..20)
            .map(|_| TestDataGenerator::generate_transaction_data())
            .collect::<Vec<_>>();

        // First analysis - cache miss
        let start1 = Instant::now();
        let result1 = self.library.analyze_program_transactions(
            &program_id,
            &transaction_history
        ).await?;
        let first_duration = start1.elapsed();

        // Second analysis - should hit cache
        let start2 = Instant::now();
        let result2 = self.library.analyze_program_transactions(
            &program_id,
            &transaction_history
        ).await?;
        let second_duration = start2.elapsed();

        // Cache hit should be faster
        if second_duration >= first_duration {
            return Err("Cache doesn't appear to be working".into());
        }

        // Results should be consistent
        if result1.idl.instructions.len() != result2.idl.instructions.len() {
            return Err("Cached results are inconsistent".into());
        }

        debug!("Cache performance test successful: first={:?}, second={:?} ({}x speedup)",
               first_duration, second_duration,
               first_duration.as_nanos() / second_duration.as_nanos().max(1));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integration::init_test_environment;

    #[tokio::test]
    async fn test_idl_sync_integration() {
        init_test_environment();

        let config = IntegrationTestConfig::default();
        let test_suite = IDLSyncIntegrationTests::new(config);

        let results = test_suite.run_all_tests().await;

        // Ensure at least 75% success rate
        assert!(results.success_rate() >= 0.75,
                "IDL sync integration tests failed with {:.1}% success rate",
                results.success_rate() * 100.0);
    }
}