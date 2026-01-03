//! Real Solana Blockchain Data Collector
//!
//! Fetches actual transaction data from Solana RPC endpoints
//! for comprehensive testing of StreamSync libraries

use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcBlockConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_transaction_status::{UiTransactionEncoding, TransactionDetails, EncodedTransaction, UiMessage, UiInstruction, UiParsedInstruction};
use serde::{Serialize, Deserialize};
use anyhow::{Result, Context};
use tracing::{info, warn};
use std::collections::HashMap;
use std::fs;
use std::time::{SystemTime, Duration};

/// Real transaction data collected from Solana blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealSolanaTransaction {
    pub signature: String,
    pub slot: u64,
    pub block_time: Option<i64>,
    pub program_interactions: Vec<ProgramInteraction>,
    pub accounts: Vec<String>,
    pub pre_balances: Vec<u64>,
    pub post_balances: Vec<u64>,
    pub log_messages: Vec<String>,
    pub compute_units_consumed: Option<u64>,
    pub success: bool,
    pub error_message: Option<String>,
    pub fee: u64,
    pub recent_blockhash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramInteraction {
    pub program_id: String,
    pub program_name: String,
    pub instruction_data: Vec<u8>,
    pub instruction_index: u8,
    pub account_indices: Vec<u8>,
    pub is_state_compression: bool,
    pub compression_data: Option<CompressionData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionData {
    pub merkle_tree: Option<String>,
    pub compressed_accounts: Vec<String>,
    pub compression_proof: Vec<String>,
    pub leaf_index: Option<u32>,
    pub tree_height: Option<u8>,
}

/// Collection statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct RealDataStats {
    pub collection_start_time: SystemTime,
    pub collection_end_time: SystemTime,
    pub rpc_endpoint: String,
    pub total_blocks_scanned: u64,
    pub total_transactions_collected: usize,
    pub transactions_per_program: HashMap<String, usize>,
    pub success_rate: f64,
    pub average_transaction_size: f64,
    pub compression_transaction_count: usize,
    pub unique_merkle_trees: Vec<String>,
    pub data_quality_metrics: DataQualityMetrics,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataQualityMetrics {
    pub complete_transactions: usize,
    pub incomplete_transactions: usize,
    pub parse_errors: usize,
    pub rpc_failures: usize,
    pub compression_coverage: f64,
    pub program_diversity: usize,
}

/// Real Solana blockchain data collector
pub struct RealDataCollector {
    rpc_client: RpcClient,
    target_programs: HashMap<String, String>, // program_id -> program_name
    collection_config: CollectionConfig,
    stats: RealDataStats,
}

#[derive(Debug)]
pub struct CollectionConfig {
    pub max_transactions: usize,
    pub max_blocks_to_scan: u64,
    pub target_slot_range: Option<(u64, u64)>,
    pub include_failed_transactions: bool,
    pub focus_on_compression: bool,
    pub timeout_per_request: Duration,
}

impl Default for CollectionConfig {
    fn default() -> Self {
        Self {
            max_transactions: 1000,
            max_blocks_to_scan: 50,
            target_slot_range: None,
            include_failed_transactions: true,
            focus_on_compression: true,
            timeout_per_request: Duration::from_secs(10),
        }
    }
}

impl RealDataCollector {
    /// Create a new real data collector
    pub fn new(rpc_endpoint: impl Into<String>, config: Option<CollectionConfig>) -> Self {
        let endpoint = rpc_endpoint.into();
        let rpc_client = RpcClient::new_with_timeout_and_commitment(
            endpoint.clone(),
            Duration::from_secs(30),
            CommitmentConfig::confirmed(),
        );

        let mut target_programs = HashMap::new();

        // Key Solana programs we want to collect data from
        target_programs.insert("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(), "SPL Token".to_string());
        target_programs.insert("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s".to_string(), "Metaplex".to_string());
        target_programs.insert("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string(), "Jupiter Aggregator".to_string());
        target_programs.insert("BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY".to_string(), "Metaplex Bubblegum".to_string());
        target_programs.insert("cmtDvXumGCrqC1Age74AVPhSRVXJMd8PJS91L8KbNCK".to_string(), "Account Compression".to_string());
        target_programs.insert("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV".to_string(), "Noop Program".to_string());

        let stats = RealDataStats {
            collection_start_time: SystemTime::now(),
            collection_end_time: SystemTime::now(),
            rpc_endpoint: endpoint,
            total_blocks_scanned: 0,
            total_transactions_collected: 0,
            transactions_per_program: HashMap::new(),
            success_rate: 0.0,
            average_transaction_size: 0.0,
            compression_transaction_count: 0,
            unique_merkle_trees: Vec::new(),
            data_quality_metrics: DataQualityMetrics {
                complete_transactions: 0,
                incomplete_transactions: 0,
                parse_errors: 0,
                rpc_failures: 0,
                compression_coverage: 0.0,
                program_diversity: 0,
            },
        };

        Self {
            rpc_client,
            target_programs,
            collection_config: config.unwrap_or_default(),
            stats,
        }
    }

    /// Collect real transaction data from Solana blockchain
    pub async fn collect_real_data(&mut self) -> Result<Vec<RealSolanaTransaction>> {
        info!("🚀 Starting real Solana blockchain data collection");
        info!("🔗 RPC Endpoint: {}", self.stats.rpc_endpoint);
        info!("🎯 Target Programs: {:?}", self.target_programs.keys().collect::<Vec<_>>());

        self.stats.collection_start_time = SystemTime::now();
        let mut collected_transactions = Vec::new();

        // Get current slot to work backwards from
        let current_slot = self.get_current_slot().await?;
        info!("📍 Current slot: {}", current_slot);

        let start_slot = if let Some((start, _end)) = self.collection_config.target_slot_range {
            std::cmp::min(start, current_slot)
        } else {
            current_slot.saturating_sub(self.collection_config.max_blocks_to_scan)
        };

        let end_slot = if let Some((_, end)) = self.collection_config.target_slot_range {
            std::cmp::min(end, current_slot)
        } else {
            current_slot
        };

        info!("🔍 Scanning slots {} to {} ({} blocks)", start_slot, end_slot, end_slot - start_slot);

        // Collect data from recent blocks
        for slot in (start_slot..=end_slot).rev() {
            if collected_transactions.len() >= self.collection_config.max_transactions {
                info!("✅ Reached target transaction count: {}", collected_transactions.len());
                break;
            }

            if self.stats.total_blocks_scanned >= self.collection_config.max_blocks_to_scan {
                info!("✅ Reached max blocks to scan: {}", self.stats.total_blocks_scanned);
                break;
            }

            match self.collect_from_block(slot).await {
                Ok(mut block_transactions) => {
                    let block_count = block_transactions.len();
                    if block_count > 0 {
                        info!("📦 Block {}: collected {} transactions", slot, block_count);
                        collected_transactions.append(&mut block_transactions);
                    }
                }
                Err(e) => {
                    self.stats.data_quality_metrics.rpc_failures += 1;
                    warn!("⚠️ Failed to collect from block {}: {}", slot, e);

                    // Continue with next block instead of failing completely
                    continue;
                }
            }

            self.stats.total_blocks_scanned += 1;

            // Rate limiting to avoid overwhelming RPC
            tokio::time::sleep(Duration::from_millis(100)).await;

            // Progress update every 10 blocks
            if self.stats.total_blocks_scanned % 10 == 0 {
                info!("📈 Progress: {} blocks scanned, {} transactions collected",
                      self.stats.total_blocks_scanned, collected_transactions.len());
            }
        }

        self.stats.collection_end_time = SystemTime::now();
        self.stats.total_transactions_collected = collected_transactions.len();

        self.calculate_final_stats(&collected_transactions);
        self.print_collection_summary();

        info!("✅ Real data collection completed: {} transactions", collected_transactions.len());

        Ok(collected_transactions)
    }

    /// Get current slot from RPC
    async fn get_current_slot(&self) -> Result<u64> {
        let slot = self.rpc_client.get_slot()
            .context("Failed to get current slot from RPC")?;
        Ok(slot)
    }

    /// Collect transactions from a specific block
    async fn collect_from_block(&mut self, slot: u64) -> Result<Vec<RealSolanaTransaction>> {
        let block_config = RpcBlockConfig {
            encoding: Some(UiTransactionEncoding::JsonParsed),
            transaction_details: Some(TransactionDetails::Full),
            rewards: Some(false),
            commitment: Some(CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
        };

        let block = self.rpc_client.get_block_with_config(slot, block_config)
            .context("Failed to fetch block")?;

        let mut block_transactions = Vec::new();
        let block_time = block.block_time;

        if let Some(transactions) = block.transactions {
            for (tx_index, ui_transaction) in transactions.iter().enumerate() {
                match self.parse_transaction(ui_transaction, slot, block_time, tx_index).await {
                    Ok(Some(parsed_tx)) => {
                        // Filter for our target programs
                        if self.is_target_transaction(&parsed_tx) {
                            block_transactions.push(parsed_tx);
                        }
                    }
                    Ok(None) => {
                        // Transaction didn't match our criteria, skip silently
                    }
                    Err(e) => {
                        self.stats.data_quality_metrics.parse_errors += 1;
                        warn!("Failed to parse transaction {}: {}", tx_index, e);
                    }
                }
            }
        }

        Ok(block_transactions)
    }

    /// Parse a UI transaction into our format
    async fn parse_transaction(
        &self,
        ui_transaction: &solana_transaction_status::EncodedTransactionWithStatusMeta,
        slot: u64,
        block_time: Option<i64>,
        _tx_index: usize,
    ) -> Result<Option<RealSolanaTransaction>> {
        // Extract transaction signature using pattern matching on EncodedTransaction enum
        let signature = match &ui_transaction.transaction {
            EncodedTransaction::Json(ui_tx) => ui_tx.signatures.first()
                .cloned()
                .unwrap_or_else(|| "unknown".to_string()),
            EncodedTransaction::Accounts(ui_tx) => ui_tx.signatures.first()
                .cloned()
                .unwrap_or_else(|| "unknown".to_string()),
            _ => return Ok(None),
        };

        if signature == "unknown" {
            return Ok(None);
        }

        // Check if transaction succeeded
        let success = ui_transaction.meta.as_ref()
            .map(|meta| meta.err.is_none())
            .unwrap_or(false);

        // Skip failed transactions if not configured to include them
        if !success && !self.collection_config.include_failed_transactions {
            return Ok(None);
        }

        // Extract basic transaction info
        let fee = ui_transaction.meta.as_ref()
            .map(|meta| meta.fee)
            .unwrap_or(0);

        let error_message = ui_transaction.meta.as_ref()
            .and_then(|meta| meta.err.as_ref())
            .map(|err| format!("{:?}", err));

        // Extract accounts and balances using pattern matching on both EncodedTransaction and UiMessage
        let accounts = match &ui_transaction.transaction {
            EncodedTransaction::Json(ui_tx) => {
                match &ui_tx.message {
                    UiMessage::Parsed(parsed_msg) => {
                        parsed_msg.account_keys.iter().map(|key| key.pubkey.clone()).collect()
                    }
                    UiMessage::Raw(raw_msg) => {
                        raw_msg.account_keys.clone()
                    }
                }
            }
            _ => Vec::new(),
        };

        let (pre_balances, post_balances) = if let Some(meta) = &ui_transaction.meta {
            (meta.pre_balances.clone(), meta.post_balances.clone())
        } else {
            (Vec::new(), Vec::new())
        };

        // Extract log messages (handle OptionSerializer)
        let log_messages = ui_transaction.meta.as_ref()
            .and_then(|meta| Option::<Vec<String>>::from(meta.log_messages.clone()))
            .unwrap_or_default();

        // Extract compute units (handle OptionSerializer)
        let compute_units_consumed = ui_transaction.meta.as_ref()
            .and_then(|meta| Option::<u64>::from(meta.compute_units_consumed.clone()));

        // Extract recent blockhash using pattern matching on both EncodedTransaction and UiMessage
        let recent_blockhash = match &ui_transaction.transaction {
            EncodedTransaction::Json(ui_tx) => {
                match &ui_tx.message {
                    UiMessage::Parsed(parsed_msg) => parsed_msg.recent_blockhash.clone(),
                    UiMessage::Raw(raw_msg) => raw_msg.recent_blockhash.clone(),
                }
            }
            _ => String::new(),
        };

        // Parse program interactions
        let program_interactions = self.extract_program_interactions(ui_transaction).await?;

        // Skip if no relevant program interactions
        if program_interactions.is_empty() {
            return Ok(None);
        }

        Ok(Some(RealSolanaTransaction {
            signature,
            slot,
            block_time,
            program_interactions,
            accounts,
            pre_balances,
            post_balances,
            log_messages,
            compute_units_consumed,
            success,
            error_message,
            fee,
            recent_blockhash,
        }))
    }

    /// Extract program interactions from transaction
    async fn extract_program_interactions(
        &self,
        ui_transaction: &solana_transaction_status::EncodedTransactionWithStatusMeta,
    ) -> Result<Vec<ProgramInteraction>> {
        let mut interactions = Vec::new();

        // Pattern match on EncodedTransaction to access message
        let ui_tx = match &ui_transaction.transaction {
            EncodedTransaction::Json(ui_tx) => ui_tx,
            _ => return Ok(interactions),
        };

        // Get account keys and instructions based on UiMessage variant
        let (account_keys, instructions): (Vec<String>, Vec<_>) = match &ui_tx.message {
            UiMessage::Parsed(parsed_msg) => {
                let keys: Vec<String> = parsed_msg.account_keys.iter()
                    .map(|key| key.pubkey.clone())
                    .collect();
                // For parsed messages, instructions can be Parsed or Compiled
                let instrs: Vec<(String, Vec<u8>, Vec<String>)> = parsed_msg.instructions.iter()
                    .filter_map(|instr| {
                        match instr {
                            UiInstruction::Parsed(ui_parsed) => {
                                // UiParsedInstruction is also an enum
                                match ui_parsed {
                                    UiParsedInstruction::Parsed(parsed) => {
                                        let program_id = parsed.program_id.clone();
                                        // Parsed instructions don't have raw data easily accessible
                                        let data = Vec::new();
                                        let accounts = Vec::new();
                                        Some((program_id, data, accounts))
                                    }
                                    UiParsedInstruction::PartiallyDecoded(partial) => {
                                        let program_id = partial.program_id.clone();
                                        let data = bs58::decode(&partial.data).into_vec().unwrap_or_default();
                                        let accounts = partial.accounts.clone();
                                        Some((program_id, data, accounts))
                                    }
                                }
                            }
                            UiInstruction::Compiled(compiled) => {
                                let program_idx = compiled.program_id_index as usize;
                                keys.get(program_idx).map(|prog_id| {
                                    let data = bs58::decode(&compiled.data).into_vec().unwrap_or_default();
                                    let accounts: Vec<String> = compiled.accounts.iter()
                                        .filter_map(|&idx| keys.get(idx as usize).cloned())
                                        .collect();
                                    (prog_id.clone(), data, accounts)
                                })
                            }
                        }
                    })
                    .collect();
                (keys, instrs)
            }
            UiMessage::Raw(raw_msg) => {
                let keys = raw_msg.account_keys.clone();
                // For raw messages, we need to decode instruction data
                let instrs: Vec<(String, Vec<u8>, Vec<String>)> = raw_msg.instructions.iter()
                    .filter_map(|instr| {
                        let program_idx = instr.program_id_index as usize;
                        keys.get(program_idx).map(|prog_id| {
                            let data = bs58::decode(&instr.data).into_vec().unwrap_or_default();
                            let accounts: Vec<String> = instr.accounts.iter()
                                .filter_map(|&idx| keys.get(idx as usize).cloned())
                                .collect();
                            (prog_id.clone(), data, accounts)
                        })
                    })
                    .collect();
                (keys, instrs)
            }
        };

        for (idx, (program_id_str, instruction_data, accounts)) in instructions.into_iter().enumerate() {
            // Check if this is one of our target programs
            if let Some(program_name) = self.target_programs.get(&program_id_str) {
                // Get account indices from the instruction
                let account_indices: Vec<u8> = accounts.iter()
                    .filter_map(|acc| {
                        account_keys.iter().position(|k| k == acc).map(|i| i as u8)
                    })
                    .collect();

                // Check for state compression indicators
                let is_state_compression = self.detect_state_compression(
                    &program_id_str,
                    &instruction_data,
                    &ui_transaction.meta
                );

                let compression_data = if is_state_compression {
                    self.extract_compression_data(ui_transaction).await?
                } else {
                    None
                };

                interactions.push(ProgramInteraction {
                    program_id: program_id_str,
                    program_name: program_name.clone(),
                    instruction_data,
                    instruction_index: idx as u8,
                    account_indices,
                    is_state_compression,
                    compression_data,
                });
            }
        }

        Ok(interactions)
    }

    /// Detect state compression based on program and context
    fn detect_state_compression(
        &self,
        program_id: &str,
        _instruction_data: &[u8],
        _meta: &Option<solana_transaction_status::UiTransactionStatusMeta>,
    ) -> bool {
        // Known compression programs
        matches!(program_id,
            "BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY" | // Metaplex Bubblegum
            "cmtDvXumGCrqC1Age74AVPhSRVXJMd8PJS91L8KbNCK"   // SPL Account Compression
        )
    }

    /// Extract compression-specific data
    async fn extract_compression_data(
        &self,
        _ui_transaction: &solana_transaction_status::EncodedTransactionWithStatusMeta,
    ) -> Result<Option<CompressionData>> {
        // This would require more sophisticated parsing of compression-specific accounts
        // For now, return a placeholder structure
        Ok(Some(CompressionData {
            merkle_tree: None,
            compressed_accounts: Vec::new(),
            compression_proof: Vec::new(),
            leaf_index: None,
            tree_height: None,
        }))
    }

    /// Check if transaction contains our target programs
    fn is_target_transaction(&self, transaction: &RealSolanaTransaction) -> bool {
        transaction.program_interactions.iter()
            .any(|interaction| self.target_programs.contains_key(&interaction.program_id))
    }

    /// Calculate final collection statistics
    fn calculate_final_stats(&mut self, transactions: &[RealSolanaTransaction]) {
        let mut program_counts = HashMap::new();
        let mut successful_count = 0;
        let mut total_size = 0usize;
        let mut compression_count = 0;
        let mut merkle_trees = std::collections::HashSet::new();

        for tx in transactions {
            if tx.success {
                successful_count += 1;
            }

            // Calculate transaction size (approximate)
            total_size += tx.signature.len() + tx.accounts.len() * 44 + tx.log_messages.join("").len();

            for interaction in &tx.program_interactions {
                *program_counts.entry(interaction.program_name.clone()).or_insert(0) += 1;

                if interaction.is_state_compression {
                    compression_count += 1;

                    if let Some(compression_data) = &interaction.compression_data {
                        if let Some(tree) = &compression_data.merkle_tree {
                            merkle_trees.insert(tree.clone());
                        }
                    }
                }
            }
        }

        self.stats.transactions_per_program = program_counts;
        self.stats.success_rate = if transactions.len() > 0 {
            successful_count as f64 / transactions.len() as f64
        } else {
            0.0
        };
        self.stats.average_transaction_size = if transactions.len() > 0 {
            total_size as f64 / transactions.len() as f64
        } else {
            0.0
        };
        self.stats.compression_transaction_count = compression_count;
        self.stats.unique_merkle_trees = merkle_trees.into_iter().collect();

        // Update data quality metrics
        self.stats.data_quality_metrics.complete_transactions = transactions.len();
        self.stats.data_quality_metrics.compression_coverage = if transactions.len() > 0 {
            compression_count as f64 / transactions.len() as f64
        } else {
            0.0
        };
        self.stats.data_quality_metrics.program_diversity = self.stats.transactions_per_program.len();
    }

    /// Print collection summary
    fn print_collection_summary(&self) {
        let duration = self.stats.collection_end_time
            .duration_since(self.stats.collection_start_time)
            .unwrap_or(Duration::ZERO);

        info!("🏆 === REAL DATA COLLECTION SUMMARY ===");
        info!("⏱️ Collection Time: {:.2}s", duration.as_secs_f64());
        info!("📊 Total Transactions: {}", self.stats.total_transactions_collected);
        info!("🧱 Blocks Scanned: {}", self.stats.total_blocks_scanned);
        info!("✅ Success Rate: {:.1}%", self.stats.success_rate * 100.0);
        info!("📦 Average Transaction Size: {:.1} bytes", self.stats.average_transaction_size);
        info!("🗜️ Compression Transactions: {}", self.stats.compression_transaction_count);
        info!("🌳 Unique Merkle Trees: {}", self.stats.unique_merkle_trees.len());

        info!("🏗️ Program Distribution:");
        for (program, count) in &self.stats.transactions_per_program {
            info!("   {}: {}", program, count);
        }

        info!("📈 Data Quality:");
        info!("   Complete Transactions: {}", self.stats.data_quality_metrics.complete_transactions);
        info!("   Parse Errors: {}", self.stats.data_quality_metrics.parse_errors);
        info!("   RPC Failures: {}", self.stats.data_quality_metrics.rpc_failures);
        info!("   Compression Coverage: {:.1}%",
              self.stats.data_quality_metrics.compression_coverage * 100.0);
        info!("   Program Diversity: {}", self.stats.data_quality_metrics.program_diversity);
    }

    /// Save collected data and statistics
    pub fn save_data(&self, transactions: &[RealSolanaTransaction], output_dir: &str) -> Result<()> {
        // Create output directory if it doesn't exist
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

/// CLI entry point for real data collection
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    info!("🚀 Starting Real Solana Data Collection");

    // Configuration
    let rpc_endpoint = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

    let output_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "./real_collected_data".to_string());

    let config = CollectionConfig {
        max_transactions: 500, // Start with smaller number for testing
        max_blocks_to_scan: 20,
        target_slot_range: None,
        include_failed_transactions: true,
        focus_on_compression: true,
        timeout_per_request: Duration::from_secs(15),
    };

    // Create collector and collect data
    let mut collector = RealDataCollector::new(rpc_endpoint, Some(config));
    let transactions = collector.collect_real_data().await?;

    // Save the data
    collector.save_data(&transactions, &output_dir)?;

    info!("✅ Real Solana data collection completed successfully!");
    info!("🎯 Collected {} real transactions from Solana blockchain", transactions.len());

    Ok(())
}