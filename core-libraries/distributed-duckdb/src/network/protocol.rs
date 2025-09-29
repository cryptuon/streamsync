//! Network protocol definitions for StreamSync P2P communication

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Cryptographic signature from a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSignature {
    pub node_id: Uuid,
    pub signature: String,
    pub timestamp: DateTime<Utc>,
}

/// Prepared proof for PBFT consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedProof {
    pub view: u64,
    pub sequence: u64,
    pub digest: String,
    pub prepare_signatures: Vec<NodeSignature>,
}

/// View change proof for PBFT consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewChangeProof {
    pub new_view: u64,
    pub node_id: Uuid,
    pub prepared_proofs: Vec<PreparedProof>,
    pub signature: NodeSignature,
}

/// Network message types for P2P communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    /// Heartbeat to maintain connection
    Heartbeat {
        node_id: Uuid,
        timestamp: DateTime<Utc>,
    },

    /// Node discovery and announcement
    NodeAnnouncement {
        node_id: Uuid,
        listen_addr: String,
        capabilities: Vec<String>,
        version: u32,
    },

    /// Peer discovery request
    PeerDiscovery {
        requesting_node: Uuid,
        known_peers: Vec<PeerAddress>,
    },

    /// Response to peer discovery
    PeerDiscoveryResponse {
        responding_node: Uuid,
        peers: Vec<PeerAddress>,
    },

    /// Consensus-related messages
    Consensus(ConsensusMessage),

    /// Data distribution messages
    DataDistribution(DataMessage),

    /// Query execution messages
    Query(QueryMessage),

    /// Gossip protocol messages
    Gossip(GossipMessage),

    /// Network topology updates
    TopologyUpdate {
        sender: Uuid,
        topology_hash: String,
        updates: Vec<TopologyChange>,
    },

    /// Error responses
    Error {
        error_code: ErrorCode,
        message: String,
        request_id: Option<Uuid>,
    },
}

/// Peer address information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerAddress {
    pub node_id: Uuid,
    pub addr: String,
    pub last_seen: DateTime<Utc>,
    pub capabilities: Vec<String>,
}

/// PBFT consensus messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusMessage {
    /// Pre-prepare phase
    PrePrepare {
        view: u64,
        sequence: u64,
        digest: String,
        proposal: ConsensusProposal,
        primary: Uuid,
    },

    /// Prepare phase
    Prepare {
        view: u64,
        sequence: u64,
        digest: String,
        node_id: Uuid,
    },

    /// Commit phase
    Commit {
        view: u64,
        sequence: u64,
        digest: String,
        node_id: Uuid,
    },

    /// View change request
    ViewChange {
        new_view: u64,
        node_id: Uuid,
        prepared_proofs: Vec<PreparedProof>,
    },

    /// New view message
    NewView {
        view: u64,
        view_change_proofs: Vec<ViewChangeProof>,
        primary: Uuid,
    },

    /// Checkpoint for state synchronization
    Checkpoint {
        sequence: u64,
        state_hash: String,
        node_id: Uuid,
    },
}

/// Consensus proposal types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusProposal {
    /// IDL update proposal
    IdlUpdate {
        program_id: String,
        new_idl: String,
        proposer: Uuid,
    },

    /// Data shard assignment
    ShardAssignment {
        shard_id: String,
        assigned_nodes: Vec<Uuid>,
        replication_factor: u32,
    },

    /// Node addition/removal
    NodeManagement {
        action: NodeAction,
        target_node: Uuid,
        reason: String,
    },

    /// Configuration update
    ConfigUpdate {
        parameter: String,
        old_value: String,
        new_value: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeAction {
    Add,
    Remove,
    Suspend,
}

/// Data distribution messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataMessage {
    /// Data replication request
    ReplicationRequest {
        shard_id: String,
        data_hash: String,
        requesting_node: Uuid,
    },

    /// Data replication response
    ReplicationResponse {
        shard_id: String,
        data: Vec<u8>,
        responding_node: Uuid,
    },

    /// Shard migration
    ShardMigration {
        shard_id: String,
        from_node: Uuid,
        to_node: Uuid,
        data: Vec<u8>,
    },

    /// Data integrity check
    IntegrityCheck {
        shard_id: String,
        expected_hash: String,
        requesting_node: Uuid,
    },

    /// Data synchronization
    DataSync {
        shard_id: String,
        version: u64,
        delta: Vec<u8>,
        sender: Uuid,
    },
}

/// Query execution messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryMessage {
    /// Distributed query request
    QueryRequest {
        query_id: Uuid,
        sql: String,
        shards: Vec<String>,
        requesting_node: Uuid,
    },

    /// Query execution response
    QueryResponse {
        query_id: Uuid,
        results: Vec<u8>,
        shard_id: String,
        responding_node: Uuid,
    },

    /// Query aggregation request
    AggregationRequest {
        query_id: Uuid,
        partial_results: Vec<Vec<u8>>,
        aggregation_function: String,
    },

    /// Final query result
    QueryResult {
        query_id: Uuid,
        final_result: Vec<u8>,
        execution_stats: QueryStats,
    },

    /// Query cancellation
    QueryCancel {
        query_id: Uuid,
        reason: String,
    },
}

/// Gossip protocol messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GossipMessage {
    /// Rumor spreading
    Rumor {
        rumor_id: Uuid,
        content: RumorContent,
        ttl: u32,
        sender: Uuid,
    },

    /// Anti-entropy synchronization
    AntiEntropy {
        digest: HashMap<String, u64>,
        requesting_node: Uuid,
    },

    /// Anti-entropy response
    AntiEntropyResponse {
        missing_data: Vec<RumorContent>,
        responding_node: Uuid,
    },
}

/// Rumor content types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RumorContent {
    NodeJoin {
        node_id: Uuid,
        addr: String,
        capabilities: Vec<String>,
    },
    NodeLeave {
        node_id: Uuid,
        reason: String,
    },
    ShardUpdate {
        shard_id: String,
        new_assignment: Vec<Uuid>,
    },
    PerformanceMetric {
        node_id: Uuid,
        metric_type: String,
        value: f64,
        timestamp: DateTime<Utc>,
    },
}

/// Network topology changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TopologyChange {
    NodeAdded {
        node_id: Uuid,
        addr: String,
    },
    NodeRemoved {
        node_id: Uuid,
    },
    ConnectionAdded {
        from_node: Uuid,
        to_node: Uuid,
    },
    ConnectionRemoved {
        from_node: Uuid,
        to_node: Uuid,
    },
}

/// Error codes for network communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorCode {
    InvalidMessage,
    AuthenticationFailed,
    NotFound,
    InternalError,
    Timeout,
    ResourceUnavailable,
    ProtocolMismatch,
    ConsensusFailure,
}

/// Supporting structures

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryStats {
    pub execution_time_ms: u64,
    pub rows_processed: u64,
    pub bytes_scanned: u64,
    pub nodes_involved: Vec<Uuid>,
}

impl NetworkMessage {
    /// Get the message type as a string
    pub fn message_type(&self) -> &'static str {
        match self {
            NetworkMessage::Heartbeat { .. } => "heartbeat",
            NetworkMessage::NodeAnnouncement { .. } => "node_announcement",
            NetworkMessage::PeerDiscovery { .. } => "peer_discovery",
            NetworkMessage::PeerDiscoveryResponse { .. } => "peer_discovery_response",
            NetworkMessage::Consensus(_) => "consensus",
            NetworkMessage::DataDistribution(_) => "data_distribution",
            NetworkMessage::Query(_) => "query",
            NetworkMessage::Gossip(_) => "gossip",
            NetworkMessage::TopologyUpdate { .. } => "topology_update",
            NetworkMessage::Error { .. } => "error",
        }
    }

    /// Get the sender node ID if available
    pub fn sender(&self) -> Option<Uuid> {
        match self {
            NetworkMessage::Heartbeat { node_id, .. } => Some(*node_id),
            NetworkMessage::NodeAnnouncement { node_id, .. } => Some(*node_id),
            NetworkMessage::PeerDiscovery { requesting_node, .. } => Some(*requesting_node),
            NetworkMessage::PeerDiscoveryResponse { responding_node, .. } => Some(*responding_node),
            NetworkMessage::TopologyUpdate { sender, .. } => Some(*sender),
            _ => None,
        }
    }

    /// Check if this is a high-priority message
    pub fn is_high_priority(&self) -> bool {
        matches!(self,
            NetworkMessage::Consensus(_) |
            NetworkMessage::Error { .. } |
            NetworkMessage::Heartbeat { .. }
        )
    }
}