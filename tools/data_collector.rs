//! Real Solana Data Collection Tool
//!
//! This tool collects authentic Solana blockchain data for testing and validation,
//! focusing on:
//! - State compression transactions (compressed NFTs)
//! - Popular program interactions (SPL Token, Metaplex, Jupiter)
//! - Various transaction patterns and account structures
//! - Building a comprehensive dataset for confidence testing

use solana_client::{
    rpc_client::RpcClient,
    rpc_config::RpcBlockConfig,
};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    slot_history::Slot,
};
use solana_transaction_status::{
    UiTransactionEncoding, TransactionDetails, EncodedTransaction,
};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, SystemTime};
use serde::{Serialize, Deserialize};
use tracing::{info, warn, debug};
use anyhow::{Result, Context};
use std::fs;
use std::path::Path;

/// Configuration for data collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataCollectionConfig {
    pub rpc_url: String,
    pub output_directory: String,
    pub target_programs: Vec<TargetProgram>,
    pub collection_limits: CollectionLimits,
    pub data_filters: DataFilters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetProgram {
    pub name: String,
    pub program_id: String,
    pub description: String,
    pub expected_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionLimits {
    pub max_blocks_to_scan: u64,
    pub max_transactions_per_program: usize,
    pub max_total_transactions: usize,
    pub max_collection_time_minutes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFilters {
    pub min_transaction_size: usize,
    pub max_transaction_size: usize,
    pub include_failed_transactions: bool,
    pub require_state_compression: bool,
}

impl Default for DataCollectionConfig {
    fn default() -> Self {
        Self {
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            output_directory: "./collected_data".to_string(),
            target_programs: vec![
                TargetProgram {
                    name: "SPL Token".to_string(),
                    program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                    description: "SPL Token Program - transfers, mints, burns".to_string(),
                    expected_patterns: vec!["transfer".to_string(), "mint_to".to_string(), "burn".to_string()],
                },
                TargetProgram {
                    name: "Metaplex Bubblegum".to_string(),
                    program_id: "BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY".to_string(),
                    description: "Compressed NFTs (cNFTs) - state compression".to_string(),
                    expected_patterns: vec!["mint_v1".to_string(), "transfer".to_string(), "burn".to_string()],
                },
                TargetProgram {
                    name: "Account Compression".to_string(),
                    program_id: "cmtDvXumGCrqC1Age74AVPhSRVXJMd8PJS91L8KbNCK".to_string(),
                    description: "SPL Account Compression for merkle trees".to_string(),
                    expected_patterns: vec!["append".to_string(), "replace_leaf".to_string()],
                },
                TargetProgram {
                    name: "Jupiter Aggregator".to_string(),
                    program_id: "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string(),
                    description: "DEX aggregator for token swaps".to_string(),
                    expected_patterns: vec!["route".to_string(), "swap".to_string()],
                },
            ],
            collection_limits: CollectionLimits {
                max_blocks_to_scan: 1000,
                max_transactions_per_program: 200,
                max_total_transactions: 1000,
                max_collection_time_minutes: 30,
            },
            data_filters: DataFilters {
                min_transaction_size: 100,
                max_transaction_size: 10_000,
                include_failed_transactions: false,
                require_state_compression: false,
            },
        }
    }
}

/// Collected transaction data
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
    // Compression-specific data
    pub is_state_compression: bool,
    pub merkle_tree: Option<String>,
    pub compressed_account_data: Option<Vec<u8>>,
    pub compression_proof: Option<Vec<String>>,
}

/// Dataset statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetStats {
    pub collection_start_time: SystemTime,
    pub collection_end_time: SystemTime,
    pub total_blocks_scanned: u64,
    pub total_transactions_collected: usize,
    pub transactions_per_program: HashMap<String, usize>,
    pub success_rate: f64,
    pub average_transaction_size: f64,
    pub compression_transaction_count: usize,
    pub unique_merkle_trees: HashSet<String>,
}

/// Main data collector
pub struct SolanaDataCollector {
    config: DataCollectionConfig,
    rpc_client: RpcClient,
    collected_data: Vec<CollectedTransaction>,
    stats: DatasetStats,
}

impl SolanaDataCollector {
    pub fn new(config: DataCollectionConfig) -> Result<Self> {
        let rpc_client = RpcClient::new_with_commitment(
            config.rpc_url.clone(),
            CommitmentConfig::confirmed(),
        );

        let stats = DatasetStats {
            collection_start_time: SystemTime::now(),
            collection_end_time: SystemTime::now(),
            total_blocks_scanned: 0,
            total_transactions_collected: 0,
            transactions_per_program: HashMap::new(),
            success_rate: 0.0,
            average_transaction_size: 0.0,
            compression_transaction_count: 0,
            unique_merkle_trees: HashSet::new(),
        };

        Ok(Self {
            config,
            rpc_client,
            collected_data: Vec::new(),
            stats,
        })
    }

    /// Start data collection process
    pub async fn collect_dataset(&mut self) -> Result<()> {
        info!("🚀 Starting Solana data collection");
        info!("📋 Config: {} target programs, max {} blocks, max {} transactions",
              self.config.target_programs.len(),
              self.config.collection_limits.max_blocks_to_scan,
              self.config.collection_limits.max_total_transactions);

        self.stats.collection_start_time = SystemTime::now();

        // Get the latest slot to work backwards from
        let latest_slot = self.rpc_client.get_slot()
            .context("Failed to get latest slot")?;

        info!("📍 Starting from slot: {}", latest_slot);

        let start_time = SystemTime::now();
        let max_duration = Duration::from_secs(self.config.collection_limits.max_collection_time_minutes * 60);

        // Collect data by scanning recent blocks
        for i in 0..self.config.collection_limits.max_blocks_to_scan {
            if start_time.elapsed().unwrap_or_default() > max_duration {
                warn!("⏰ Collection time limit reached");
                break;
            }

            if self.collected_data.len() >= self.config.collection_limits.max_total_transactions {
                info!("✅ Transaction limit reached");
                break;
            }

            let slot = latest_slot.saturating_sub(i);
            self.collect_block_data(slot).await?;

            if i % 100 == 0 {
                info!("📊 Progress: {} blocks scanned, {} transactions collected",
                      i + 1, self.collected_data.len());
            }
        }

        self.stats.collection_end_time = SystemTime::now();
        self.finalize_stats();

        info!("✅ Data collection completed!");
        self.print_collection_summary();

        Ok(())
    }

    /// Collect data from a specific block
    async fn collect_block_data(&mut self, slot: Slot) -> Result<()> {
        debug!("🔍 Scanning slot: {}", slot);

        // Get block with transaction details
        let block = match self.rpc_client.get_block_with_config(
            slot,
            RpcBlockConfig {
                encoding: Some(UiTransactionEncoding::Json),
                transaction_details: Some(TransactionDetails::Full),
                rewards: Some(false),
                commitment: Some(CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
            },
        ) {
            Ok(block) => block,
            Err(e) => {
                debug!("⚠️ Failed to get block {}: {}", slot, e);
                return Ok(()); // Skip this block
            }
        };

        self.stats.total_blocks_scanned += 1;

        if let Some(transactions) = block.transactions {
            for transaction in transactions {
                // Extract message from the EncodedTransaction (only Json variant has full message)
                if let EncodedTransaction::Json(ui_tx) = &transaction.transaction {
                    self.process_transaction(slot, block.block_time, &transaction, &ui_tx.message).await?;
                }
            }
        }

        Ok(())
    }

    /// Process a single transaction
    async fn process_transaction(
        &mut self,
        slot: Slot,
        block_time: Option<i64>,
        transaction: &solana_transaction_status::EncodedTransactionWithStatusMeta,
        message: &solana_transaction_status::UiMessage,
    ) -> Result<()> {

        // Extract account keys and instructions
        let account_keys = match message {
            solana_transaction_status::UiMessage::Parsed(parsed) => {
                parsed.account_keys.iter().map(|k| k.pubkey.clone()).collect()
            }
            solana_transaction_status::UiMessage::Raw(raw) => {
                raw.account_keys.clone()
            }
        };

        // Only process parsed messages which have UiInstruction
        let instructions = match message {
            solana_transaction_status::UiMessage::Parsed(parsed) => &parsed.instructions,
            solana_transaction_status::UiMessage::Raw(_) => return Ok(()), // Skip raw messages
        };

        // Check if transaction involves our target programs
        for instruction in instructions {
            if let Some(program_index) = self.get_program_index(instruction) {
                if let Some(program_id) = account_keys.get(program_index) {
                    if let Some(target_program) = self.find_target_program(program_id) {
                        let collected_tx = self.create_collected_transaction(
                            slot,
                            block_time,
                            transaction,
                            &target_program,
                            program_id,
                            instruction,
                            &account_keys,
                        ).await?;

                        if self.should_include_transaction(&collected_tx) {
                            debug!("📦 Collected transaction: {} for program: {}",
                                   collected_tx.signature, collected_tx.program_name);
                            self.collected_data.push(collected_tx);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Create a CollectedTransaction from the raw data
    async fn create_collected_transaction(
        &self,
        slot: Slot,
        block_time: Option<i64>,
        transaction: &solana_transaction_status::EncodedTransactionWithStatusMeta,
        target_program: &TargetProgram,
        program_id: &str,
        instruction: &solana_transaction_status::UiInstruction,
        account_keys: &[String],
    ) -> Result<CollectedTransaction> {

        // Extract signature from the EncodedTransaction
        let signature = match &transaction.transaction {
            EncodedTransaction::Json(ui_tx) => ui_tx.signatures.first()
                .cloned()
                .unwrap_or_else(|| "unknown".to_string()),
            EncodedTransaction::Accounts(ui_tx) => ui_tx.signatures.first()
                .cloned()
                .unwrap_or_else(|| "unknown".to_string()),
            _ => "unknown".to_string(),
        };

        let success = transaction.meta
            .as_ref()
            .map(|meta| meta.err.is_none())
            .unwrap_or(false);

        let error_message = transaction.meta
            .as_ref()
            .and_then(|meta| meta.err.as_ref())
            .map(|err| format!("{:?}", err));

        let (pre_balances, post_balances) = transaction.meta
            .as_ref()
            .map(|meta| (meta.pre_balances.clone(), meta.post_balances.clone()))
            .unwrap_or((vec![], vec![]));

        let log_messages = transaction.meta
            .as_ref()
            .and_then(|meta| Option::<Vec<String>>::from(meta.log_messages.clone()))
            .unwrap_or_default();

        let compute_units_consumed = transaction.meta
            .as_ref()
            .and_then(|meta| Option::<u64>::from(meta.compute_units_consumed.clone()));

        // Extract instruction data
        let instruction_data = self.extract_instruction_data(instruction);

        // Check for state compression patterns
        let is_state_compression = self.detect_state_compression(&log_messages, &instruction_data);
        let merkle_tree = self.extract_merkle_tree(&account_keys, &log_messages);

        Ok(CollectedTransaction {
            signature,
            slot,
            block_time,
            program_id: program_id.to_string(),
            program_name: target_program.name.clone(),
            instruction_data,
            accounts: account_keys.to_vec(),
            pre_balances,
            post_balances,
            log_messages,
            compute_units_consumed,
            success,
            error_message,
            is_state_compression,
            merkle_tree,
            compressed_account_data: None, // TODO: Extract from logs if available
            compression_proof: None, // TODO: Extract from logs if available
        })
    }

    /// Extract instruction data from UI instruction
    fn extract_instruction_data(&self, instruction: &solana_transaction_status::UiInstruction) -> Vec<u8> {
        match instruction {
            solana_transaction_status::UiInstruction::Compiled(compiled) => {
                bs58::decode(&compiled.data).into_vec().unwrap_or_default()
            }
            solana_transaction_status::UiInstruction::Parsed(_) => {
                // For parsed instructions, we'd need to reconstruct the data
                // For now, return empty data
                vec![]
            }
        }
    }

    /// Detect if transaction involves state compression
    fn detect_state_compression(&self, log_messages: &[String], instruction_data: &[u8]) -> bool {
        // Look for compression-related log messages
        let compression_keywords = [
            "Program log: Instruction: Append",
            "Program log: Instruction: ReplaceLeaf",
            "Program log: Instruction: Transfer",
            "compressed",
            "merkle",
            "leaf",
        ];

        for log in log_messages {
            for keyword in &compression_keywords {
                if log.contains(keyword) {
                    return true;
                }
            }
        }

        // Check for known compression instruction discriminators
        if instruction_data.len() >= 8 {
            let _discriminator = &instruction_data[0..8];
            // Add known compression discriminators here
            // This would need to be populated with actual discriminators
        }

        false
    }

    /// Extract merkle tree address from transaction
    fn extract_merkle_tree(&self, _account_keys: &[String], log_messages: &[String]) -> Option<String> {
        // Look for merkle tree references in logs
        for log in log_messages {
            if log.contains("merkle tree") || log.contains("tree:") {
                // Try to extract the tree address from the log message
                // This is a simplified extraction - real implementation would be more sophisticated
                if let Some(start) = log.find("tree: ") {
                    let tree_part = &log[start + 6..];
                    if let Some(end) = tree_part.find(' ') {
                        return Some(tree_part[..end].to_string());
                    }
                }
            }
        }

        // If not found in logs, check if any account looks like a merkle tree
        // (This is heuristic-based and would need refinement)
        None
    }

    /// Check if transaction should be included in dataset
    fn should_include_transaction(&self, tx: &CollectedTransaction) -> bool {
        // Apply filters
        if !self.config.data_filters.include_failed_transactions && !tx.success {
            return false;
        }

        if tx.instruction_data.len() < self.config.data_filters.min_transaction_size ||
           tx.instruction_data.len() > self.config.data_filters.max_transaction_size {
            return false;
        }

        if self.config.data_filters.require_state_compression && !tx.is_state_compression {
            return false;
        }

        // Check per-program limits
        let current_count = self.collected_data
            .iter()
            .filter(|t| t.program_id == tx.program_id)
            .count();

        if current_count >= self.config.collection_limits.max_transactions_per_program {
            return false;
        }

        true
    }

    /// Helper methods
    fn get_program_index(&self, instruction: &solana_transaction_status::UiInstruction) -> Option<usize> {
        match instruction {
            solana_transaction_status::UiInstruction::Compiled(compiled) => Some(compiled.program_id_index as usize),
            solana_transaction_status::UiInstruction::Parsed(_parsed) => {
                // For parsed instructions, try to extract program ID
                None // Simplified for now
            }
        }
    }

    fn find_target_program(&self, program_id: &str) -> Option<&TargetProgram> {
        self.config.target_programs
            .iter()
            .find(|p| p.program_id == program_id)
    }

    /// Finalize statistics
    fn finalize_stats(&mut self) {
        self.stats.total_transactions_collected = self.collected_data.len();

        // Calculate per-program stats
        for tx in &self.collected_data {
            *self.stats.transactions_per_program.entry(tx.program_name.clone()).or_insert(0) += 1;

            if tx.is_state_compression {
                self.stats.compression_transaction_count += 1;
            }

            if let Some(tree) = &tx.merkle_tree {
                self.stats.unique_merkle_trees.insert(tree.clone());
            }
        }

        // Calculate success rate
        let successful_count = self.collected_data.iter().filter(|tx| tx.success).count();
        self.stats.success_rate = if self.collected_data.is_empty() {
            0.0
        } else {
            successful_count as f64 / self.collected_data.len() as f64
        };

        // Calculate average transaction size
        let total_size: usize = self.collected_data.iter().map(|tx| tx.instruction_data.len()).sum();
        self.stats.average_transaction_size = if self.collected_data.is_empty() {
            0.0
        } else {
            total_size as f64 / self.collected_data.len() as f64
        };
    }

    /// Print collection summary
    fn print_collection_summary(&self) {
        let duration = self.stats.collection_end_time
            .duration_since(self.stats.collection_start_time)
            .unwrap_or_default();

        info!("📊 === DATA COLLECTION SUMMARY ===");
        info!("⏱️  Collection Duration: {:?}", duration);
        info!("📦 Total Transactions Collected: {}", self.stats.total_transactions_collected);
        info!("🏗️  Blocks Scanned: {}", self.stats.total_blocks_scanned);
        info!("✅ Success Rate: {:.2}%", self.stats.success_rate * 100.0);
        info!("📏 Average Transaction Size: {:.1} bytes", self.stats.average_transaction_size);
        info!("🗜️  State Compression Transactions: {}", self.stats.compression_transaction_count);
        info!("🌳 Unique Merkle Trees: {}", self.stats.unique_merkle_trees.len());

        info!("📋 Per-Program Breakdown:");
        for (program, count) in &self.stats.transactions_per_program {
            info!("   {}: {} transactions", program, count);
        }
    }

    /// Save collected dataset to files
    pub fn save_dataset(&self) -> Result<()> {
        let output_dir = Path::new(&self.config.output_directory);
        fs::create_dir_all(output_dir)
            .context("Failed to create output directory")?;

        // Save main dataset
        let dataset_file = output_dir.join("solana_dataset.json");
        let dataset_json = serde_json::to_string_pretty(&self.collected_data)
            .context("Failed to serialize dataset")?;
        fs::write(dataset_file, dataset_json)
            .context("Failed to write dataset file")?;

        // Save statistics
        let stats_file = output_dir.join("dataset_stats.json");
        let stats_json = serde_json::to_string_pretty(&self.stats)
            .context("Failed to serialize stats")?;
        fs::write(stats_file, stats_json)
            .context("Failed to write stats file")?;

        // Save configuration
        let config_file = output_dir.join("collection_config.json");
        let config_json = serde_json::to_string_pretty(&self.config)
            .context("Failed to serialize config")?;
        fs::write(config_file, config_json)
            .context("Failed to write config file")?;

        info!("💾 Dataset saved to: {}", output_dir.display());
        Ok(())
    }
}

/// CLI entry point
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    info!("🚀 Starting Solana Data Collector");

    // Load or create configuration
    let config = DataCollectionConfig::default();

    // Create and run collector
    let mut collector = SolanaDataCollector::new(config)?;
    collector.collect_dataset().await?;
    collector.save_dataset()?;

    info!("✅ Data collection completed successfully!");
    Ok(())
}