use anyhow::{anyhow, Result};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use crate::client::SolanaClient;
use crate::config::SolanaConfig;
use crate::types::{IndexedTransaction, IndexedBlock, IndexingStats, TransactionFilter};

pub struct TransactionIndexer {
    client: Arc<SolanaClient>,
    config: SolanaConfig,
    running: Arc<RwLock<bool>>,

    // In-memory caches
    recent_blocks: Arc<DashMap<u64, IndexedBlock>>,
    recent_transactions: Arc<DashMap<String, IndexedTransaction>>,

    // Event broadcasting
    transaction_sender: broadcast::Sender<IndexedTransaction>,
    block_sender: broadcast::Sender<IndexedBlock>,

    // Tracking state
    last_processed_slot: Arc<RwLock<u64>>,
    stats: Arc<RwLock<IndexingStats>>,
}

impl TransactionIndexer {
    pub fn new(config: SolanaConfig) -> Result<Self> {
        let client = Arc::new(SolanaClient::new(config.clone())?);

        let recent_blocks = Arc::new(DashMap::with_capacity(config.slot_cache_size));
        let recent_transactions = Arc::new(DashMap::with_capacity(config.slot_cache_size * 100));

        let (transaction_sender, _) = broadcast::channel(1000);
        let (block_sender, _) = broadcast::channel(100);

        let stats = Arc::new(RwLock::new(IndexingStats::new()));

        info!("Initialized transaction indexer with RPC: {}", config.rpc_url);

        Ok(Self {
            client,
            config,
            running: Arc::new(RwLock::new(false)),
            recent_blocks,
            recent_transactions,
            transaction_sender,
            block_sender,
            last_processed_slot: Arc::new(RwLock::new(0)),
            stats,
        })
    }

    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(anyhow!("Transaction indexer is already running"));
        }

        *running = true;
        info!("Starting transaction indexer...");

        // Get current slot to start from
        let current_slot = self.client.get_latest_slot().await?;
        {
            let mut last_slot = self.last_processed_slot.write().await;
            *last_slot = current_slot.saturating_sub(10); // Start from 10 slots back
        }

        // Start background indexing task
        let indexer_clone = self.clone_for_task();
        tokio::spawn(async move {
            indexer_clone.indexing_loop().await;
        });

        // Start cleanup task
        let cleanup_clone = self.clone_for_task();
        tokio::spawn(async move {
            cleanup_clone.cleanup_loop().await;
        });

        info!("Transaction indexer started successfully");
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Ok(());
        }

        info!("Stopping transaction indexer...");
        *running = false;

        // Allow some time for background tasks to finish
        tokio::time::sleep(Duration::from_secs(2)).await;

        info!("Transaction indexer stopped");
        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    pub async fn get_stats(&self) -> IndexingStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    pub fn subscribe_to_transactions(&self) -> broadcast::Receiver<IndexedTransaction> {
        self.transaction_sender.subscribe()
    }

    pub fn subscribe_to_blocks(&self) -> broadcast::Receiver<IndexedBlock> {
        self.block_sender.subscribe()
    }

    pub async fn get_transaction(&self, signature: &str) -> Option<IndexedTransaction> {
        self.recent_transactions.get(signature).map(|tx| tx.clone())
    }

    pub async fn get_block(&self, slot: u64) -> Option<IndexedBlock> {
        self.recent_blocks.get(&slot).map(|block| block.clone())
    }

    pub async fn get_transactions_by_filter(&self, filter: &TransactionFilter) -> Vec<IndexedTransaction> {
        let mut matching_txs = Vec::new();

        for entry in self.recent_transactions.iter() {
            let tx = entry.value();

            // Check slot range
            if let Some(min_slot) = filter.min_slot {
                if tx.slot < min_slot {
                    continue;
                }
            }
            if let Some(max_slot) = filter.max_slot {
                if tx.slot > max_slot {
                    continue;
                }
            }

            // Check success status
            if !filter.include_failed && !tx.success {
                continue;
            }

            // Check programs
            if !filter.programs.is_empty() {
                let mut matches_program = false;
                for instruction in &tx.instructions {
                    if let Ok(program_pubkey) = instruction.program_id.parse() {
                        if filter.programs.contains(&program_pubkey) {
                            matches_program = true;
                            break;
                        }
                    }
                }
                if !matches_program {
                    continue;
                }
            }

            // Check accounts
            if !filter.accounts.is_empty() {
                let mut matches_account = false;
                for account in &tx.accounts {
                    if let Ok(account_pubkey) = account.pubkey.parse() {
                        if filter.accounts.contains(&account_pubkey) {
                            matches_account = true;
                            break;
                        }
                    }
                }
                if !matches_account {
                    continue;
                }
            }

            matching_txs.push(tx.clone());
        }

        // Sort by slot (newest first)
        matching_txs.sort_by(|a, b| b.slot.cmp(&a.slot));
        matching_txs
    }

    async fn indexing_loop(&self) {
        let mut interval = interval(self.config.get_polling_interval());

        while self.is_running().await {
            interval.tick().await;

            if let Err(e) = self.process_new_blocks().await {
                error!("Error processing new blocks: {}", e);
                self.increment_error_stats().await;
            }
        }

        debug!("Indexing loop terminated");
    }

    async fn process_new_blocks(&self) -> Result<()> {
        let latest_slot = self.client.get_latest_slot().await?;
        let last_processed = *self.last_processed_slot.read().await;

        if latest_slot <= last_processed {
            return Ok(());
        }

        let slots_to_process = std::cmp::min(latest_slot - last_processed, 50);

        debug!("Processing {} slots from {} to {}", slots_to_process, last_processed + 1, latest_slot);

        let mut processed_count = 0;

        for slot in (last_processed + 1)..=latest_slot {
            if !self.is_running().await {
                break;
            }

            match self.process_slot(slot).await {
                Ok(Some((block, transactions))) => {
                    self.cache_block(block.clone()).await;
                    self.cache_transactions(transactions.clone()).await;

                    // Broadcast events
                    let _ = self.block_sender.send(block);
                    for tx in transactions {
                        let _ = self.transaction_sender.send(tx);
                    }

                    processed_count += 1;
                }
                Ok(None) => {
                    debug!("Skipped empty slot: {}", slot);
                }
                Err(e) => {
                    warn!("Failed to process slot {}: {}", slot, e);
                    self.increment_error_stats().await;
                }
            }

            // Update progress
            {
                let mut last_slot = self.last_processed_slot.write().await;
                *last_slot = slot;
            }

            // Rate limiting
            if processed_count >= 10 {
                tokio::time::sleep(Duration::from_millis(100)).await;
                processed_count = 0;
            }
        }

        Ok(())
    }

    async fn process_slot(&self, slot: u64) -> Result<Option<(IndexedBlock, Vec<IndexedTransaction>)>> {
        match self.client.get_block(slot).await? {
            Some(block) => {
                let transactions = self.client.get_transactions_for_block(slot).await?;

                self.update_block_stats(1, slot).await;
                self.update_transaction_stats(transactions.len() as u64).await;

                Ok(Some((block, transactions)))
            }
            None => Ok(None),
        }
    }

    async fn cache_block(&self, block: IndexedBlock) {
        self.recent_blocks.insert(block.slot, block);
    }

    async fn cache_transactions(&self, transactions: Vec<IndexedTransaction>) {
        for tx in transactions {
            self.recent_transactions.insert(tx.signature.clone(), tx);
        }
    }

    async fn cleanup_loop(&self) {
        let mut cleanup_interval = interval(Duration::from_secs(60));

        while self.is_running().await {
            cleanup_interval.tick().await;
            self.cleanup_old_data().await;
        }
    }

    async fn cleanup_old_data(&self) {
        let current_slot = match self.client.get_latest_slot().await {
            Ok(slot) => slot,
            Err(_) => return,
        };

        let cutoff_slot = current_slot.saturating_sub(self.config.slot_cache_size as u64);

        // Clean up old blocks
        self.recent_blocks.retain(|&slot, _| slot > cutoff_slot);

        // Clean up old transactions
        self.recent_transactions.retain(|_, tx| tx.slot > cutoff_slot);

        debug!("Cleaned up data older than slot {}", cutoff_slot);
    }

    async fn update_transaction_stats(&self, count: u64) {
        let mut stats = self.stats.write().await;
        stats.update_transaction_count(count);
    }

    async fn update_block_stats(&self, count: u64, last_slot: u64) {
        let mut stats = self.stats.write().await;
        stats.update_block_count(count, last_slot);
    }

    async fn increment_error_stats(&self) {
        let mut stats = self.stats.write().await;
        stats.increment_errors();
    }

    fn clone_for_task(&self) -> Self {
        Self {
            client: self.client.clone(),
            config: self.config.clone(),
            running: self.running.clone(),
            recent_blocks: self.recent_blocks.clone(),
            recent_transactions: self.recent_transactions.clone(),
            transaction_sender: self.transaction_sender.clone(),
            block_sender: self.block_sender.clone(),
            last_processed_slot: self.last_processed_slot.clone(),
            stats: self.stats.clone(),
        }
    }
}