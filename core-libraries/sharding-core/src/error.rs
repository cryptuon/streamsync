//! Error types for the sharding core library

use std::net::SocketAddr;
use thiserror::Error;

/// Result type alias for sharding operations
pub type Result<T> = std::result::Result<T, ShardError>;

/// Comprehensive error types for sharding operations
#[derive(Error, Debug)]
pub enum ShardError {
    /// Configuration errors
    #[error("Invalid configuration: {reason}")]
    InvalidConfiguration { reason: String },

    /// Node management errors
    #[error("Node not found: {node_id}")]
    NodeNotFound { node_id: String },

    #[error("Node already exists: {node_id}")]
    NodeAlreadyExists { node_id: String },

    #[error("Insufficient nodes for replication factor {required}, only {available} available")]
    InsufficientNodes { required: usize, available: usize },

    #[error("Node is unhealthy: {node_id}, status: {status}")]
    NodeUnhealthy { node_id: String, status: String },

    /// Hash ring errors
    #[error("Hash ring is empty")]
    EmptyHashRing,

    #[error("Hash function error: {reason}")]
    HashFunctionError { reason: String },

    #[error("Virtual node collision detected")]
    VirtualNodeCollision,

    /// Migration errors
    #[error("Migration already in progress for key range {start}-{end}")]
    MigrationInProgress { start: u64, end: u64 },

    #[error("Migration failed: {reason}")]
    MigrationFailed { reason: String },

    #[error("Migration timeout after {timeout_ms}ms")]
    MigrationTimeout { timeout_ms: u64 },

    #[error("No migration plan available")]
    NoMigrationPlan,

    /// Replication errors
    #[error("Replication failed: {reason}")]
    ReplicationFailed { reason: String },

    #[error("Quorum not achieved: got {actual} votes, needed {required}")]
    QuorumNotAchieved { actual: usize, required: usize },

    #[error("Replica not found: {replica_id}")]
    ReplicaNotFound { replica_id: String },

    #[error("Inconsistent replicas detected for key {key}")]
    InconsistentReplicas { key: String },

    /// Network and communication errors
    #[error("Network error: {reason}")]
    NetworkError { reason: String },

    #[error("Connection failed to {address}: {reason}")]
    ConnectionFailed { address: SocketAddr, reason: String },

    #[error("Communication timeout with node {node_id}")]
    CommunicationTimeout { node_id: String },

    /// Data integrity errors
    #[error("Serialization error: {reason}")]
    SerializationError { reason: String },

    #[error("Deserialization error: {reason}")]
    DeserializationError { reason: String },

    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Data corruption detected for key {key}")]
    DataCorruption { key: String },

    /// Resource errors
    #[error("Storage capacity exceeded: {used}/{total} bytes")]
    StorageCapacityExceeded { used: u64, total: u64 },

    #[error("Memory limit exceeded: {used}/{limit} bytes")]
    MemoryLimitExceeded { used: usize, limit: usize },

    #[error("Rate limit exceeded: {current_rate}/s > {limit}/s")]
    RateLimitExceeded { current_rate: u64, limit: u64 },

    /// Generic errors
    #[error("Operation not supported: {operation}")]
    OperationNotSupported { operation: String },

    #[error("Invalid state: {reason}")]
    InvalidState { reason: String },

    #[error("Internal error: {reason}")]
    InternalError { reason: String },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl ShardError {
    /// Check if the error is recoverable (temporary)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            ShardError::NetworkError { .. }
                | ShardError::ConnectionFailed { .. }
                | ShardError::CommunicationTimeout { .. }
                | ShardError::MigrationTimeout { .. }
                | ShardError::QuorumNotAchieved { .. }
                | ShardError::RateLimitExceeded { .. }
        )
    }

    /// Check if the error is related to node health
    pub fn is_node_error(&self) -> bool {
        matches!(
            self,
            ShardError::NodeNotFound { .. }
                | ShardError::NodeUnhealthy { .. }
                | ShardError::InsufficientNodes { .. }
        )
    }

    /// Check if the error is related to data integrity
    pub fn is_data_error(&self) -> bool {
        matches!(
            self,
            ShardError::ChecksumMismatch { .. }
                | ShardError::DataCorruption { .. }
                | ShardError::InconsistentReplicas { .. }
                | ShardError::SerializationError { .. }
                | ShardError::DeserializationError { .. }
        )
    }

    /// Get error category for metrics and monitoring
    pub fn category(&self) -> &'static str {
        match self {
            ShardError::InvalidConfiguration { .. } => "configuration",
            ShardError::NodeNotFound { .. }
            | ShardError::NodeAlreadyExists { .. }
            | ShardError::InsufficientNodes { .. }
            | ShardError::NodeUnhealthy { .. } => "node",
            ShardError::EmptyHashRing
            | ShardError::HashFunctionError { .. }
            | ShardError::VirtualNodeCollision => "hash_ring",
            ShardError::MigrationInProgress { .. }
            | ShardError::MigrationFailed { .. }
            | ShardError::MigrationTimeout { .. }
            | ShardError::NoMigrationPlan => "migration",
            ShardError::ReplicationFailed { .. }
            | ShardError::QuorumNotAchieved { .. }
            | ShardError::ReplicaNotFound { .. }
            | ShardError::InconsistentReplicas { .. } => "replication",
            ShardError::NetworkError { .. }
            | ShardError::ConnectionFailed { .. }
            | ShardError::CommunicationTimeout { .. } => "network",
            ShardError::SerializationError { .. }
            | ShardError::DeserializationError { .. }
            | ShardError::ChecksumMismatch { .. }
            | ShardError::DataCorruption { .. } => "data",
            ShardError::StorageCapacityExceeded { .. }
            | ShardError::MemoryLimitExceeded { .. }
            | ShardError::RateLimitExceeded { .. } => "resource",
            ShardError::OperationNotSupported { .. }
            | ShardError::InvalidState { .. }
            | ShardError::InternalError { .. }
            | ShardError::IoError(_) => "generic",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categorization() {
        let config_error = ShardError::InvalidConfiguration {
            reason: "test".to_string(),
        };
        assert_eq!(config_error.category(), "configuration");
        assert!(!config_error.is_recoverable());

        let network_error = ShardError::NetworkError {
            reason: "test".to_string(),
        };
        assert_eq!(network_error.category(), "network");
        assert!(network_error.is_recoverable());

        let node_error = ShardError::NodeNotFound {
            node_id: "test".to_string(),
        };
        assert_eq!(node_error.category(), "node");
        assert!(node_error.is_node_error());

        let data_error = ShardError::DataCorruption {
            key: "test".to_string(),
        };
        assert_eq!(data_error.category(), "data");
        assert!(data_error.is_data_error());
    }

    #[test]
    fn test_error_recovery_classification() {
        let recoverable_errors = vec![
            ShardError::NetworkError {
                reason: "test".to_string(),
            },
            ShardError::CommunicationTimeout {
                node_id: "test".to_string(),
            },
            ShardError::QuorumNotAchieved {
                actual: 1,
                required: 2,
            },
        ];

        for error in recoverable_errors {
            assert!(error.is_recoverable(), "Error should be recoverable: {}", error);
        }

        let non_recoverable_errors = vec![
            ShardError::InvalidConfiguration {
                reason: "test".to_string(),
            },
            ShardError::DataCorruption {
                key: "test".to_string(),
            },
            ShardError::VirtualNodeCollision,
        ];

        for error in non_recoverable_errors {
            assert!(!error.is_recoverable(), "Error should not be recoverable: {}", error);
        }
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let shard_error: ShardError = io_error.into();

        assert!(matches!(shard_error, ShardError::IoError(_)));
        assert_eq!(shard_error.category(), "generic");
    }
}