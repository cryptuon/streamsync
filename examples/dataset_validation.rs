//! Dataset-Driven Validation Testing
//!
//! This example demonstrates comprehensive testing using our collected real Solana dataset
//! to build statistical confidence in the StreamSync libraries

#![allow(dead_code)]

use zk_reconstruction::{ZKReconstructionLibrary, types::{TruncatedData, CompressionParams, CompressionType, TruncationMetadata}};
use idl_sync::IDLSyncLibrary;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::time::SystemTime;
use std::fs;
use tracing::{info, warn};
use serde::{Serialize, Deserialize};
use anyhow::{Result, Context};

/// Collected transaction from our dataset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectedTransaction {
    pub signature: String,
    pub slot: u64,
    pub block_time: Option<i64>,
    pub program_id: String,
    pub program_name: String,
    pub instruction_data: Vec<u8>,
    pub accounts: Vec<String>,
    pub pre_balances: Vec<u64>,
    pub post_balances: Vec<u64>,
    pub log_messages: Vec<String>,
    pub compute_units_consumed: Option<u64>,
    pub success: bool,
    pub error_message: Option<String>,
    pub is_state_compression: bool,
    pub merkle_tree: Option<String>,
    pub compressed_account_data: Option<Vec<u8>>,
    pub compression_proof: Option<Vec<String>>,
}

/// Validation test results
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResults {
    pub total_tests: usize,
    pub successful_tests: usize,
    pub failed_tests: usize,
    pub success_rate: f64,
    pub program_results: std::collections::HashMap<String, ProgramTestResults>,
    pub compression_results: CompressionTestResults,
    pub confidence_score: f64,
    pub validation_timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramTestResults {
    pub program_name: String,
    pub total_tests: usize,
    pub successful_reconstructions: usize,
    pub failed_reconstructions: usize,
    pub success_rate: f64,
    pub average_confidence: f64,
    pub average_reconstruction_time_ms: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompressionTestResults {
    pub total_compression_tests: usize,
    pub successful_compressions: usize,
    pub compression_success_rate: f64,
    pub average_compression_confidence: f64,
    pub merkle_trees_tested: usize,
}

/// Dataset-driven validation test runner
pub struct DatasetValidator {
    dataset_path: String,
    zk_reconstructor: ZKReconstructionLibrary,
    idl_sync: IDLSyncLibrary,
}

impl DatasetValidator {
    pub fn new(dataset_path: impl Into<String>) -> Self {
        Self {
            dataset_path: dataset_path.into(),
            zk_reconstructor: ZKReconstructionLibrary::new(),
            idl_sync: IDLSyncLibrary::new(),
        }
    }

    /// Run comprehensive validation with the collected dataset
    pub async fn run_comprehensive_validation(&self) -> Result<ValidationResults> {
        info!("🧪 Starting comprehensive dataset validation");

        // Load the dataset
        let dataset = self.load_dataset().await
            .context("Failed to load dataset")?;

        info!("📊 Loaded {} transactions for validation", dataset.len());

        let mut results = ValidationResults {
            total_tests: dataset.len(),
            successful_tests: 0,
            failed_tests: 0,
            success_rate: 0.0,
            program_results: std::collections::HashMap::new(),
            compression_results: CompressionTestResults {
                total_compression_tests: 0,
                successful_compressions: 0,
                compression_success_rate: 0.0,
                average_compression_confidence: 0.0,
                merkle_trees_tested: 0,
            },
            confidence_score: 0.0,
            validation_timestamp: SystemTime::now(),
        };

        // Group transactions by program for systematic testing
        let program_groups = self.group_by_program(&dataset);

        // Test each program category
        for (program_name, transactions) in program_groups {
            info!("🔬 Testing {} with {} transactions", program_name, transactions.len());

            let program_results = self.test_program_transactions(&program_name, &transactions).await?;

            // Update overall stats
            results.successful_tests += program_results.successful_reconstructions;
            results.failed_tests += program_results.failed_reconstructions;

            results.program_results.insert(program_name.clone(), program_results);
        }

        // Test compression-specific functionality
        let compression_txs: Vec<_> = dataset.iter().filter(|tx| tx.is_state_compression).collect();
        if !compression_txs.is_empty() {
            info!("🗜️ Testing {} compression transactions", compression_txs.len());
            results.compression_results = self.test_compression_transactions(&compression_txs).await?;
        }

        // Calculate final metrics
        results.success_rate = if results.total_tests > 0 {
            results.successful_tests as f64 / results.total_tests as f64
        } else {
            0.0
        };

        results.confidence_score = self.calculate_confidence_score(&results);

        self.print_validation_summary(&results);

        Ok(results)
    }

    /// Load dataset from JSON file
    async fn load_dataset(&self) -> Result<Vec<CollectedTransaction>> {
        let dataset_file = format!("{}/solana_dataset.json", self.dataset_path);
        let data = fs::read_to_string(&dataset_file)
            .context("Failed to read dataset file")?;

        let transactions: Vec<CollectedTransaction> = serde_json::from_str(&data)
            .context("Failed to parse dataset JSON")?;

        Ok(transactions)
    }

    /// Group transactions by program for organized testing
    fn group_by_program<'a>(&self, dataset: &'a [CollectedTransaction]) -> std::collections::HashMap<String, Vec<&'a CollectedTransaction>> {
        let mut groups = std::collections::HashMap::new();

        for tx in dataset {
            groups.entry(tx.program_name.clone()).or_insert_with(Vec::new).push(tx);
        }

        groups
    }

    /// Test transactions for a specific program
    async fn test_program_transactions(&self, program_name: &str, transactions: &[&CollectedTransaction]) -> Result<ProgramTestResults> {
        let mut successful = 0;
        let mut failed = 0;
        let mut total_confidence = 0.0;
        let mut total_time = 0.0;

        for (i, tx) in transactions.iter().enumerate() {
            if i % 50 == 0 {
                info!("  📈 Progress: {}/{} transactions tested", i, transactions.len());
            }

            let start_time = std::time::Instant::now();

            match self.test_single_transaction(tx).await {
                Ok(confidence) => {
                    successful += 1;
                    total_confidence += confidence;
                }
                Err(e) => {
                    failed += 1;
                    if transactions.len() <= 20 { // Only log errors for small sets
                        warn!("  ⚠️ Transaction {} failed: {}", tx.signature, e);
                    }
                }
            }

            total_time += start_time.elapsed().as_millis() as f64;
        }

        let success_rate = if transactions.len() > 0 {
            successful as f64 / transactions.len() as f64
        } else {
            0.0
        };

        let avg_confidence = if successful > 0 {
            total_confidence / successful as f64
        } else {
            0.0
        };

        let avg_time = if transactions.len() > 0 {
            total_time / transactions.len() as f64
        } else {
            0.0
        };

        info!("  ✅ {} results: {:.1}% success, {:.2} avg confidence, {:.1}ms avg time",
              program_name, success_rate * 100.0, avg_confidence, avg_time);

        Ok(ProgramTestResults {
            program_name: program_name.to_string(),
            total_tests: transactions.len(),
            successful_reconstructions: successful,
            failed_reconstructions: failed,
            success_rate,
            average_confidence: avg_confidence,
            average_reconstruction_time_ms: avg_time,
        })
    }

    /// Test a single transaction for ZK reconstruction
    async fn test_single_transaction(&self, tx: &CollectedTransaction) -> Result<f64> {
        // Create truncated data from the transaction
        let truncated_data = self.create_truncated_data_from_transaction(tx)?;

        // Set up compression parameters
        let compression_params = self.create_compression_params_from_transaction(tx)?;

        // Attempt reconstruction
        let result = self.zk_reconstructor
            .reconstruct_compressed_account(&truncated_data, &compression_params)
            .await?;

        Ok(result.confidence_score)
    }

    /// Create truncated data from collected transaction
    fn create_truncated_data_from_transaction(&self, tx: &CollectedTransaction) -> Result<TruncatedData> {
        let account_pubkey = Pubkey::from_str(&tx.program_id)
            .context("Failed to parse program ID")?;

        // Simulate truncation by taking first half of instruction data
        let truncation_point = (tx.instruction_data.len() / 2).max(8);
        let truncated_bytes = tx.instruction_data[0..truncation_point].to_vec();

        Ok(TruncatedData {
            data: truncated_bytes,
            original_size_hint: Some(tx.instruction_data.len()),
            truncation_point,
            metadata: TruncationMetadata {
                slot: tx.slot,
                account: account_pubkey,
                program_id: account_pubkey,
                compression_type: if tx.is_state_compression {
                    CompressionType::StateCompression
                } else {
                    CompressionType::Standard
                },
                truncation_timestamp: SystemTime::now(),
            },
        })
    }

    /// Create compression parameters from transaction
    fn create_compression_params_from_transaction(&self, tx: &CollectedTransaction) -> Result<CompressionParams> {
        let mut params = CompressionParams::default();

        if tx.is_state_compression {
            params.compression_type = CompressionType::StateCompression;
            params.merkle_tree_height = 14; // Common height for cNFTs

            // Use a hash of the transaction signature as root hash
            let sig_bytes = tx.signature.as_bytes();
            let mut root_hash = [0u8; 32];
            for (i, &byte) in sig_bytes.iter().take(32).enumerate() {
                root_hash[i] = byte;
            }
            params.root_hash = root_hash;
        }

        Ok(params)
    }

    /// Test compression-specific functionality
    async fn test_compression_transactions(&self, transactions: &[&CollectedTransaction]) -> Result<CompressionTestResults> {
        let mut successful = 0;
        let mut total_confidence = 0.0;
        let mut merkle_trees = std::collections::HashSet::new();

        for tx in transactions {
            if let Some(tree) = &tx.merkle_tree {
                merkle_trees.insert(tree.clone());
            }

            match self.test_single_transaction(tx).await {
                Ok(confidence) => {
                    successful += 1;
                    total_confidence += confidence;
                }
                Err(_) => {} // Compression failures are expected in some cases
            }
        }

        let success_rate = if transactions.len() > 0 {
            successful as f64 / transactions.len() as f64
        } else {
            0.0
        };

        let avg_confidence = if successful > 0 {
            total_confidence / successful as f64
        } else {
            0.0
        };

        Ok(CompressionTestResults {
            total_compression_tests: transactions.len(),
            successful_compressions: successful,
            compression_success_rate: success_rate,
            average_compression_confidence: avg_confidence,
            merkle_trees_tested: merkle_trees.len(),
        })
    }

    /// Calculate overall confidence score
    fn calculate_confidence_score(&self, results: &ValidationResults) -> f64 {
        let mut weighted_score = 0.0;
        let mut total_weight = 0.0;

        // Weight program results by transaction count
        for program_result in results.program_results.values() {
            let weight = program_result.total_tests as f64;
            weighted_score += program_result.success_rate * program_result.average_confidence * weight;
            total_weight += weight;
        }

        // Add compression results with higher weight (compression is more complex)
        if results.compression_results.total_compression_tests > 0 {
            let compression_weight = results.compression_results.total_compression_tests as f64 * 1.5;
            weighted_score += results.compression_results.compression_success_rate *
                             results.compression_results.average_compression_confidence *
                             compression_weight;
            total_weight += compression_weight;
        }

        if total_weight > 0.0 {
            weighted_score / total_weight
        } else {
            0.0
        }
    }

    /// Print comprehensive validation summary
    fn print_validation_summary(&self, results: &ValidationResults) {
        info!("🏆 === COMPREHENSIVE VALIDATION RESULTS ===");
        info!("📊 Overall Performance:");
        info!("   Total Tests: {}", results.total_tests);
        info!("   Success Rate: {:.2}%", results.success_rate * 100.0);
        info!("   Overall Confidence Score: {:.3}", results.confidence_score);

        info!("🏗️ Program-Specific Results:");
        for (program, result) in &results.program_results {
            info!("   {}: {:.1}% success ({}/{}), {:.2} avg confidence, {:.1}ms avg time",
                  program,
                  result.success_rate * 100.0,
                  result.successful_reconstructions,
                  result.total_tests,
                  result.average_confidence,
                  result.average_reconstruction_time_ms);
        }

        if results.compression_results.total_compression_tests > 0 {
            info!("🗜️ Compression Results:");
            info!("   Success Rate: {:.1}% ({}/{})",
                  results.compression_results.compression_success_rate * 100.0,
                  results.compression_results.successful_compressions,
                  results.compression_results.total_compression_tests);
            info!("   Average Confidence: {:.2}", results.compression_results.average_compression_confidence);
            info!("   Merkle Trees Tested: {}", results.compression_results.merkle_trees_tested);
        }

        // Provide confidence assessment
        self.assess_confidence_level(results);
    }

    /// Assess and report confidence level
    fn assess_confidence_level(&self, results: &ValidationResults) {
        let confidence = results.confidence_score;
        let success_rate = results.success_rate;

        info!("🎯 Confidence Assessment:");

        if success_rate >= 0.90 && confidence >= 0.70 {
            info!("   ✅ HIGH CONFIDENCE - Libraries are production-ready");
            info!("   ✅ Excellent success rate and reconstruction quality");
        } else if success_rate >= 0.75 && confidence >= 0.50 {
            info!("   ⚠️  MEDIUM CONFIDENCE - Libraries show good performance");
            info!("   ⚠️  Some edge cases may need attention");
        } else if success_rate >= 0.50 && confidence >= 0.30 {
            info!("   🟡 LOW CONFIDENCE - Libraries need improvement");
            info!("   🟡 Consider algorithm refinements");
        } else {
            info!("   ❌ INSUFFICIENT CONFIDENCE - Significant issues detected");
            info!("   ❌ Major algorithmic improvements needed");
        }

        info!("📋 Next Steps:");
        if results.confidence_score >= 0.70 {
            info!("   • Ready for production testing with real data");
            info!("   • Consider performance optimization");
            info!("   • Implement monitoring and alerting");
        } else {
            info!("   • Focus on improving reconstruction algorithms");
            info!("   • Analyze failure patterns in detail");
            info!("   • Collect more diverse test data");
        }
    }

    /// Save validation results
    pub fn save_results(&self, results: &ValidationResults) -> Result<()> {
        let output_file = format!("{}/validation_results.json", self.dataset_path);
        let results_json = serde_json::to_string_pretty(results)
            .context("Failed to serialize results")?;

        fs::write(output_file, results_json)
            .context("Failed to write results file")?;

        info!("💾 Validation results saved to: {}/validation_results.json", self.dataset_path);
        Ok(())
    }
}

/// CLI entry point for dataset validation
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    info!("🚀 Starting Dataset-Driven Validation");

    let dataset_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "./collected_data".to_string());

    let validator = DatasetValidator::new(dataset_path);
    let results = validator.run_comprehensive_validation().await?;
    validator.save_results(&results)?;

    info!("✅ Dataset validation completed successfully!");

    Ok(())
}