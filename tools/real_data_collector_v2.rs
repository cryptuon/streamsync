//! Real Solana Blockchain Data Collector (Simplified)
//!
//! Fetches actual transaction data from Solana RPC endpoints
//! using a simplified approach that works with current APIs

use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_transaction_status::{UiTransactionEncoding, EncodedConfirmedTransactionWithStatusMeta};
use serde::{Serialize, Deserialize};
use anyhow::{Result, Context};
use tracing::{info, warn};
use std::collections::HashMap;
use std::fs;
use std::time::{SystemTime, Duration};

/// Simplified real transaction data from Solana blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimplifiedRealTransaction {
    pub signature: String,
    pub slot: u64,
    pub success: bool,
    pub fee: u64,
    pub accounts: Vec<String>,
    pub pre_balances: Vec<u64>,
    pub post_balances: Vec<u64>,
    pub log_messages: Vec<String>,
    pub program_interactions: Vec<SimpleProgramInteraction>,
    pub compute_units_consumed: Option<u64>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleProgramInteraction {
    pub program_id: String,
    pub program_name: String,
    pub instruction_data_size: usize,
    pub is_target_program: bool,
    pub is_state_compression: bool,
}

/// Collection results
#[derive(Debug, Serialize, Deserialize)]
pub struct SimplifiedCollectionStats {
    pub collection_start_time: SystemTime,
    pub collection_end_time: SystemTime,
    pub rpc_endpoint: String,
    pub signatures_processed: usize,
    pub successful_fetches: usize,
    pub failed_fetches: usize,
    pub transactions_with_target_programs: usize,
    pub program_distribution: HashMap<String, usize>,
    pub success_rate: f64,
    pub compression_transactions: usize,
}

/// Simplified real data collector
pub struct SimplifiedRealCollector {
    rpc_client: RpcClient,
    target_programs: HashMap<String, String>,
    stats: SimplifiedCollectionStats,
}

impl SimplifiedRealCollector {
    /// Create new collector
    pub fn new(rpc_endpoint: impl Into<String>) -> Self {
        let endpoint = rpc_endpoint.into();
        let rpc_client = RpcClient::new_with_timeout_and_commitment(
            endpoint.clone(),
            Duration::from_secs(30),
            CommitmentConfig::confirmed(),
        );

        let mut target_programs = HashMap::new();
        target_programs.insert("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(), "SPL Token".to_string());
        target_programs.insert("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s".to_string(), "Metaplex".to_string());
        target_programs.insert("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string(), "Jupiter Aggregator".to_string());
        target_programs.insert("BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY".to_string(), "Metaplex Bubblegum".to_string());
        target_programs.insert("cmtDvXumGCrqC1Age74AVPhSRVXJMd8PJS91L8KbNCK".to_string(), "Account Compression".to_string());

        let stats = SimplifiedCollectionStats {
            collection_start_time: SystemTime::now(),
            collection_end_time: SystemTime::now(),
            rpc_endpoint: endpoint,
            signatures_processed: 0,
            successful_fetches: 0,
            failed_fetches: 0,
            transactions_with_target_programs: 0,
            program_distribution: HashMap::new(),
            success_rate: 0.0,
            compression_transactions: 0,
        };

        Self {
            rpc_client,
            target_programs,
            stats,
        }
    }

    /// Collect real transaction data using confirmed transaction signatures
    pub async fn collect_real_data(&mut self, max_transactions: usize) -> Result<Vec<SimplifiedRealTransaction>> {
        info!("🚀 Starting simplified real data collection");
        info!("🔗 RPC Endpoint: {}", self.stats.rpc_endpoint);

        self.stats.collection_start_time = SystemTime::now();
        let mut collected_transactions = Vec::new();

        // Get recent confirmed signatures
        let signatures = self.get_recent_signatures(max_transactions * 2).await?;
        info!("🔍 Found {} recent signatures", signatures.len());

        // Process each signature
        for (i, signature) in signatures.iter().enumerate() {
            if collected_transactions.len() >= max_transactions {
                break;
            }

            self.stats.signatures_processed += 1;

            match self.fetch_and_parse_transaction(signature).await {
                Ok(Some(transaction)) => {
                    if self.is_target_transaction(&transaction) {
                        collected_transactions.push(transaction);
                        self.stats.transactions_with_target_programs += 1;
                    }
                    self.stats.successful_fetches += 1;
                }
                Ok(None) => {
                    // Transaction didn't contain target programs
                    self.stats.successful_fetches += 1;
                }
                Err(e) => {
                    self.stats.failed_fetches += 1;
                    warn!("Failed to fetch transaction {}: {}", signature, e);
                }
            }

            // Progress update
            if i % 50 == 0 && i > 0 {
                info!("📈 Progress: {}/{} signatures processed, {} target transactions found",
                      i, signatures.len(), collected_transactions.len());
            }

            // Rate limiting
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        self.stats.collection_end_time = SystemTime::now();
        self.calculate_stats(&collected_transactions);
        self.print_summary();

        info!("✅ Real data collection completed: {} transactions", collected_transactions.len());

        Ok(collected_transactions)
    }

    /// Get recent confirmed transaction signatures
    async fn get_recent_signatures(&self, limit: usize) -> Result<Vec<String>> {
        info!("📝 Fetching recent transaction signatures...");

        // Get signatures for the most commonly used programs to increase hit rate
        let signatures = self.rpc_client
            .get_signatures_for_address_with_config(
                &"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".parse()?,
                GetConfirmedSignaturesForAddress2Config {
                    limit: Some(limit),
                    ..Default::default()
                },
            )
            .context("Failed to get signatures")?;

        Ok(signatures.into_iter().map(|sig| sig.signature).collect())
    }

    /// Fetch and parse a single transaction by signature
    async fn fetch_and_parse_transaction(&self, signature: &str) -> Result<Option<SimplifiedRealTransaction>> {
        let config = RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::JsonParsed),
            commitment: Some(CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
        };

        let transaction = self.rpc_client
            .get_transaction_with_config(&signature.parse()?, config)
            .context("Failed to fetch transaction")?;

        self.parse_transaction(signature, &transaction)
    }

    /// Parse transaction into our simplified format
    fn parse_transaction(
        &self,
        signature: &str,
        transaction: &EncodedConfirmedTransactionWithStatusMeta,
    ) -> Result<Option<SimplifiedRealTransaction>> {
        let slot = transaction.slot;

        let success = transaction.transaction.meta
            .as_ref()
            .map(|meta| meta.err.is_none())
            .unwrap_or(false);

        let fee = transaction.transaction.meta
            .as_ref()
            .map(|meta| meta.fee)
            .unwrap_or(0);

        let error_message = transaction.transaction.meta
            .as_ref()
            .and_then(|meta| meta.err.as_ref())
            .map(|err| format!("{:?}", err));

        // Extract accounts (simplified - just get the count)
        let accounts = Vec::new(); // Simplified for now

        let (pre_balances, post_balances) = if let Some(meta) = &transaction.transaction.meta {
            (meta.pre_balances.clone(), meta.post_balances.clone())
        } else {
            (Vec::new(), Vec::new())
        };

        // Extract log messages
        let log_messages = transaction.transaction.meta
            .as_ref()
            .and_then(|meta| {
                // Handle OptionSerializer properly
                match &meta.log_messages {
                    solana_transaction_status::option_serializer::OptionSerializer::Some(logs) => Some(logs.clone()),
                    _ => None,
                }
            })
            .unwrap_or_default();

        // Extract compute units
        let compute_units_consumed = transaction.transaction.meta
            .as_ref()
            .and_then(|meta| {
                match meta.compute_units_consumed {
                    solana_transaction_status::option_serializer::OptionSerializer::Some(units) => Some(units),
                    _ => None,
                }
            });

        // Analyze program interactions from logs
        let program_interactions = self.extract_program_interactions_from_logs(&log_messages);

        Ok(Some(SimplifiedRealTransaction {
            signature: signature.to_string(),
            slot,
            success,
            fee,
            accounts,
            pre_balances,
            post_balances,
            log_messages,
            program_interactions,
            compute_units_consumed,
            error_message,
        }))
    }

    /// Extract program interactions from log messages
    fn extract_program_interactions_from_logs(&self, logs: &[String]) -> Vec<SimpleProgramInteraction> {
        let mut interactions = Vec::new();

        for log in logs {
            // Look for program invocations in logs
            if log.starts_with("Program ") && log.contains(" invoke") {
                if let Some(program_id) = self.extract_program_id_from_log(log) {
                    if let Some(program_name) = self.target_programs.get(&program_id) {
                        let is_state_compression = matches!(program_id.as_str(),
                            "BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY" |
                            "cmtDvXumGCrqC1Age74AVPhSRVXJMd8PJS91L8KbNCK"
                        );

                        interactions.push(SimpleProgramInteraction {
                            program_id,
                            program_name: program_name.clone(),
                            instruction_data_size: 0, // Not available from logs
                            is_target_program: true,
                            is_state_compression,
                        });
                    }
                }
            }
        }

        interactions
    }

    /// Extract program ID from log message
    fn extract_program_id_from_log(&self, log: &str) -> Option<String> {
        // Parse log format: "Program <program_id> invoke [<depth>]"
        if let Some(start) = log.find("Program ") {
            let after_program = &log[start + 8..];
            if let Some(end) = after_program.find(" invoke") {
                let program_id = &after_program[..end];
                return Some(program_id.to_string());
            }
        }
        None
    }

    /// Check if transaction contains target programs
    fn is_target_transaction(&self, transaction: &SimplifiedRealTransaction) -> bool {
        !transaction.program_interactions.is_empty()
    }

    /// Calculate final statistics
    fn calculate_stats(&mut self, transactions: &[SimplifiedRealTransaction]) {
        let mut program_counts = HashMap::new();
        let mut compression_count = 0;

        for tx in transactions {
            for interaction in &tx.program_interactions {
                *program_counts.entry(interaction.program_name.clone()).or_insert(0) += 1;

                if interaction.is_state_compression {
                    compression_count += 1;
                }
            }
        }

        self.stats.program_distribution = program_counts;
        self.stats.success_rate = if self.stats.signatures_processed > 0 {
            self.stats.successful_fetches as f64 / self.stats.signatures_processed as f64
        } else {
            0.0
        };
        self.stats.compression_transactions = compression_count;
    }

    /// Print collection summary
    fn print_summary(&self) {
        let duration = self.stats.collection_end_time
            .duration_since(self.stats.collection_start_time)
            .unwrap_or(Duration::ZERO);

        info!("🏆 === REAL DATA COLLECTION SUMMARY ===");
        info!("⏱️ Collection Time: {:.2}s", duration.as_secs_f64());
        info!("📝 Signatures Processed: {}", self.stats.signatures_processed);
        info!("✅ Successful Fetches: {}", self.stats.successful_fetches);
        info!("❌ Failed Fetches: {}", self.stats.failed_fetches);
        info!("🎯 Target Transactions: {}", self.stats.transactions_with_target_programs);
        info!("📊 Success Rate: {:.1}%", self.stats.success_rate * 100.0);
        info!("🗜️ Compression Transactions: {}", self.stats.compression_transactions);

        info!("🏗️ Program Distribution:");
        for (program, count) in &self.stats.program_distribution {
            info!("   {}: {}", program, count);
        }
    }

    /// Save collected data
    pub fn save_data(&self, transactions: &[SimplifiedRealTransaction], output_dir: &str) -> Result<()> {
        fs::create_dir_all(output_dir)
            .context("Failed to create output directory")?;

        // Save transactions
        let transactions_file = format!("{}/real_solana_dataset.json", output_dir);
        let transactions_json = serde_json::to_string_pretty(transactions)
            .context("Failed to serialize transactions")?;
        fs::write(&transactions_file, transactions_json)
            .context("Failed to write transactions file")?;

        // Save statistics
        let stats_file = format!("{}/real_dataset_stats.json", output_dir);
        let stats_json = serde_json::to_string_pretty(&self.stats)
            .context("Failed to serialize statistics")?;
        fs::write(&stats_file, stats_json)
            .context("Failed to write statistics file")?;

        info!("💾 Real data saved to:");
        info!("   📄 Transactions: {}", transactions_file);
        info!("   📊 Statistics: {}", stats_file);

        Ok(())
    }
}

/// CLI entry point
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    info!("🚀 Starting Real Solana Data Collection (Simplified)");

    let rpc_endpoint = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

    let output_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "./real_collected_data".to_string());

    let max_transactions = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    info!("🔗 Using RPC endpoint: {}", rpc_endpoint);
    info!("📁 Output directory: {}", output_dir);
    info!("🎯 Target transactions: {}", max_transactions);

    let mut collector = SimplifiedRealCollector::new(rpc_endpoint);
    let transactions = collector.collect_real_data(max_transactions).await?;

    collector.save_data(&transactions, &output_dir)?;

    info!("✅ Real Solana data collection completed!");
    info!("🎯 Collected {} real transactions from Solana blockchain", transactions.len());

    Ok(())
}