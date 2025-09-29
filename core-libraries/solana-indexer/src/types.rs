use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedTransaction {
    pub id: Uuid,
    pub signature: String,
    pub slot: u64,
    pub block_time: Option<DateTime<Utc>>,
    pub fee: u64,
    pub success: bool,
    pub accounts: Vec<IndexedAccount>,
    pub instructions: Vec<IndexedInstruction>,
    pub log_messages: Option<Vec<String>>,
    pub compute_units_consumed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedAccount {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
    pub pre_balance: u64,
    pub post_balance: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedInstruction {
    pub program_id: String,
    pub accounts: Vec<u8>,
    pub data: String,
    pub inner_instructions: Vec<IndexedInnerInstruction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedInnerInstruction {
    pub instruction_index: u8,
    pub program_id: String,
    pub accounts: Vec<u8>,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedBlock {
    pub slot: u64,
    pub blockhash: String,
    pub parent_slot: u64,
    pub block_time: Option<DateTime<Utc>>,
    pub block_height: Option<u64>,
    pub transaction_count: usize,
    pub successful_transactions: usize,
    pub failed_transactions: usize,
    pub total_fees: u64,
    pub leader: String,
}

#[derive(Debug, Clone)]
pub struct TransactionFilter {
    pub programs: Vec<Pubkey>,
    pub accounts: Vec<Pubkey>,
    pub include_failed: bool,
    pub min_slot: Option<u64>,
    pub max_slot: Option<u64>,
}

impl Default for TransactionFilter {
    fn default() -> Self {
        Self {
            programs: vec![],
            accounts: vec![],
            include_failed: false,
            min_slot: None,
            max_slot: None,
        }
    }
}

impl TransactionFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_programs(mut self, programs: Vec<Pubkey>) -> Self {
        self.programs = programs;
        self
    }

    pub fn with_accounts(mut self, accounts: Vec<Pubkey>) -> Self {
        self.accounts = accounts;
        self
    }

    pub fn with_failed_transactions(mut self, include_failed: bool) -> Self {
        self.include_failed = include_failed;
        self
    }

    pub fn with_slot_range(mut self, min_slot: u64, max_slot: u64) -> Self {
        self.min_slot = Some(min_slot);
        self.max_slot = Some(max_slot);
        self
    }
}

#[derive(Debug, Clone)]
pub struct IndexingStats {
    pub transactions_indexed: u64,
    pub blocks_indexed: u64,
    pub errors_encountered: u64,
    pub last_indexed_slot: u64,
    pub indexing_rate_per_second: f64,
    pub started_at: DateTime<Utc>,
    pub last_update: DateTime<Utc>,
}

impl IndexingStats {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            transactions_indexed: 0,
            blocks_indexed: 0,
            errors_encountered: 0,
            last_indexed_slot: 0,
            indexing_rate_per_second: 0.0,
            started_at: now,
            last_update: now,
        }
    }

    pub fn update_transaction_count(&mut self, count: u64) {
        self.transactions_indexed += count;
        self.update_rate();
    }

    pub fn update_block_count(&mut self, count: u64, last_slot: u64) {
        self.blocks_indexed += count;
        self.last_indexed_slot = last_slot;
        self.update_rate();
    }

    pub fn increment_errors(&mut self) {
        self.errors_encountered += 1;
        self.last_update = Utc::now();
    }

    fn update_rate(&mut self) {
        let now = Utc::now();
        let duration = (now - self.started_at).num_seconds() as f64;
        if duration > 0.0 {
            self.indexing_rate_per_second = self.transactions_indexed as f64 / duration;
        }
        self.last_update = now;
    }
}