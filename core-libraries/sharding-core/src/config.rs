//! Configuration for sharding operations

use crate::{Result, ShardError};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for sharding behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardConfig {
    /// Number of virtual nodes per physical node
    pub virtual_nodes: usize,
    /// Replication factor for fault tolerance
    pub replication_factor: usize,
    /// Timeout for migration operations in milliseconds
    pub migration_timeout_ms: u64,
    /// Maximum concurrent migrations
    pub max_concurrent_migrations: usize,
    /// Hash function to use
    pub hash_function: HashFunctionType,
    /// Enable automatic rebalancing when nodes join/leave
    pub auto_rebalance: bool,
    /// Rebalance threshold (percentage of imbalance to trigger rebalance)
    pub rebalance_threshold: f64,
    /// Maximum number of keys to migrate in a single batch
    pub migration_batch_size: usize,
    /// Gossip interval for node health checks in milliseconds
    pub gossip_interval_ms: u64,
    /// Node failure detection timeout in milliseconds
    pub failure_detection_timeout_ms: u64,
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval_ms: u64,
    /// Enable compression for migration data
    pub enable_compression: bool,
    /// Checksum validation for data integrity
    pub enable_checksums: bool,
    /// Maximum memory usage for migration buffers in bytes
    pub max_migration_memory: usize,
    /// Metrics collection interval in milliseconds
    pub metrics_interval_ms: u64,
}

/// Available hash functions
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HashFunctionType {
    /// SHA-256 hash function (secure but slower)
    Sha256,
    /// AHash (fast, non-cryptographic)
    AHash,
    /// xxHash (very fast, good distribution)
    XxHash,
}

impl Default for ShardConfig {
    fn default() -> Self {
        Self {
            virtual_nodes: 150,
            replication_factor: 3,
            migration_timeout_ms: 30_000,
            max_concurrent_migrations: 4,
            hash_function: HashFunctionType::AHash,
            auto_rebalance: true,
            rebalance_threshold: 0.1, // 10% imbalance
            migration_batch_size: 1000,
            gossip_interval_ms: 1_000,
            failure_detection_timeout_ms: 10_000,
            heartbeat_interval_ms: 5_000,
            enable_compression: true,
            enable_checksums: true,
            max_migration_memory: 64 * 1024 * 1024, // 64MB
            metrics_interval_ms: 10_000,
        }
    }
}

impl ShardConfig {
    /// Create a new configuration builder
    pub fn builder() -> ShardConfigBuilder {
        ShardConfigBuilder::new()
    }

    /// Create a configuration optimized for testing
    pub fn test_config() -> Self {
        Self {
            virtual_nodes: 10,
            replication_factor: 1,
            migration_timeout_ms: 1_000,
            max_concurrent_migrations: 1,
            hash_function: HashFunctionType::AHash,
            auto_rebalance: false,
            rebalance_threshold: 0.2,
            migration_batch_size: 100,
            gossip_interval_ms: 100,
            failure_detection_timeout_ms: 1_000,
            heartbeat_interval_ms: 500,
            enable_compression: false,
            enable_checksums: false,
            max_migration_memory: 1024 * 1024, // 1MB
            metrics_interval_ms: 1_000,
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.virtual_nodes == 0 {
            return Err(ShardError::InvalidConfiguration {
                reason: "virtual_nodes must be greater than 0".to_string(),
            });
        }

        if self.virtual_nodes > 10_000 {
            return Err(ShardError::InvalidConfiguration {
                reason: "virtual_nodes should not exceed 10,000 for performance reasons".to_string(),
            });
        }

        if self.replication_factor == 0 {
            return Err(ShardError::InvalidConfiguration {
                reason: "replication_factor must be at least 1".to_string(),
            });
        }

        if self.replication_factor > 10 {
            return Err(ShardError::InvalidConfiguration {
                reason: "replication_factor should not exceed 10".to_string(),
            });
        }

        if self.migration_timeout_ms < 1_000 {
            return Err(ShardError::InvalidConfiguration {
                reason: "migration_timeout_ms should be at least 1000ms".to_string(),
            });
        }

        if self.max_concurrent_migrations == 0 {
            return Err(ShardError::InvalidConfiguration {
                reason: "max_concurrent_migrations must be at least 1".to_string(),
            });
        }

        if !(0.0..=1.0).contains(&self.rebalance_threshold) {
            return Err(ShardError::InvalidConfiguration {
                reason: "rebalance_threshold must be between 0.0 and 1.0".to_string(),
            });
        }

        if self.migration_batch_size == 0 {
            return Err(ShardError::InvalidConfiguration {
                reason: "migration_batch_size must be greater than 0".to_string(),
            });
        }

        if self.gossip_interval_ms == 0 {
            return Err(ShardError::InvalidConfiguration {
                reason: "gossip_interval_ms must be greater than 0".to_string(),
            });
        }

        if self.heartbeat_interval_ms == 0 {
            return Err(ShardError::InvalidConfiguration {
                reason: "heartbeat_interval_ms must be greater than 0".to_string(),
            });
        }

        if self.max_migration_memory < 1024 * 1024 {
            return Err(ShardError::InvalidConfiguration {
                reason: "max_migration_memory should be at least 1MB".to_string(),
            });
        }

        Ok(())
    }

    /// Get migration timeout as Duration
    pub fn migration_timeout(&self) -> Duration {
        Duration::from_millis(self.migration_timeout_ms)
    }

    /// Get gossip interval as Duration
    pub fn gossip_interval(&self) -> Duration {
        Duration::from_millis(self.gossip_interval_ms)
    }

    /// Get failure detection timeout as Duration
    pub fn failure_detection_timeout(&self) -> Duration {
        Duration::from_millis(self.failure_detection_timeout_ms)
    }

    /// Get heartbeat interval as Duration
    pub fn heartbeat_interval(&self) -> Duration {
        Duration::from_millis(self.heartbeat_interval_ms)
    }

    /// Get metrics interval as Duration
    pub fn metrics_interval(&self) -> Duration {
        Duration::from_millis(self.metrics_interval_ms)
    }
}

/// Builder for ShardConfig
#[derive(Debug)]
pub struct ShardConfigBuilder {
    config: ShardConfig,
}

impl Default for ShardConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ShardConfigBuilder {
    /// Create a new builder with default values
    pub fn new() -> Self {
        Self {
            config: ShardConfig::default(),
        }
    }

    /// Set the number of virtual nodes
    pub fn virtual_nodes(mut self, virtual_nodes: usize) -> Self {
        self.config.virtual_nodes = virtual_nodes;
        self
    }

    /// Set the replication factor
    pub fn replication_factor(mut self, replication_factor: usize) -> Self {
        self.config.replication_factor = replication_factor;
        self
    }

    /// Set the migration timeout in milliseconds
    pub fn migration_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.config.migration_timeout_ms = timeout_ms;
        self
    }

    /// Set the maximum concurrent migrations
    pub fn max_concurrent_migrations(mut self, max_migrations: usize) -> Self {
        self.config.max_concurrent_migrations = max_migrations;
        self
    }

    /// Set the hash function
    pub fn hash_function(mut self, hash_function: HashFunctionType) -> Self {
        self.config.hash_function = hash_function;
        self
    }

    /// Enable or disable automatic rebalancing
    pub fn auto_rebalance(mut self, enabled: bool) -> Self {
        self.config.auto_rebalance = enabled;
        self
    }

    /// Set the rebalance threshold
    pub fn rebalance_threshold(mut self, threshold: f64) -> Self {
        self.config.rebalance_threshold = threshold;
        self
    }

    /// Set the migration batch size
    pub fn migration_batch_size(mut self, batch_size: usize) -> Self {
        self.config.migration_batch_size = batch_size;
        self
    }

    /// Set the gossip interval in milliseconds
    pub fn gossip_interval_ms(mut self, interval_ms: u64) -> Self {
        self.config.gossip_interval_ms = interval_ms;
        self
    }

    /// Set the failure detection timeout in milliseconds
    pub fn failure_detection_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.config.failure_detection_timeout_ms = timeout_ms;
        self
    }

    /// Set the heartbeat interval in milliseconds
    pub fn heartbeat_interval_ms(mut self, interval_ms: u64) -> Self {
        self.config.heartbeat_interval_ms = interval_ms;
        self
    }

    /// Enable or disable compression
    pub fn enable_compression(mut self, enabled: bool) -> Self {
        self.config.enable_compression = enabled;
        self
    }

    /// Enable or disable checksums
    pub fn enable_checksums(mut self, enabled: bool) -> Self {
        self.config.enable_checksums = enabled;
        self
    }

    /// Set the maximum migration memory in bytes
    pub fn max_migration_memory(mut self, max_memory: usize) -> Self {
        self.config.max_migration_memory = max_memory;
        self
    }

    /// Set the metrics interval in milliseconds
    pub fn metrics_interval_ms(mut self, interval_ms: u64) -> Self {
        self.config.metrics_interval_ms = interval_ms;
        self
    }

    /// Build the configuration
    pub fn build(self) -> Result<ShardConfig> {
        self.config.validate()?;
        Ok(self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ShardConfig::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.virtual_nodes, 150);
        assert_eq!(config.replication_factor, 3);
        assert_eq!(config.hash_function, HashFunctionType::AHash);
        assert!(config.auto_rebalance);
    }

    #[test]
    fn test_test_config() {
        let config = ShardConfig::test_config();
        assert!(config.validate().is_ok());
        assert_eq!(config.virtual_nodes, 10);
        assert_eq!(config.replication_factor, 1);
        assert!(!config.auto_rebalance);
    }

    #[test]
    fn test_config_builder() {
        let config = ShardConfig::builder()
            .virtual_nodes(100)
            .replication_factor(2)
            .migration_timeout_ms(5000)
            .hash_function(HashFunctionType::Sha256)
            .auto_rebalance(false)
            .build()
            .unwrap();

        assert_eq!(config.virtual_nodes, 100);
        assert_eq!(config.replication_factor, 2);
        assert_eq!(config.migration_timeout_ms, 5000);
        assert_eq!(config.hash_function, HashFunctionType::Sha256);
        assert!(!config.auto_rebalance);
    }

    #[test]
    fn test_invalid_configurations() {
        // Zero virtual nodes
        let result = ShardConfig::builder()
            .virtual_nodes(0)
            .build();
        assert!(result.is_err());

        // Zero replication factor
        let result = ShardConfig::builder()
            .replication_factor(0)
            .build();
        assert!(result.is_err());

        // Too short migration timeout
        let result = ShardConfig::builder()
            .migration_timeout_ms(500)
            .build();
        assert!(result.is_err());

        // Invalid rebalance threshold
        let result = ShardConfig::builder()
            .rebalance_threshold(1.5)
            .build();
        assert!(result.is_err());

        // Zero migration batch size
        let result = ShardConfig::builder()
            .migration_batch_size(0)
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_duration_conversions() {
        let config = ShardConfig::default();

        assert_eq!(config.migration_timeout(), Duration::from_millis(30_000));
        assert_eq!(config.gossip_interval(), Duration::from_millis(1_000));
        assert_eq!(config.failure_detection_timeout(), Duration::from_millis(10_000));
        assert_eq!(config.heartbeat_interval(), Duration::from_millis(5_000));
        assert_eq!(config.metrics_interval(), Duration::from_millis(10_000));
    }

    #[test]
    fn test_config_serialization() {
        let config = ShardConfig::default();

        // Test JSON serialization
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ShardConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.virtual_nodes, deserialized.virtual_nodes);
        assert_eq!(config.replication_factor, deserialized.replication_factor);
        assert_eq!(config.hash_function, deserialized.hash_function);
    }
}