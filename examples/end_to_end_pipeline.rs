//! End-to-End StreamSync Pipeline Example
//!
//! This example demonstrates the complete StreamSync pipeline:
//! 1. ZK reconstruction of compressed account data
//! 2. IDL analysis of program behavior
//! 3. Distributed analytical queries using DuckDB
//!
//! This showcases how all three libraries work together to provide
//! comprehensive blockchain data analysis capabilities.

use zk_reconstruction::{
    ZKReconstructionLibrary,
    types::{TruncatedData, CompressionParams, CompressionType, AccountMetadata},
};
use idl_sync::{IDLSyncLibrary, types::IDLAnalysisConfig};
use distributed_duckdb::{DistributedCoordinator, Query};
use solana_sdk::pubkey::Pubkey;
use std::time::{Duration, Instant};
use tracing::{info, warn, error};
use serde_json::json;

/// Represents a complete data processing pipeline
pub struct StreamSyncPipeline {
    zk_reconstruction: ZKReconstructionLibrary,
    idl_sync: IDLSyncLibrary,
    duckdb_coordinator: DistributedCoordinator,
}

/// Pipeline processing results
#[derive(Debug)]
pub struct PipelineResults {
    pub accounts_processed: usize,
    pub programs_analyzed: usize,
    pub queries_executed: usize,
    pub total_processing_time: Duration,
    pub success_rate: f64,
}

impl StreamSyncPipeline {
    pub fn new() -> Self {
        Self {
            zk_reconstruction: ZKReconstructionLibrary::new(),
            idl_sync: IDLSyncLibrary::new(IDLAnalysisConfig::default()),
            duckdb_coordinator: DistributedCoordinator::new(),
        }
    }

    /// Process a complete data pipeline from raw compressed data to insights
    pub async fn process_pipeline(
        &self,
        raw_data: Vec<CompressedAccountData>,
        analysis_queries: Vec<String>,
    ) -> Result<PipelineResults, Box<dyn std::error::Error + Send + Sync>> {

        let start_time = Instant::now();
        let mut results = PipelineResults {
            accounts_processed: 0,
            programs_analyzed: 0,
            queries_executed: 0,
            total_processing_time: Duration::ZERO,
            success_rate: 0.0,
        };

        info!("🚀 Starting end-to-end pipeline processing");
        info!("   📦 Input: {} compressed accounts", raw_data.len());
        info!("   📊 Analysis queries: {}", analysis_queries.len());

        // Step 1: ZK Reconstruction
        info!("📦 Step 1: ZK Reconstruction Phase");
        let reconstructed_accounts = self.reconstruct_accounts(raw_data).await?;
        results.accounts_processed = reconstructed_accounts.len();
        info!("   ✅ Reconstructed {} accounts", results.accounts_processed);

        // Step 2: IDL Analysis
        info!("🔄 Step 2: IDL Analysis Phase");
        let program_idls = self.analyze_program_behavior(&reconstructed_accounts).await?;
        results.programs_analyzed = program_idls.len();
        info!("   ✅ Analyzed {} programs", results.programs_analyzed);

        // Step 3: Store data and generate insights
        info!("🗄️ Step 3: Data Storage and Analytics");
        let analytical_results = self.execute_analytical_queries(
            &reconstructed_accounts,
            &program_idls,
            analysis_queries,
        ).await?;
        results.queries_executed = analytical_results.len();
        info!("   ✅ Executed {} analytical queries", results.queries_executed);

        // Step 4: Generate insights report
        info!("📊 Step 4: Insights Generation");
        self.generate_insights_report(&reconstructed_accounts, &program_idls, &analytical_results).await?;

        results.total_processing_time = start_time.elapsed();
        results.success_rate = 1.0; // Simplified success calculation

        info!("✅ Pipeline completed in {:?}", results.total_processing_time);
        Ok(results)
    }

    /// Reconstruct compressed accounts using ZK reconstruction
    async fn reconstruct_accounts(
        &self,
        raw_data: Vec<CompressedAccountData>,
    ) -> Result<Vec<ReconstructedAccountInfo>, Box<dyn std::error::Error + Send + Sync>> {

        let mut reconstructed = Vec::new();

        for (i, account_data) in raw_data.iter().enumerate() {
            info!("   Processing account {}/{}", i + 1, raw_data.len());

            let truncated_data = TruncatedData {
                data: account_data.compressed_data.clone(),
                metadata: AccountMetadata {
                    account: account_data.account_id,
                    program_id: account_data.program_id,
                    slot: account_data.slot,
                    compression_type: account_data.compression_type.clone(),
                },
            };

            let compression_params = CompressionParams {
                compression_type: account_data.compression_type.clone(),
                merkle_tree_height: account_data.merkle_tree_height,
                compression_level: account_data.compression_level,
            };

            match self.zk_reconstruction.reconstruct_compressed_account(
                &truncated_data,
                &compression_params
            ).await {
                Ok(result) => {
                    let info = ReconstructedAccountInfo {
                        account_id: account_data.account_id,
                        program_id: account_data.program_id,
                        slot: account_data.slot,
                        original_size: account_data.compressed_data.len(),
                        reconstructed_size: result.account_data.len(),
                        confidence: result.confidence_score,
                        reconstruction_method: format!("{:?}", result.reconstruction_method),
                        reconstruction_time: result.reconstruction_time,
                        account_data: result.account_data,
                    };

                    info!("     ✅ Account {} reconstructed: {} → {} bytes ({:.1}% confidence)",
                          account_data.account_id,
                          info.original_size,
                          info.reconstructed_size,
                          info.confidence * 100.0);

                    reconstructed.push(info);
                },
                Err(e) => {
                    warn!("     ❌ Failed to reconstruct account {}: {}", account_data.account_id, e);
                }
            }
        }

        Ok(reconstructed)
    }

    /// Analyze program behavior to generate IDLs
    async fn analyze_program_behavior(
        &self,
        reconstructed_accounts: &[ReconstructedAccountInfo],
    ) -> Result<Vec<ProgramIDLInfo>, Box<dyn std::error::Error + Send + Sync>> {

        // Group accounts by program
        let mut programs = std::collections::HashMap::new();
        for account in reconstructed_accounts {
            programs.entry(account.program_id)
                .or_insert_with(Vec::new)
                .push(account);
        }

        let mut program_idls = Vec::new();

        for (program_id, accounts) in programs {
            info!("   Analyzing program {} with {} accounts", program_id, accounts.len());

            // Generate synthetic transaction history from account data
            let transaction_history = self.generate_transaction_history(accounts);

            match self.idl_sync.analyze_program_transactions(
                &program_id,
                &transaction_history
            ).await {
                Ok(generated_idl) => {
                    let program_info = ProgramIDLInfo {
                        program_id,
                        account_count: accounts.len(),
                        instruction_count: generated_idl.idl.instructions.len(),
                        account_types: generated_idl.idl.accounts.len(),
                        confidence: generated_idl.confidence.overall_confidence,
                        analysis_timestamp: chrono::Utc::now(),
                    };

                    info!("     ✅ Program {}: {} instructions, {} account types ({:.1}% confidence)",
                          program_id,
                          program_info.instruction_count,
                          program_info.account_types,
                          program_info.confidence * 100.0);

                    program_idls.push(program_info);
                },
                Err(e) => {
                    warn!("     ❌ Failed to analyze program {}: {}", program_id, e);
                }
            }
        }

        Ok(program_idls)
    }

    /// Execute analytical queries using distributed DuckDB
    async fn execute_analytical_queries(
        &self,
        reconstructed_accounts: &[ReconstructedAccountInfo],
        program_idls: &[ProgramIDLInfo],
        queries: Vec<String>,
    ) -> Result<Vec<QueryResult>, Box<dyn std::error::Error + Send + Sync>> {

        let mut results = Vec::new();

        // First, insert data into analytical tables
        self.populate_analytical_tables(reconstructed_accounts, program_idls).await?;

        // Execute each analytical query
        for (i, sql) in queries.iter().enumerate() {
            info!("   Executing query {}/{}", i + 1, queries.len());

            let query = Query {
                sql: sql.clone(),
            };

            match self.duckdb_coordinator.execute_query(query).await {
                Ok(result) => {
                    info!("     ✅ Query {} completed: {} rows returned", i + 1, result.rows.len());
                    results.push(QueryResult {
                        query_id: i,
                        sql: sql.clone(),
                        rows_returned: result.rows.len(),
                        execution_time: result.execution_metadata
                            .map(|m| Duration::from_millis(m.execution_time_ms))
                            .unwrap_or_default(),
                        success: true,
                    });
                },
                Err(e) => {
                    warn!("     ❌ Query {} failed: {}", i + 1, e);
                    results.push(QueryResult {
                        query_id: i,
                        sql: sql.clone(),
                        rows_returned: 0,
                        execution_time: Duration::ZERO,
                        success: false,
                    });
                }
            }
        }

        Ok(results)
    }

    /// Populate analytical tables with reconstructed data
    async fn populate_analytical_tables(
        &self,
        accounts: &[ReconstructedAccountInfo],
        programs: &[ProgramIDLInfo],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

        // Create accounts table
        let accounts_data = accounts.iter()
            .map(|acc| format!(
                "('{}', '{}', {}, {}, {}, {:.3}, '{}')",
                acc.account_id,
                acc.program_id,
                acc.slot,
                acc.original_size,
                acc.reconstructed_size,
                acc.confidence,
                acc.reconstruction_method
            ))
            .collect::<Vec<_>>()
            .join(", ");

        if !accounts_data.is_empty() {
            let create_accounts_table = Query {
                sql: format!(
                    "CREATE OR REPLACE TABLE accounts AS SELECT * FROM VALUES {} AS t(account_id, program_id, slot, original_size, reconstructed_size, confidence, method)",
                    accounts_data
                ),
            };

            self.duckdb_coordinator.execute_query(create_accounts_table).await?;
        }

        // Create programs table
        let programs_data = programs.iter()
            .map(|prog| format!(
                "('{}', {}, {}, {}, {:.3})",
                prog.program_id,
                prog.account_count,
                prog.instruction_count,
                prog.account_types,
                prog.confidence
            ))
            .collect::<Vec<_>>()
            .join(", ");

        if !programs_data.is_empty() {
            let create_programs_table = Query {
                sql: format!(
                    "CREATE OR REPLACE TABLE programs AS SELECT * FROM VALUES {} AS t(program_id, account_count, instruction_count, account_types, confidence)",
                    programs_data
                ),
            };

            self.duckdb_coordinator.execute_query(create_programs_table).await?;
        }

        Ok(())
    }

    /// Generate insights report
    async fn generate_insights_report(
        &self,
        accounts: &[ReconstructedAccountInfo],
        programs: &[ProgramIDLInfo],
        queries: &[QueryResult],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

        info!("📊 Generating comprehensive insights report");

        // Account reconstruction insights
        let total_accounts = accounts.len();
        let avg_confidence = if total_accounts > 0 {
            accounts.iter().map(|a| a.confidence).sum::<f64>() / total_accounts as f64
        } else {
            0.0
        };

        let avg_compression_ratio = if total_accounts > 0 {
            accounts.iter()
                .map(|a| a.reconstructed_size as f64 / a.original_size.max(1) as f64)
                .sum::<f64>() / total_accounts as f64
        } else {
            1.0
        };

        info!("   🏛️ Account Reconstruction Insights:");
        info!("     - Total accounts processed: {}", total_accounts);
        info!("     - Average reconstruction confidence: {:.1}%", avg_confidence * 100.0);
        info!("     - Average expansion ratio: {:.2}x", avg_compression_ratio);

        // Program analysis insights
        let total_programs = programs.len();
        let avg_program_confidence = if total_programs > 0 {
            programs.iter().map(|p| p.confidence).sum::<f64>() / total_programs as f64
        } else {
            0.0
        };

        let total_instructions: usize = programs.iter().map(|p| p.instruction_count).sum();
        let total_account_types: usize = programs.iter().map(|p| p.account_types).sum();

        info!("   🔄 Program Analysis Insights:");
        info!("     - Total programs analyzed: {}", total_programs);
        info!("     - Average IDL confidence: {:.1}%", avg_program_confidence * 100.0);
        info!("     - Total instructions detected: {}", total_instructions);
        info!("     - Total account types detected: {}", total_account_types);

        // Query performance insights
        let successful_queries = queries.iter().filter(|q| q.success).count();
        let avg_query_time = if !queries.is_empty() {
            queries.iter()
                .map(|q| q.execution_time.as_millis() as f64)
                .sum::<f64>() / queries.len() as f64
        } else {
            0.0
        };

        info!("   🗄️ Query Performance Insights:");
        info!("     - Query success rate: {:.1}% ({}/{})",
              (successful_queries as f64 / queries.len().max(1) as f64) * 100.0,
              successful_queries, queries.len());
        info!("     - Average query time: {:.1}ms", avg_query_time);

        // Performance recommendations
        info!("   💡 Performance Recommendations:");
        if avg_confidence < 0.8 {
            warn!("     - Consider improving reconstruction patterns (confidence < 80%)");
        }
        if avg_program_confidence < 0.7 {
            warn!("     - Consider increasing transaction sample size for IDL analysis");
        }
        if avg_query_time > 100.0 {
            warn!("     - Consider optimizing query performance or adding indexes");
        }

        Ok(())
    }

    /// Generate synthetic transaction history from account data
    fn generate_transaction_history(&self, accounts: &[&ReconstructedAccountInfo]) -> Vec<Vec<u8>> {
        let mut transactions = Vec::new();

        for account in accounts {
            // Create synthetic transaction based on account characteristics
            let mut tx_data = Vec::new();

            // Mock signature
            tx_data.extend_from_slice(&[0x42; 64]);

            // Instruction discriminator based on account size
            let instruction_type = match account.reconstructed_size {
                0..=100 => 0x01,      // Small account operations
                101..=1000 => 0x02,   // Medium account operations
                _ => 0x03,            // Large account operations
            };
            tx_data.push(instruction_type);

            // Parameters based on account data
            tx_data.extend_from_slice(&account.slot.to_le_bytes());

            // Account keys
            tx_data.extend_from_slice(&account.account_id.to_bytes());
            tx_data.extend_from_slice(&account.program_id.to_bytes());

            transactions.push(tx_data);
        }

        transactions
    }
}

/// Compressed account data input
#[derive(Debug, Clone)]
pub struct CompressedAccountData {
    pub account_id: Pubkey,
    pub program_id: Pubkey,
    pub slot: u64,
    pub compressed_data: Vec<u8>,
    pub compression_type: CompressionType,
    pub merkle_tree_height: u32,
    pub compression_level: u32,
}

/// Reconstructed account information
#[derive(Debug)]
pub struct ReconstructedAccountInfo {
    pub account_id: Pubkey,
    pub program_id: Pubkey,
    pub slot: u64,
    pub original_size: usize,
    pub reconstructed_size: usize,
    pub confidence: f64,
    pub reconstruction_method: String,
    pub reconstruction_time: Duration,
    pub account_data: Vec<u8>,
}

/// Program IDL information
#[derive(Debug)]
pub struct ProgramIDLInfo {
    pub program_id: Pubkey,
    pub account_count: usize,
    pub instruction_count: usize,
    pub account_types: usize,
    pub confidence: f64,
    pub analysis_timestamp: chrono::DateTime<chrono::Utc>,
}

/// Query execution result
#[derive(Debug)]
pub struct QueryResult {
    pub query_id: usize,
    pub sql: String,
    pub rows_returned: usize,
    pub execution_time: Duration,
    pub success: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("🌊 StreamSync End-to-End Pipeline Example");

    // Create pipeline
    let pipeline = StreamSyncPipeline::new();

    // Generate sample data
    let sample_data = generate_sample_compressed_data(20);
    let analysis_queries = generate_sample_queries();

    // Process the complete pipeline
    match pipeline.process_pipeline(sample_data, analysis_queries).await {
        Ok(results) => {
            info!("🎉 Pipeline completed successfully!");
            info!("   📊 Final Results:");
            info!("     - Accounts processed: {}", results.accounts_processed);
            info!("     - Programs analyzed: {}", results.programs_analyzed);
            info!("     - Queries executed: {}", results.queries_executed);
            info!("     - Total time: {:?}", results.total_processing_time);
            info!("     - Success rate: {:.1}%", results.success_rate * 100.0);
        },
        Err(e) => {
            error!("❌ Pipeline failed: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

/// Generate sample compressed account data for testing
fn generate_sample_compressed_data(count: usize) -> Vec<CompressedAccountData> {
    let mut data = Vec::new();

    for i in 0..count {
        let compressed_data = generate_compressed_bytes(200 + i * 50);

        data.push(CompressedAccountData {
            account_id: Pubkey::new_unique(),
            program_id: if i < count / 2 {
                // First half uses same program to test IDL analysis
                Pubkey::from([1u8; 32])
            } else {
                Pubkey::new_unique()
            },
            slot: 1000 + i as u64,
            compressed_data,
            compression_type: match i % 3 {
                0 => CompressionType::Standard,
                1 => CompressionType::StateCompression,
                _ => CompressionType::Custom("test".to_string()),
            },
            merkle_tree_height: 10 + (i % 10) as u32,
            compression_level: 3 + (i % 6) as u32,
        });
    }

    data
}

/// Generate compressed-looking bytes
fn generate_compressed_bytes(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);

    for i in 0..size {
        let byte = match i % 8 {
            0..=2 => 0xFF,                    // Header pattern
            3..=5 => (i % 256) as u8,         // Data
            6 => 0x00,                       // Separator
            7 => ((i * 17) % 256) as u8,     // Checksum-like
            _ => unreachable!(),
        };
        data.push(byte);
    }

    data
}

/// Generate sample analytical queries
fn generate_sample_queries() -> Vec<String> {
    vec![
        "SELECT COUNT(*) as total_accounts FROM accounts".to_string(),
        "SELECT program_id, COUNT(*) as account_count FROM accounts GROUP BY program_id ORDER BY account_count DESC".to_string(),
        "SELECT AVG(confidence) as avg_confidence, method FROM accounts GROUP BY method".to_string(),
        "SELECT program_id, AVG(confidence) as avg_idl_confidence FROM programs WHERE confidence > 0.8 GROUP BY program_id".to_string(),
        "SELECT slot, COUNT(*) as accounts_in_slot FROM accounts GROUP BY slot ORDER BY slot DESC LIMIT 10".to_string(),
    ]
}