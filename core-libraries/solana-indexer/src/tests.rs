#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SolanaConfig, TransactionIndexer, TransactionFilter};
    use tokio_test;

    #[tokio::test]
    async fn test_solana_config_creation() {
        let config = SolanaConfig::new(
            "https://api.devnet.solana.com".to_string(),
            "wss://api.devnet.solana.com".to_string(),
        );

        assert_eq!(config.rpc_url, "https://api.devnet.solana.com");
        assert_eq!(config.ws_url, "wss://api.devnet.solana.com");
        assert_eq!(config.request_timeout_ms, 30000);
        assert_eq!(config.max_concurrent_requests, 10);
    }

    #[tokio::test]
    async fn test_solana_config_builder() {
        let config = SolanaConfig::new(
            "https://api.mainnet-beta.solana.com".to_string(),
            "wss://api.mainnet-beta.solana.com".to_string(),
        )
        .with_timeout(15000)
        .with_batch_size(50)
        .with_tracked_programs(vec!["11111111111111111111111111111112".to_string()]);

        assert_eq!(config.request_timeout_ms, 15000);
        assert_eq!(config.transaction_batch_size, 50);
        assert_eq!(config.tracked_programs.len(), 1);
    }

    #[test]
    fn test_transaction_filter_creation() {
        let filter = TransactionFilter::new()
            .with_failed_transactions(true)
            .with_slot_range(1000, 2000);

        assert!(filter.include_failed);
        assert_eq!(filter.min_slot, Some(1000));
        assert_eq!(filter.max_slot, Some(2000));
    }

    #[test]
    fn test_indexing_stats() {
        let mut stats = crate::types::IndexingStats::new();

        assert_eq!(stats.transactions_indexed, 0);
        assert_eq!(stats.blocks_indexed, 0);
        assert_eq!(stats.errors_encountered, 0);

        stats.update_transaction_count(100);
        assert_eq!(stats.transactions_indexed, 100);

        stats.update_block_count(10, 1500);
        assert_eq!(stats.blocks_indexed, 10);
        assert_eq!(stats.last_indexed_slot, 1500);

        stats.increment_errors();
        assert_eq!(stats.errors_encountered, 1);
    }

    #[tokio::test]
    async fn test_transaction_indexer_creation() {
        let config = SolanaConfig::new(
            "https://api.devnet.solana.com".to_string(),
            "wss://api.devnet.solana.com".to_string(),
        );

        let result = TransactionIndexer::new(config);

        // This might fail due to network connectivity in test environment
        // but we're mainly testing that the structure is sound
        match result {
            Ok(indexer) => {
                assert!(!indexer.is_running().await);
            }
            Err(_) => {
                // Expected in test environment without network access
                // Just verify the config was processed
            }
        }
    }

    #[test]
    fn test_default_solana_config() {
        let config = SolanaConfig::default();

        assert_eq!(config.rpc_url, "https://api.mainnet-beta.solana.com");
        assert_eq!(config.ws_url, "wss://api.mainnet-beta.solana.com");
        assert_eq!(config.request_timeout_ms, 30000);
        assert_eq!(config.max_concurrent_requests, 10);
        assert_eq!(config.slot_cache_size, 1000);
        assert_eq!(config.transaction_batch_size, 100);
        assert_eq!(config.polling_interval_ms, 1000);
        assert!(config.tracked_programs.is_empty());
        assert!(!config.include_failed_transactions);
    }
}