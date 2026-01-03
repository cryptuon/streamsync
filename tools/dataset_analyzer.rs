//! Dataset Analysis Tool
//!
//! Analyzes collected Solana data to:
//! - Validate data quality and completeness
//! - Extract patterns for ZK reconstruction testing
//! - Generate test cases with known good/bad examples
//! - Create confidence metrics for library validation

use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::fs;
use anyhow::{Result, Context};
use tracing::info;

/// Re-export the CollectedTransaction type for analysis
/// In a real implementation, this would be in a shared library
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

/// Analysis results for the dataset
#[derive(Debug, Serialize, Deserialize)]
pub struct DatasetAnalysis {
    pub dataset_quality: QualityMetrics,
    pub program_patterns: HashMap<String, ProgramPatternAnalysis>,
    pub compression_analysis: CompressionAnalysis,
    pub test_case_recommendations: Vec<TestCaseRecommendation>,
    pub confidence_building_plan: ConfidencePlan,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub total_transactions: usize,
    pub successful_transactions: usize,
    pub failed_transactions: usize,
    pub success_rate: f64,
    pub data_completeness: f64,
    pub program_coverage: HashMap<String, usize>,
    pub size_distribution: SizeDistribution,
    pub temporal_coverage: TemporalCoverage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProgramPatternAnalysis {
    pub program_name: String,
    pub total_transactions: usize,
    pub unique_instruction_patterns: usize,
    pub common_instruction_discriminators: Vec<InstructionPattern>,
    pub account_usage_patterns: Vec<AccountPattern>,
    pub log_message_patterns: Vec<String>,
    pub state_changes: StateChangeAnalysis,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompressionAnalysis {
    pub total_compression_transactions: usize,
    pub unique_merkle_trees: usize,
    pub compression_ratios: Vec<f64>,
    pub average_compression_ratio: f64,
    pub merkle_tree_heights: HashMap<String, u32>,
    pub leaf_count_distributions: Vec<u64>,
    pub proof_sizes: Vec<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestCaseRecommendation {
    pub category: TestCategory,
    pub description: String,
    pub sample_transactions: Vec<String>,
    pub expected_outcome: ExpectedOutcome,
    pub confidence_level: f64,
    pub test_complexity: TestComplexity,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfidencePlan {
    pub minimum_test_cases: usize,
    pub recommended_test_categories: Vec<TestCategory>,
    pub coverage_targets: CoverageTargets,
    pub validation_milestones: Vec<ValidationMilestone>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TestCategory {
    BasicReconstruction,
    CompressionReconstruction,
    LargeDataReconstruction,
    FailureScenarios,
    PerformanceBenchmarks,
    CrossProgramInteractions,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ExpectedOutcome {
    ShouldSucceed,
    ShouldFail,
    ShouldPartiallySucceed,
    ShouldTimeout,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TestComplexity {
    Simple,
    Medium,
    Complex,
    VeryComplex,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstructionPattern {
    pub discriminator: Vec<u8>,
    pub frequency: usize,
    pub average_size: f64,
    pub success_rate: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountPattern {
    pub account_type: String,
    pub frequency: usize,
    pub typical_roles: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StateChangeAnalysis {
    pub balance_changes: Vec<i64>,
    pub account_creations: usize,
    pub account_closures: usize,
    pub data_modifications: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SizeDistribution {
    pub min_size: usize,
    pub max_size: usize,
    pub average_size: f64,
    pub median_size: usize,
    pub size_buckets: HashMap<String, usize>, // e.g., "0-100", "100-500", etc.
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TemporalCoverage {
    pub earliest_slot: u64,
    pub latest_slot: u64,
    pub slot_range: u64,
    pub time_span_hours: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoverageTargets {
    pub min_transactions_per_program: usize,
    pub min_compression_examples: usize,
    pub min_failure_examples: usize,
    pub target_program_coverage: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationMilestone {
    pub milestone: String,
    pub required_test_cases: usize,
    pub success_criteria: String,
    pub confidence_threshold: f64,
}

/// Dataset analyzer
pub struct DatasetAnalyzer {
    dataset_path: String,
    transactions: Vec<CollectedTransaction>,
}

impl DatasetAnalyzer {
    pub fn new(dataset_path: impl Into<String>) -> Self {
        Self {
            dataset_path: dataset_path.into(),
            transactions: Vec::new(),
        }
    }

    /// Load and analyze the dataset
    pub async fn analyze(&mut self) -> Result<DatasetAnalysis> {
        info!("📊 Starting dataset analysis");

        // Load the dataset
        self.load_dataset().await?;

        info!("📈 Loaded {} transactions for analysis", self.transactions.len());

        // Perform analysis
        let quality_metrics = self.analyze_quality();
        let program_patterns = self.analyze_program_patterns();
        let compression_analysis = self.analyze_compression();
        let test_case_recommendations = self.generate_test_recommendations();
        let confidence_plan = self.create_confidence_plan();

        let analysis = DatasetAnalysis {
            dataset_quality: quality_metrics,
            program_patterns,
            compression_analysis,
            test_case_recommendations,
            confidence_building_plan: confidence_plan,
        };

        self.print_analysis_summary(&analysis);

        Ok(analysis)
    }

    /// Load dataset from file
    async fn load_dataset(&mut self) -> Result<()> {
        let dataset_file = Path::new(&self.dataset_path).join("solana_dataset.json");

        if !dataset_file.exists() {
            return Err(anyhow::anyhow!("Dataset file not found: {}", dataset_file.display()));
        }

        let data = fs::read_to_string(&dataset_file)
            .context("Failed to read dataset file")?;

        self.transactions = serde_json::from_str(&data)
            .context("Failed to parse dataset JSON")?;

        Ok(())
    }

    /// Analyze data quality metrics
    fn analyze_quality(&self) -> QualityMetrics {
        let total = self.transactions.len();
        let successful = self.transactions.iter().filter(|tx| tx.success).count();
        let failed = total - successful;
        let success_rate = if total > 0 { successful as f64 / total as f64 } else { 0.0 };

        // Calculate data completeness
        let complete_transactions = self.transactions.iter().filter(|tx| {
            !tx.instruction_data.is_empty() &&
            !tx.accounts.is_empty() &&
            tx.slot > 0
        }).count();
        let data_completeness = if total > 0 { complete_transactions as f64 / total as f64 } else { 0.0 };

        // Program coverage
        let mut program_coverage = HashMap::new();
        for tx in &self.transactions {
            *program_coverage.entry(tx.program_name.clone()).or_insert(0) += 1;
        }

        // Size distribution
        let sizes: Vec<usize> = self.transactions.iter().map(|tx| tx.instruction_data.len()).collect();
        let size_distribution = self.calculate_size_distribution(&sizes);

        // Temporal coverage
        let slots: Vec<u64> = self.transactions.iter().map(|tx| tx.slot).collect();
        let temporal_coverage = self.calculate_temporal_coverage(&slots);

        QualityMetrics {
            total_transactions: total,
            successful_transactions: successful,
            failed_transactions: failed,
            success_rate,
            data_completeness,
            program_coverage,
            size_distribution,
            temporal_coverage,
        }
    }

    /// Analyze patterns per program
    fn analyze_program_patterns(&self) -> HashMap<String, ProgramPatternAnalysis> {
        let mut patterns = HashMap::new();

        // Group transactions by program
        let mut program_groups: HashMap<String, Vec<&CollectedTransaction>> = HashMap::new();
        for tx in &self.transactions {
            program_groups.entry(tx.program_name.clone()).or_default().push(tx);
        }

        for (program_name, txs) in program_groups {
            let analysis = self.analyze_single_program(&program_name, &txs);
            patterns.insert(program_name, analysis);
        }

        patterns
    }

    /// Analyze a single program's patterns
    fn analyze_single_program(&self, program_name: &str, transactions: &[&CollectedTransaction]) -> ProgramPatternAnalysis {
        // Extract instruction patterns
        let mut discriminator_map: HashMap<Vec<u8>, Vec<&CollectedTransaction>> = HashMap::new();

        for tx in transactions {
            if tx.instruction_data.len() >= 8 {
                let discriminator = tx.instruction_data[0..8].to_vec();
                discriminator_map.entry(discriminator).or_default().push(tx);
            }
        }

        let common_patterns: Vec<InstructionPattern> = discriminator_map.iter().map(|(disc, txs)| {
            let avg_size = txs.iter().map(|tx| tx.instruction_data.len()).sum::<usize>() as f64 / txs.len() as f64;
            let success_count = txs.iter().filter(|tx| tx.success).count();
            let success_rate = success_count as f64 / txs.len() as f64;

            InstructionPattern {
                discriminator: disc.clone(),
                frequency: txs.len(),
                average_size: avg_size,
                success_rate,
            }
        }).collect();

        // Analyze account usage patterns
        let account_patterns = self.analyze_account_patterns(transactions);

        // Extract common log patterns
        let log_patterns = self.extract_log_patterns(transactions);

        // Analyze state changes
        let state_changes = self.analyze_state_changes(transactions);

        ProgramPatternAnalysis {
            program_name: program_name.to_string(),
            total_transactions: transactions.len(),
            unique_instruction_patterns: discriminator_map.len(),
            common_instruction_discriminators: common_patterns,
            account_usage_patterns: account_patterns,
            log_message_patterns: log_patterns,
            state_changes,
        }
    }

    /// Analyze compression-specific data
    fn analyze_compression(&self) -> CompressionAnalysis {
        let compression_txs: Vec<_> = self.transactions.iter().filter(|tx| tx.is_state_compression).collect();

        let total_compression_transactions = compression_txs.len();

        let unique_trees: HashSet<String> = compression_txs.iter()
            .filter_map(|tx| tx.merkle_tree.as_ref())
            .cloned()
            .collect();

        // Calculate compression ratios (estimated)
        let compression_ratios: Vec<f64> = compression_txs.iter()
            .filter_map(|tx| {
                if let Some(compressed_data) = &tx.compressed_account_data {
                    if compressed_data.len() > 0 {
                        // Estimate original size vs compressed size
                        Some(tx.instruction_data.len() as f64 / compressed_data.len() as f64)
                    } else {
                        None
                    }
                } else {
                    // Estimate based on instruction data vs typical account size
                    Some(1024.0 / tx.instruction_data.len() as f64)
                }
            })
            .collect();

        let average_compression_ratio = if compression_ratios.is_empty() {
            0.0
        } else {
            compression_ratios.iter().sum::<f64>() / compression_ratios.len() as f64
        };

        CompressionAnalysis {
            total_compression_transactions,
            unique_merkle_trees: unique_trees.len(),
            compression_ratios,
            average_compression_ratio,
            merkle_tree_heights: HashMap::new(), // Would need to be extracted from transaction data
            leaf_count_distributions: vec![],
            proof_sizes: vec![],
        }
    }

    /// Generate test case recommendations
    fn generate_test_recommendations(&self) -> Vec<TestCaseRecommendation> {
        let mut recommendations = Vec::new();

        // Basic reconstruction tests
        recommendations.push(TestCaseRecommendation {
            category: TestCategory::BasicReconstruction,
            description: "Test basic ZK reconstruction with simple SPL Token transfers".to_string(),
            sample_transactions: self.get_sample_transactions("SPL Token", 5),
            expected_outcome: ExpectedOutcome::ShouldSucceed,
            confidence_level: 0.9,
            test_complexity: TestComplexity::Simple,
        });

        // Compression reconstruction tests
        if self.transactions.iter().any(|tx| tx.is_state_compression) {
            recommendations.push(TestCaseRecommendation {
                category: TestCategory::CompressionReconstruction,
                description: "Test ZK reconstruction with compressed NFT data".to_string(),
                sample_transactions: self.get_compression_samples(5),
                expected_outcome: ExpectedOutcome::ShouldSucceed,
                confidence_level: 0.7,
                test_complexity: TestComplexity::Complex,
            });
        }

        // Large data tests
        let large_txs = self.get_large_transactions(3);
        if !large_txs.is_empty() {
            recommendations.push(TestCaseRecommendation {
                category: TestCategory::LargeDataReconstruction,
                description: "Test reconstruction with large transaction data".to_string(),
                sample_transactions: large_txs,
                expected_outcome: ExpectedOutcome::ShouldPartiallySucceed,
                confidence_level: 0.6,
                test_complexity: TestComplexity::Complex,
            });
        }

        // Failure scenario tests
        let failed_txs = self.get_failed_transactions(5);
        if !failed_txs.is_empty() {
            recommendations.push(TestCaseRecommendation {
                category: TestCategory::FailureScenarios,
                description: "Test error handling with known failed transactions".to_string(),
                sample_transactions: failed_txs,
                expected_outcome: ExpectedOutcome::ShouldFail,
                confidence_level: 0.95,
                test_complexity: TestComplexity::Medium,
            });
        }

        recommendations
    }

    /// Create confidence building plan
    fn create_confidence_plan(&self) -> ConfidencePlan {
        let total_txs = self.transactions.len();

        ConfidencePlan {
            minimum_test_cases: (total_txs / 10).max(50),
            recommended_test_categories: vec![
                TestCategory::BasicReconstruction,
                TestCategory::CompressionReconstruction,
                TestCategory::FailureScenarios,
                TestCategory::PerformanceBenchmarks,
            ],
            coverage_targets: CoverageTargets {
                min_transactions_per_program: 20,
                min_compression_examples: 10,
                min_failure_examples: 5,
                target_program_coverage: 0.8,
            },
            validation_milestones: vec![
                ValidationMilestone {
                    milestone: "Basic Functionality".to_string(),
                    required_test_cases: 25,
                    success_criteria: "90% success rate on simple reconstructions".to_string(),
                    confidence_threshold: 0.9,
                },
                ValidationMilestone {
                    milestone: "Compression Support".to_string(),
                    required_test_cases: 15,
                    success_criteria: "70% success rate on compression reconstructions".to_string(),
                    confidence_threshold: 0.7,
                },
                ValidationMilestone {
                    milestone: "Production Ready".to_string(),
                    required_test_cases: 100,
                    success_criteria: "95% success rate across all categories".to_string(),
                    confidence_threshold: 0.95,
                },
            ],
        }
    }

    // Helper methods
    fn calculate_size_distribution(&self, sizes: &[usize]) -> SizeDistribution {
        if sizes.is_empty() {
            return SizeDistribution {
                min_size: 0,
                max_size: 0,
                average_size: 0.0,
                median_size: 0,
                size_buckets: HashMap::new(),
            };
        }

        let mut sorted_sizes = sizes.to_vec();
        sorted_sizes.sort();

        let min_size = *sorted_sizes.first().unwrap();
        let max_size = *sorted_sizes.last().unwrap();
        let average_size = sizes.iter().sum::<usize>() as f64 / sizes.len() as f64;
        let median_size = sorted_sizes[sorted_sizes.len() / 2];

        // Create size buckets
        let mut buckets = HashMap::new();
        for &size in sizes {
            let bucket = match size {
                0..=100 => "0-100",
                101..=500 => "101-500",
                501..=1000 => "501-1000",
                1001..=5000 => "1001-5000",
                _ => "5000+",
            };
            *buckets.entry(bucket.to_string()).or_insert(0) += 1;
        }

        SizeDistribution {
            min_size,
            max_size,
            average_size,
            median_size,
            size_buckets: buckets,
        }
    }

    fn calculate_temporal_coverage(&self, slots: &[u64]) -> TemporalCoverage {
        if slots.is_empty() {
            return TemporalCoverage {
                earliest_slot: 0,
                latest_slot: 0,
                slot_range: 0,
                time_span_hours: 0.0,
            };
        }

        let earliest = *slots.iter().min().unwrap();
        let latest = *slots.iter().max().unwrap();
        let range = latest - earliest;

        // Approximate: ~2.4 slots per second
        let time_span_hours = (range as f64 * 0.4) / 3600.0;

        TemporalCoverage {
            earliest_slot: earliest,
            latest_slot: latest,
            slot_range: range,
            time_span_hours,
        }
    }

    fn analyze_account_patterns(&self, transactions: &[&CollectedTransaction]) -> Vec<AccountPattern> {
        // Simplified account pattern analysis
        vec![
            AccountPattern {
                account_type: "Program Account".to_string(),
                frequency: transactions.len(),
                typical_roles: vec!["program".to_string()],
            }
        ]
    }

    fn extract_log_patterns(&self, transactions: &[&CollectedTransaction]) -> Vec<String> {
        let mut patterns = HashSet::new();

        for tx in transactions {
            for log in &tx.log_messages {
                if log.contains("Program log:") {
                    patterns.insert(log.clone());
                }
            }
        }

        patterns.into_iter().take(10).collect()
    }

    fn analyze_state_changes(&self, transactions: &[&CollectedTransaction]) -> StateChangeAnalysis {
        let mut balance_changes = Vec::new();
        let mut account_creations = 0;
        let account_closures = 0;
        let mut data_modifications = 0;

        for tx in transactions {
            // Calculate balance changes
            for (pre, post) in tx.pre_balances.iter().zip(tx.post_balances.iter()) {
                let change = *post as i64 - *pre as i64;
                balance_changes.push(change);
            }

            // Detect account creations (simplified)
            if tx.pre_balances.iter().any(|&b| b == 0) && tx.post_balances.iter().any(|&b| b > 0) {
                account_creations += 1;
            }

            // Detect modifications (any instruction data suggests modification)
            if !tx.instruction_data.is_empty() {
                data_modifications += 1;
            }
        }

        StateChangeAnalysis {
            balance_changes,
            account_creations,
            account_closures,
            data_modifications,
        }
    }

    fn get_sample_transactions(&self, program_name: &str, count: usize) -> Vec<String> {
        self.transactions
            .iter()
            .filter(|tx| tx.program_name == program_name && tx.success)
            .take(count)
            .map(|tx| tx.signature.clone())
            .collect()
    }

    fn get_compression_samples(&self, count: usize) -> Vec<String> {
        self.transactions
            .iter()
            .filter(|tx| tx.is_state_compression && tx.success)
            .take(count)
            .map(|tx| tx.signature.clone())
            .collect()
    }

    fn get_large_transactions(&self, count: usize) -> Vec<String> {
        let mut large_txs: Vec<_> = self.transactions
            .iter()
            .filter(|tx| tx.instruction_data.len() > 1000)
            .collect();

        large_txs.sort_by_key(|tx| tx.instruction_data.len());
        large_txs.reverse();

        large_txs
            .into_iter()
            .take(count)
            .map(|tx| tx.signature.clone())
            .collect()
    }

    fn get_failed_transactions(&self, count: usize) -> Vec<String> {
        self.transactions
            .iter()
            .filter(|tx| !tx.success)
            .take(count)
            .map(|tx| tx.signature.clone())
            .collect()
    }

    /// Print analysis summary
    fn print_analysis_summary(&self, analysis: &DatasetAnalysis) {
        info!("📊 === DATASET ANALYSIS SUMMARY ===");

        let quality = &analysis.dataset_quality;
        info!("📈 Quality Metrics:");
        info!("   Total Transactions: {}", quality.total_transactions);
        info!("   Success Rate: {:.2}%", quality.success_rate * 100.0);
        info!("   Data Completeness: {:.2}%", quality.data_completeness * 100.0);

        info!("🏗️  Program Coverage:");
        for (program, count) in &quality.program_coverage {
            info!("   {}: {} transactions", program, count);
        }

        let compression = &analysis.compression_analysis;
        info!("🗜️  Compression Analysis:");
        info!("   Compression Transactions: {}", compression.total_compression_transactions);
        info!("   Unique Merkle Trees: {}", compression.unique_merkle_trees);
        info!("   Avg Compression Ratio: {:.2}", compression.average_compression_ratio);

        info!("🧪 Test Recommendations: {} categories", analysis.test_case_recommendations.len());

        let plan = &analysis.confidence_building_plan;
        info!("📋 Confidence Plan:");
        info!("   Minimum Test Cases: {}", plan.minimum_test_cases);
        info!("   Validation Milestones: {}", plan.validation_milestones.len());
    }

    /// Save analysis results
    pub fn save_analysis(&self, analysis: &DatasetAnalysis, output_dir: &str) -> Result<()> {
        let output_path = Path::new(output_dir);
        fs::create_dir_all(output_path)?;

        let analysis_file = output_path.join("dataset_analysis.json");
        let analysis_json = serde_json::to_string_pretty(analysis)?;
        fs::write(analysis_file, analysis_json)?;

        info!("💾 Analysis saved to: {}", output_path.display());
        Ok(())
    }
}

/// CLI entry point for dataset analyzer
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("📊 Starting Dataset Analysis");

    let dataset_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "./collected_data".to_string());

    let mut analyzer = DatasetAnalyzer::new(dataset_path);
    let analysis = analyzer.analyze().await?;
    analyzer.save_analysis(&analysis, "./collected_data")?;

    info!("✅ Dataset analysis completed!");
    Ok(())
}