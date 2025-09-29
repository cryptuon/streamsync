//! Message Log for PBFT Consensus
//!
//! This module manages the storage and retrieval of consensus messages,
//! including pre-prepare, prepare, commit, and checkpoint messages.

use super::ConsensusProposal;
use crate::network::protocol::{PreparedProof, NodeSignature};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Message log entry for consensus messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageLogEntry {
    pub view: u64,
    pub sequence: u64,
    pub digest: String,
    pub message_type: MessageType,
    pub sender: Uuid,
    pub timestamp: DateTime<Utc>,
    pub signature: Option<NodeSignature>,
}

/// Type of consensus message
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    PrePrepare,
    Prepare,
    Commit,
    Checkpoint,
}

/// Pre-prepare message data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrePrepareEntry {
    pub view: u64,
    pub sequence: u64,
    pub digest: String,
    pub proposal: ConsensusProposal,
    pub primary: Uuid,
    pub timestamp: DateTime<Utc>,
}

/// Checkpoint message data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointEntry {
    pub sequence: u64,
    pub state_hash: String,
    pub nodes: HashSet<Uuid>,
    pub timestamp: DateTime<Utc>,
}

/// Message log for storing and querying consensus messages
pub struct MessageLog {
    /// Pre-prepare messages indexed by (view, sequence)
    pre_prepares: HashMap<(u64, u64), PrePrepareEntry>,

    /// Prepare messages indexed by (view, sequence, digest) -> set of nodes
    prepares: HashMap<(u64, u64, String), HashSet<Uuid>>,

    /// Commit messages indexed by (view, sequence, digest) -> set of nodes
    commits: HashMap<(u64, u64, String), HashSet<Uuid>>,

    /// Checkpoint messages indexed by sequence -> checkpoint data
    checkpoints: HashMap<u64, CheckpointEntry>,

    /// Complete message log for replay/verification
    message_history: Vec<MessageLogEntry>,

    /// Garbage collection watermark
    low_watermark: u64,

    /// Maximum log size before cleanup
    max_log_size: usize,
}

impl MessageLog {
    /// Create a new message log
    pub fn new() -> Self {
        Self {
            pre_prepares: HashMap::new(),
            prepares: HashMap::new(),
            commits: HashMap::new(),
            checkpoints: HashMap::new(),
            message_history: Vec::new(),
            low_watermark: 0,
            max_log_size: 10000,
        }
    }

    /// Add a pre-prepare message
    pub fn add_pre_prepare(&mut self, view: u64, sequence: u64, digest: String, proposal: ConsensusProposal) {
        let entry = PrePrepareEntry {
            view,
            sequence,
            digest: digest.clone(),
            proposal,
            primary: Uuid::new_v4(), // This should be passed as parameter
            timestamp: Utc::now(),
        };

        self.pre_prepares.insert((view, sequence), entry);

        // Add to message history
        self.add_to_history(MessageLogEntry {
            view,
            sequence,
            digest,
            message_type: MessageType::PrePrepare,
            sender: Uuid::new_v4(), // This should be passed as parameter
            timestamp: Utc::now(),
            signature: None,
        });

        tracing::debug!("📝 Added pre-prepare for view {} sequence {}", view, sequence);
    }

    /// Add a prepare message
    pub fn add_prepare(&mut self, view: u64, sequence: u64, digest: String, node_id: Uuid) {
        let key = (view, sequence, digest.clone());
        let nodes = self.prepares.entry(key).or_insert_with(HashSet::new);

        if nodes.insert(node_id) {
            // Add to message history
            self.add_to_history(MessageLogEntry {
                view,
                sequence,
                digest,
                message_type: MessageType::Prepare,
                sender: node_id,
                timestamp: Utc::now(),
                signature: None,
            });

            tracing::debug!("📝 Added prepare from {} for view {} sequence {}", node_id, view, sequence);
        }
    }

    /// Add a commit message
    pub fn add_commit(&mut self, view: u64, sequence: u64, digest: String, node_id: Uuid) {
        let key = (view, sequence, digest.clone());
        let nodes = self.commits.entry(key).or_insert_with(HashSet::new);

        if nodes.insert(node_id) {
            // Add to message history
            self.add_to_history(MessageLogEntry {
                view,
                sequence,
                digest,
                message_type: MessageType::Commit,
                sender: node_id,
                timestamp: Utc::now(),
                signature: None,
            });

            tracing::debug!("📝 Added commit from {} for view {} sequence {}", node_id, view, sequence);
        }
    }

    /// Add a checkpoint message
    pub fn add_checkpoint(&mut self, sequence: u64, state_hash: String, node_id: Uuid) {
        let checkpoint = self.checkpoints.entry(sequence).or_insert_with(|| CheckpointEntry {
            sequence,
            state_hash: state_hash.clone(),
            nodes: HashSet::new(),
            timestamp: Utc::now(),
        });

        if checkpoint.nodes.insert(node_id) {
            // Add to message history
            self.add_to_history(MessageLogEntry {
                view: 0, // Checkpoints are view-independent
                sequence,
                digest: state_hash,
                message_type: MessageType::Checkpoint,
                sender: node_id,
                timestamp: Utc::now(),
                signature: None,
            });

            tracing::debug!("📝 Added checkpoint from {} for sequence {}", node_id, sequence);
        }
    }

    /// Get digest for a view/sequence pair
    pub fn get_digest(&self, view: u64, sequence: u64) -> Option<String> {
        self.pre_prepares.get(&(view, sequence)).map(|entry| entry.digest.clone())
    }

    /// Get proposal for a view/sequence pair
    pub fn get_proposal(&self, view: u64, sequence: u64) -> Option<ConsensusProposal> {
        self.pre_prepares.get(&(view, sequence)).map(|entry| entry.proposal.clone())
    }

    /// Count prepare messages for a specific proposal
    pub fn count_prepares(&self, view: u64, sequence: u64, digest: &str) -> usize {
        let key = (view, sequence, digest.to_string());
        self.prepares.get(&key).map_or(0, |nodes| nodes.len())
    }

    /// Count commit messages for a specific proposal
    pub fn count_commits(&self, view: u64, sequence: u64, digest: &str) -> usize {
        let key = (view, sequence, digest.to_string());
        self.commits.get(&key).map_or(0, |nodes| nodes.len())
    }

    /// Count checkpoint messages for a sequence
    pub fn count_checkpoints(&self, sequence: u64) -> usize {
        self.checkpoints.get(&sequence).map_or(0, |checkpoint| checkpoint.nodes.len())
    }

    /// Get nodes that sent prepare messages
    pub fn get_prepare_nodes(&self, view: u64, sequence: u64, digest: &str) -> HashSet<Uuid> {
        let key = (view, sequence, digest.to_string());
        self.prepares.get(&key).cloned().unwrap_or_default()
    }

    /// Get nodes that sent commit messages
    pub fn get_commit_nodes(&self, view: u64, sequence: u64, digest: &str) -> HashSet<Uuid> {
        let key = (view, sequence, digest.to_string());
        self.commits.get(&key).cloned().unwrap_or_default()
    }

    /// Check if we have a pre-prepare for a view/sequence
    pub fn has_pre_prepare(&self, view: u64, sequence: u64) -> bool {
        self.pre_prepares.contains_key(&(view, sequence))
    }

    /// Check if a node sent a prepare message
    pub fn has_prepare(&self, view: u64, sequence: u64, digest: &str, node_id: Uuid) -> bool {
        let key = (view, sequence, digest.to_string());
        self.prepares.get(&key).map_or(false, |nodes| nodes.contains(&node_id))
    }

    /// Check if a node sent a commit message
    pub fn has_commit(&self, view: u64, sequence: u64, digest: &str, node_id: Uuid) -> bool {
        let key = (view, sequence, digest.to_string());
        self.commits.get(&key).map_or(false, |nodes| nodes.contains(&node_id))
    }

    /// Get prepared proof for view change
    pub fn get_prepared_proof(&self, view: u64, sequence: u64) -> Option<PreparedProof> {
        if let Some(pre_prepare) = self.pre_prepares.get(&(view, sequence)) {
            let prepare_nodes = self.get_prepare_nodes(view, sequence, &pre_prepare.digest);

            if !prepare_nodes.is_empty() {
                return Some(PreparedProof {
                    view,
                    sequence,
                    digest: pre_prepare.digest.clone(),
                    prepare_signatures: prepare_nodes.into_iter()
                        .map(|node_id| NodeSignature {
                            node_id,
                            signature: "".to_string(), // Simplified for demo
                            timestamp: Utc::now(),
                        })
                        .collect(),
                });
            }
        }
        None
    }

    /// Get all prepared proofs for view change
    pub fn get_all_prepared_proofs(&self, max_view: u64) -> Vec<PreparedProof> {
        let mut proofs = Vec::new();

        for (&(view, sequence), _) in &self.pre_prepares {
            if view <= max_view {
                if let Some(proof) = self.get_prepared_proof(view, sequence) {
                    proofs.push(proof);
                }
            }
        }

        proofs.sort_by_key(|p| (p.view, p.sequence));
        proofs
    }

    /// Get latest stable checkpoint
    pub fn get_latest_stable_checkpoint(&self, quorum_size: usize) -> Option<(u64, String)> {
        self.checkpoints.iter()
            .filter(|(_, checkpoint)| checkpoint.nodes.len() >= quorum_size)
            .max_by_key(|(&sequence, _)| sequence)
            .map(|(&sequence, checkpoint)| (sequence, checkpoint.state_hash.clone()))
    }

    /// Cleanup messages before a sequence number
    pub fn cleanup_before_sequence(&mut self, sequence: u64) {
        self.low_watermark = sequence;

        // Clean up pre-prepares
        self.pre_prepares.retain(|&(_, seq), _| seq > sequence);

        // Clean up prepares
        self.prepares.retain(|&(_, seq, _), _| seq > sequence);

        // Clean up commits
        self.commits.retain(|&(_, seq, _), _| seq > sequence);

        // Clean up old checkpoints but keep the latest stable one
        if let Some((latest_stable, _)) = self.get_latest_stable_checkpoint(1) {
            self.checkpoints.retain(|&seq, _| seq >= latest_stable);
        }

        // Clean up message history
        self.message_history.retain(|entry| entry.sequence > sequence);

        tracing::info!("🗑️ Cleaned up messages before sequence {}", sequence);
    }

    /// Get message statistics
    pub fn get_stats(&self) -> MessageLogStats {
        MessageLogStats {
            pre_prepare_count: self.pre_prepares.len(),
            prepare_count: self.prepares.len(),
            commit_count: self.commits.len(),
            checkpoint_count: self.checkpoints.len(),
            total_messages: self.message_history.len(),
            low_watermark: self.low_watermark,
            latest_sequence: self.pre_prepares.keys()
                .map(|(_, seq)| *seq)
                .max()
                .unwrap_or(0),
        }
    }

    /// Verify message log consistency
    pub fn verify_consistency(&self) -> Result<()> {
        // Check that all prepare/commit messages have corresponding pre-prepares
        for &(view, sequence, _) in self.prepares.keys() {
            if !self.pre_prepares.contains_key(&(view, sequence)) {
                return Err(anyhow::anyhow!("Prepare without pre-prepare: view {} sequence {}", view, sequence));
            }
        }

        for &(view, sequence, _) in self.commits.keys() {
            if !self.pre_prepares.contains_key(&(view, sequence)) {
                return Err(anyhow::anyhow!("Commit without pre-prepare: view {} sequence {}", view, sequence));
            }
        }

        tracing::debug!("✅ Message log consistency verified");
        Ok(())
    }

    /// Export message log for debugging
    pub fn export_log(&self) -> Vec<MessageLogEntry> {
        self.message_history.clone()
    }

    /// Import message log (for testing/recovery)
    pub fn import_log(&mut self, entries: Vec<MessageLogEntry>) -> Result<()> {
        for entry in entries {
            match entry.message_type {
                MessageType::PrePrepare => {
                    // Would need full proposal data to reconstruct
                    tracing::warn!("Cannot fully reconstruct pre-prepare from log entry");
                }
                MessageType::Prepare => {
                    self.add_prepare(entry.view, entry.sequence, entry.digest, entry.sender);
                }
                MessageType::Commit => {
                    self.add_commit(entry.view, entry.sequence, entry.digest, entry.sender);
                }
                MessageType::Checkpoint => {
                    self.add_checkpoint(entry.sequence, entry.digest, entry.sender);
                }
            }
        }
        Ok(())
    }

    /// Add entry to message history
    fn add_to_history(&mut self, entry: MessageLogEntry) {
        self.message_history.push(entry);

        // Cleanup if log gets too large
        if self.message_history.len() > self.max_log_size {
            let remove_count = self.max_log_size / 4; // Remove 25% of old entries
            self.message_history.drain(0..remove_count);
        }
    }

    /// Get message sequence gaps
    pub fn find_sequence_gaps(&self) -> Vec<(u64, u64)> {
        let mut sequences: Vec<u64> = self.pre_prepares.keys()
            .map(|(_, seq)| *seq)
            .collect();

        if sequences.is_empty() {
            return vec![];
        }

        sequences.sort();
        let mut gaps = Vec::new();

        for i in 1..sequences.len() {
            let prev = sequences[i - 1];
            let curr = sequences[i];

            if curr > prev + 1 {
                gaps.push((prev + 1, curr - 1));
            }
        }

        gaps
    }
}

/// Message log statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageLogStats {
    pub pre_prepare_count: usize,
    pub prepare_count: usize,
    pub commit_count: usize,
    pub checkpoint_count: usize,
    pub total_messages: usize,
    pub low_watermark: u64,
    pub latest_sequence: u64,
}