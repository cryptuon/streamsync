//! Configuration for the consensus engine

use crate::types::NodeId;
use crate::error::{ConsensusError, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for the consensus engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// This node's identifier
    pub node_id: NodeId,

    /// List of all participants in the consensus group
    pub participants: Vec<NodeId>,

    /// Maximum number of Byzantine faults tolerated
    pub max_faults: usize,

    /// Timeout for consensus requests
    pub request_timeout: Duration,

    /// Timeout for view changes
    pub view_change_timeout: Duration,

    /// Interval between checkpoints (in number of committed proposals)
    pub checkpoint_interval: u64,

    /// Maximum number of proposals to buffer before blocking
    pub max_pending_proposals: usize,

    /// Maximum size of a proposal in bytes
    pub max_proposal_size: usize,

    /// Enable detailed logging for debugging
    pub enable_debug_logs: bool,

    /// Batch size for processing messages
    pub message_batch_size: usize,

    /// Buffer size for incoming messages
    pub message_buffer_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            node_id: uuid::Uuid::new_v4(),
            participants: vec![],
            max_faults: 1,
            request_timeout: Duration::from_millis(5000),
            view_change_timeout: Duration::from_millis(10000),
            checkpoint_interval: 100,
            max_pending_proposals: 1000,
            max_proposal_size: 1024 * 1024, // 1MB
            enable_debug_logs: false,
            message_batch_size: 100,
            message_buffer_size: 10000,
        }
    }
}

impl Config {
    /// Create a new configuration with the given node ID and participants
    pub fn new(node_id: NodeId, participants: Vec<NodeId>) -> Self {
        let mut config = Self::default();
        config.node_id = node_id;
        config.participants = participants;
        config.max_faults = config.calculate_max_faults();
        config
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.participants.is_empty() {
            return Err(ConsensusError::Configuration {
                message: "No participants specified".to_string(),
            });
        }

        if !self.participants.contains(&self.node_id) {
            return Err(ConsensusError::NodeNotParticipant {
                node_id: self.node_id,
            });
        }

        let n = self.participants.len();

        // Byzantine consensus requires at least 4 nodes to tolerate 1 fault
        if n < 4 {
            return Err(ConsensusError::InsufficientNodes {
                required: 4,
                actual: n,
            });
        }

        let required_nodes = 3 * self.max_faults + 1;
        if n < required_nodes {
            return Err(ConsensusError::InsufficientNodes {
                required: required_nodes,
                actual: n,
            });
        }

        if self.checkpoint_interval == 0 {
            return Err(ConsensusError::Configuration {
                message: "Checkpoint interval must be greater than 0".to_string(),
            });
        }

        if self.max_proposal_size == 0 {
            return Err(ConsensusError::Configuration {
                message: "Max proposal size must be greater than 0".to_string(),
            });
        }

        if self.request_timeout.is_zero() {
            return Err(ConsensusError::Configuration {
                message: "Request timeout must be greater than 0".to_string(),
            });
        }

        if self.view_change_timeout.is_zero() {
            return Err(ConsensusError::Configuration {
                message: "View change timeout must be greater than 0".to_string(),
            });
        }

        Ok(())
    }

    /// Calculate the maximum number of Byzantine faults based on participant count
    pub fn calculate_max_faults(&self) -> usize {
        if self.participants.len() < 4 {
            0
        } else {
            (self.participants.len() - 1) / 3
        }
    }

    /// Get the quorum size (minimum number of nodes for consensus)
    pub fn quorum_size(&self) -> usize {
        2 * self.max_faults + 1
    }

    /// Get the primary node for a given view
    pub fn primary_for_view(&self, view: u64) -> Option<NodeId> {
        if self.participants.is_empty() {
            return None;
        }

        let index = (view as usize) % self.participants.len();
        self.participants.get(index).copied()
    }

    /// Check if this node is the primary for the given view
    pub fn is_primary(&self, view: u64) -> bool {
        self.primary_for_view(view) == Some(self.node_id)
    }

    /// Get the index of this node in the participants list
    pub fn node_index(&self) -> Option<usize> {
        self.participants.iter().position(|&id| id == self.node_id)
    }

    /// Check if a node is a participant
    pub fn is_participant(&self, node_id: NodeId) -> bool {
        self.participants.contains(&node_id)
    }

    /// Get participant count
    pub fn participant_count(&self) -> usize {
        self.participants.len()
    }

    /// Set request timeout
    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// Set view change timeout
    pub fn with_view_change_timeout(mut self, timeout: Duration) -> Self {
        self.view_change_timeout = timeout;
        self
    }

    /// Set checkpoint interval
    pub fn with_checkpoint_interval(mut self, interval: u64) -> Self {
        self.checkpoint_interval = interval;
        self
    }

    /// Enable debug logging
    pub fn with_debug_logs(mut self, enable: bool) -> Self {
        self.enable_debug_logs = enable;
        self
    }

    /// Set maximum proposal size
    pub fn with_max_proposal_size(mut self, size: usize) -> Self {
        self.max_proposal_size = size;
        self
    }

    /// Create a minimal configuration for testing with the given number of nodes
    pub fn test_config(num_nodes: usize) -> Self {
        let participants: Vec<NodeId> = (0..num_nodes).map(|_| uuid::Uuid::new_v4()).collect();
        let node_id = participants[0];

        Self::new(node_id, participants)
            .with_request_timeout(Duration::from_millis(1000))
            .with_view_change_timeout(Duration::from_millis(2000))
            .with_checkpoint_interval(10)
            .with_debug_logs(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();

        // Empty participants should fail
        assert!(config.validate().is_err());

        // Add participants
        let node_id = Uuid::new_v4();
        config.node_id = node_id;
        config.participants = vec![node_id, Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
        config.max_faults = config.calculate_max_faults();

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_max_faults_calculation() {
        let config = Config::test_config(4);
        assert_eq!(config.calculate_max_faults(), 1);

        let config = Config::test_config(7);
        assert_eq!(config.calculate_max_faults(), 2);

        let config = Config::test_config(3);
        assert_eq!(config.calculate_max_faults(), 0);
    }

    #[test]
    fn test_quorum_size() {
        let config = Config::test_config(4);
        assert_eq!(config.quorum_size(), 3); // 2 * 1 + 1

        let config = Config::test_config(7);
        assert_eq!(config.quorum_size(), 5); // 2 * 2 + 1
    }

    #[test]
    fn test_primary_selection() {
        let config = Config::test_config(4);

        // View 0 should select first participant
        assert_eq!(config.primary_for_view(0), Some(config.participants[0]));

        // View 1 should select second participant
        assert_eq!(config.primary_for_view(1), Some(config.participants[1]));

        // View should wrap around
        assert_eq!(config.primary_for_view(4), Some(config.participants[0]));
    }

    #[test]
    fn test_is_primary() {
        let config = Config::test_config(4);

        // First node should be primary for view 0
        assert!(config.is_primary(0));
        assert!(!config.is_primary(1));
    }

    #[test]
    fn test_insufficient_nodes() {
        let participants = vec![Uuid::new_v4(), Uuid::new_v4()]; // Only 2 nodes
        let config = Config::new(participants[0], participants);

        match config.validate() {
            Err(ConsensusError::InsufficientNodes { required, actual }) => {
                assert_eq!(required, 4); // Byzantine consensus minimum
                assert_eq!(actual, 2);
            }
            _ => panic!("Expected InsufficientNodes error"),
        }
    }

    #[test]
    fn test_node_not_participant() {
        let participants = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
        let non_participant = Uuid::new_v4();
        let config = Config::new(non_participant, participants);

        match config.validate() {
            Err(ConsensusError::NodeNotParticipant { node_id }) => {
                assert_eq!(node_id, non_participant);
            }
            _ => panic!("Expected NodeNotParticipant error"),
        }
    }
}