use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};

use crate::config::SolanaConfig;
use crate::types::{IndexedBlock, IndexedTransaction, IndexingStats};

pub struct SolanaClient {
    rpc_client: Arc<RpcClient>,
    config: SolanaConfig,
    semaphore: Arc<Semaphore>,
    stats: Arc<tokio::sync::RwLock<IndexingStats>>,
}

impl SolanaClient {
    pub fn new(config: SolanaConfig) -> Result<Self> {
        let rpc_client = Arc::new(RpcClient::new_with_timeout(
            config.rpc_url.clone(),
            config.get_timeout(),
        ));

        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_requests));
        let stats = Arc::new(tokio::sync::RwLock::new(IndexingStats::new()));

        info!("Initialized Solana client for RPC: {}", config.rpc_url);

        Ok(Self {
            rpc_client,
            config,
            semaphore,
            stats,
        })
    }

    pub async fn get_latest_slot(&self) -> Result<u64> {
        let _permit = self.semaphore.acquire().await.map_err(|e| anyhow!("Semaphore error: {}", e))?;

        let slot = self.rpc_client
            .get_slot()
            .map_err(|e| anyhow!("Failed to get latest slot: {}", e))?;

        debug!("Latest slot: {}", slot);
        Ok(slot)
    }

    pub async fn get_block(&self, slot: u64) -> Result<Option<IndexedBlock>> {
        let _permit = self.semaphore.acquire().await.map_err(|e| anyhow!("Semaphore error: {}", e))?;

        // Simplified block creation for demonstration
        let indexed_block = IndexedBlock {
            slot,
            blockhash: format!("block_hash_{}", slot),
            parent_slot: slot.saturating_sub(1),
            block_time: Some(chrono::Utc::now()),
            block_height: Some(slot),
            transaction_count: 1,
            successful_transactions: 1,
            failed_transactions: 0,
            total_fees: 5000,
            leader: "demo_validator".to_string(),
        };

        debug!("Created mock block for slot: {}", slot);
        Ok(Some(indexed_block))
    }

    pub async fn get_transactions_for_block(&self, slot: u64) -> Result<Vec<IndexedTransaction>> {
        // Simplified transaction creation for demonstration
        let transactions = vec![IndexedTransaction {
            id: uuid::Uuid::new_v4(),
            signature: format!("sig_{}", slot),
            slot,
            block_time: Some(chrono::Utc::now()),
            fee: 5000,
            success: true,
            accounts: vec![],
            instructions: vec![],
            log_messages: None,
            compute_units_consumed: Some(200000),
        }];

        self.update_transaction_stats(transactions.len() as u64).await;
        debug!("Created {} mock transactions for slot: {}", transactions.len(), slot);
        Ok(transactions)
    }

    pub async fn get_slot_range(&self, start_slot: u64, end_slot: u64) -> Result<Vec<u64>> {
        let mut slots = Vec::new();
        for slot in start_slot..=end_slot {
            slots.push(slot);
        }
        Ok(slots)
    }

    pub async fn get_stats(&self) -> IndexingStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    fn should_index_transaction(&self, transaction: &IndexedTransaction) -> bool {
        if !self.config.include_failed_transactions && !transaction.success {
            return false;
        }

        if self.config.tracked_programs.is_empty() {
            return true;
        }

        // For demo purposes, always index
        true
    }

    async fn update_transaction_stats(&self, count: u64) {
        let mut stats = self.stats.write().await;
        stats.update_transaction_count(count);
    }

    async fn increment_error_count(&self) {
        let mut stats = self.stats.write().await;
        stats.increment_errors();
    }
}