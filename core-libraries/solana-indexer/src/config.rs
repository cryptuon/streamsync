use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaConfig {
    /// RPC endpoint URL
    pub rpc_url: String,

    /// WebSocket endpoint URL for real-time updates
    pub ws_url: String,

    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,

    /// Maximum number of concurrent RPC requests
    pub max_concurrent_requests: usize,

    /// Number of recent slots to keep in memory
    pub slot_cache_size: usize,

    /// Batch size for transaction fetching
    pub transaction_batch_size: usize,

    /// How often to poll for new blocks (milliseconds)
    pub polling_interval_ms: u64,

    /// Programs to specifically track (empty = track all)
    pub tracked_programs: Vec<String>,

    /// Whether to include failed transactions
    pub include_failed_transactions: bool,
}

impl Default for SolanaConfig {
    fn default() -> Self {
        Self {
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            ws_url: "wss://api.mainnet-beta.solana.com".to_string(),
            request_timeout_ms: 30000,
            max_concurrent_requests: 10,
            slot_cache_size: 1000,
            transaction_batch_size: 100,
            polling_interval_ms: 1000,
            tracked_programs: vec![],
            include_failed_transactions: false,
        }
    }
}

impl SolanaConfig {
    pub fn new(rpc_url: String, ws_url: String) -> Self {
        Self {
            rpc_url,
            ws_url,
            ..Default::default()
        }
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.request_timeout_ms = timeout_ms;
        self
    }

    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.transaction_batch_size = batch_size;
        self
    }

    pub fn with_tracked_programs(mut self, programs: Vec<String>) -> Self {
        self.tracked_programs = programs;
        self
    }

    pub fn get_timeout(&self) -> Duration {
        Duration::from_millis(self.request_timeout_ms)
    }

    pub fn get_polling_interval(&self) -> Duration {
        Duration::from_millis(self.polling_interval_ms)
    }
}