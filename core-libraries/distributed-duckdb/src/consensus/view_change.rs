//! View Change Management for PBFT Consensus
//!
//! This module handles view changes in the PBFT protocol, including
//! detecting when view changes are needed, collecting view change messages,
//! and coordinating the transition to new views.

use super::ConsensusConfig;
use crate::network::protocol::{PreparedProof, ViewChangeProof, NodeSignature};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// View change message data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewChangeMessage {
    pub new_view: u64,
    pub node_id: Uuid,
    pub prepared_proofs: Vec<PreparedProof>,
    pub last_checkpoint: u64,
    pub checkpoint_proof: Option<CheckpointProof>,
    pub timestamp: DateTime<Utc>,
    pub signature: Option<NodeSignature>,
}

/// Checkpoint proof for view change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointProof {
    pub sequence: u64,
    pub state_hash: String,
    pub signatures: Vec<NodeSignature>,
}

/// View change state tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewChangeState {
    pub target_view: u64,
    pub messages: Vec<ViewChangeMessage>,
    pub supporting_nodes: HashSet<Uuid>,
    pub started_at: DateTime<Utc>,
    pub timeout_at: DateTime<Utc>,
    pub completed: bool,
}

impl ViewChangeState {
    /// Create a new view change state
    pub fn new(target_view: u64, timeout_ms: u64) -> Self {
        let now = Utc::now();
        Self {
            target_view,
            messages: Vec::new(),
            supporting_nodes: HashSet::new(),
            started_at: now,
            timeout_at: now + chrono::Duration::milliseconds(timeout_ms as i64),
            completed: false,
        }
    }

    /// Add a view change message
    pub fn add_message(&mut self, message: ViewChangeMessage) -> bool {
        if message.new_view != self.target_view {
            return false;
        }

        if self.supporting_nodes.insert(message.node_id) {
            self.messages.push(message);
            true
        } else {
            false
        }
    }

    /// Check if we have enough support
    pub fn has_quorum(&self, quorum_size: usize) -> bool {
        self.supporting_nodes.len() >= quorum_size
    }

    /// Check if view change has timed out
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.timeout_at
    }

    /// Mark as completed
    pub fn complete(&mut self) {
        self.completed = true;
    }
}

/// Reason for triggering a view change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViewChangeReason {
    /// Primary timeout - no pre-prepare received
    PrimaryTimeout,
    /// Primary fault - invalid or malicious behavior
    PrimaryFault(String),
    /// Network partition detected
    NetworkPartition,
    /// Performance degradation
    PerformanceDegradation,
    /// Manual trigger for testing
    ManualTrigger,
    /// Recovery from checkpoint
    CheckpointRecovery,
}

/// View change trigger conditions
#[derive(Debug, Clone)]
pub struct ViewChangeTrigger {
    pub reason: ViewChangeReason,
    pub detected_at: DateTime<Utc>,
    pub evidence: Option<String>,
    pub reporting_node: Uuid,
}

/// Manager for PBFT view changes
pub struct ViewChangeManager {
    config: ConsensusConfig,
    current_view: u64,
    active_view_changes: HashMap<u64, ViewChangeState>,
    view_change_history: Vec<ViewChangeRecord>,

    // Trigger detection
    last_pre_prepare_time: Option<DateTime<Utc>>,
    performance_metrics: PerformanceTracker,

    // View change timeouts
    base_timeout_ms: u64,
    backoff_factor: f64,
    max_timeout_ms: u64,
}

/// Historical record of view changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewChangeRecord {
    pub from_view: u64,
    pub to_view: u64,
    pub reason: ViewChangeReason,
    pub triggered_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub participating_nodes: Vec<Uuid>,
    pub duration_ms: Option<u64>,
}

/// Performance tracking for view change triggers
#[derive(Debug, Clone)]
struct PerformanceTracker {
    recent_commit_times: Vec<u64>,
    average_commit_time: u64,
    commit_timeout_threshold: u64,
    last_update: DateTime<Utc>,
}

impl PerformanceTracker {
    fn new() -> Self {
        Self {
            recent_commit_times: Vec::new(),
            average_commit_time: 0,
            commit_timeout_threshold: 5000, // 5 seconds
            last_update: Utc::now(),
        }
    }

    fn record_commit_time(&mut self, time_ms: u64) {
        self.recent_commit_times.push(time_ms);

        // Keep only recent measurements
        if self.recent_commit_times.len() > 100 {
            self.recent_commit_times.remove(0);
        }

        // Update average
        self.average_commit_time = self.recent_commit_times.iter().sum::<u64>() / self.recent_commit_times.len() as u64;
        self.last_update = Utc::now();
    }

    fn is_performance_degraded(&self) -> bool {
        self.average_commit_time > self.commit_timeout_threshold && self.recent_commit_times.len() >= 10
    }
}

impl ViewChangeManager {
    /// Create a new view change manager
    pub fn new(config: ConsensusConfig) -> Self {
        Self {
            current_view: 0,
            base_timeout_ms: config.view_change_timeout_ms,
            backoff_factor: 1.5,
            max_timeout_ms: config.view_change_timeout_ms * 4,
            config,
            active_view_changes: HashMap::new(),
            view_change_history: Vec::new(),
            last_pre_prepare_time: None,
            performance_metrics: PerformanceTracker::new(),
        }
    }

    /// Update current view
    pub fn set_current_view(&mut self, view: u64) {
        self.current_view = view;

        // Clean up old view change states
        self.active_view_changes.retain(|&v, _| v > view);
    }

    /// Record pre-prepare received (resets timeout)
    pub fn record_pre_prepare(&mut self) {
        self.last_pre_prepare_time = Some(Utc::now());
    }

    /// Record commit time for performance tracking
    pub fn record_commit_time(&mut self, time_ms: u64) {
        self.performance_metrics.record_commit_time(time_ms);
    }

    /// Check if view change should be triggered
    pub fn should_trigger_view_change(&self) -> Option<ViewChangeTrigger> {
        // Check for primary timeout
        if let Some(last_pre_prepare) = self.last_pre_prepare_time {
            let timeout_duration = chrono::Duration::milliseconds(self.base_timeout_ms as i64);
            if Utc::now() > last_pre_prepare + timeout_duration {
                return Some(ViewChangeTrigger {
                    reason: ViewChangeReason::PrimaryTimeout,
                    detected_at: Utc::now(),
                    evidence: Some(format!("No pre-prepare for {} ms",
                                         (Utc::now() - last_pre_prepare).num_milliseconds())),
                    reporting_node: self.config.node_id,
                });
            }
        } else {
            // No pre-prepare ever received
            let startup_timeout = chrono::Duration::milliseconds((self.base_timeout_ms * 2) as i64);
            if Utc::now() > self.performance_metrics.last_update + startup_timeout {
                return Some(ViewChangeTrigger {
                    reason: ViewChangeReason::PrimaryTimeout,
                    detected_at: Utc::now(),
                    evidence: Some("No pre-prepare received since startup".to_string()),
                    reporting_node: self.config.node_id,
                });
            }
        }

        // Check for performance degradation
        if self.performance_metrics.is_performance_degraded() {
            return Some(ViewChangeTrigger {
                reason: ViewChangeReason::PerformanceDegradation,
                detected_at: Utc::now(),
                evidence: Some(format!("Average commit time: {} ms",
                                     self.performance_metrics.average_commit_time)),
                reporting_node: self.config.node_id,
            });
        }

        None
    }

    /// Initiate a view change
    pub fn initiate_view_change(&mut self, reason: ViewChangeReason) -> Result<u64> {
        let new_view = self.current_view + 1;

        tracing::info!("🔄 Initiating view change from {} to {} due to {:?}",
                      self.current_view, new_view, reason);

        // Calculate timeout with exponential backoff
        let timeout_ms = self.calculate_view_change_timeout(new_view);

        // Create view change state
        let view_change_state = ViewChangeState::new(new_view, timeout_ms);
        self.active_view_changes.insert(new_view, view_change_state);

        // Record in history
        let record = ViewChangeRecord {
            from_view: self.current_view,
            to_view: new_view,
            reason,
            triggered_at: Utc::now(),
            completed_at: None,
            participating_nodes: vec![self.config.node_id],
            duration_ms: None,
        };
        self.view_change_history.push(record);

        Ok(new_view)
    }

    /// Add a view change message from another node
    pub fn add_view_change(&mut self, new_view: u64, node_id: Uuid, prepared_proofs: Vec<PreparedProof>) {
        tracing::debug!("📨 Received view change for view {} from {}", new_view, node_id);

        // Create view change state if it doesn't exist
        if !self.active_view_changes.contains_key(&new_view) {
            let timeout_ms = self.calculate_view_change_timeout(new_view);
            let view_change_state = ViewChangeState::new(new_view, timeout_ms);
            self.active_view_changes.insert(new_view, view_change_state);
        }

        // Add the message
        if let Some(state) = self.active_view_changes.get_mut(&new_view) {
            let message = ViewChangeMessage {
                new_view,
                node_id,
                prepared_proofs,
                last_checkpoint: 0, // Should be provided
                checkpoint_proof: None,
                timestamp: Utc::now(),
                signature: None,
            };

            state.add_message(message);
        }
    }

    /// Check if we have quorum for a view change
    pub fn has_quorum(&self, view: u64, quorum_size: usize) -> bool {
        self.active_view_changes
            .get(&view)
            .map_or(false, |state| state.has_quorum(quorum_size))
    }

    /// Get view change proofs for new-view message
    pub fn get_view_change_proofs(&self, view: u64) -> Vec<ViewChangeProof> {
        if let Some(state) = self.active_view_changes.get(&view) {
            state.messages.iter().map(|msg| ViewChangeProof {
                new_view: msg.new_view,
                node_id: msg.node_id,
                prepared_proofs: msg.prepared_proofs.clone(),
                signature: msg.signature.clone().unwrap_or(NodeSignature {
                    node_id: msg.node_id,
                    signature: "".to_string(),
                    timestamp: msg.timestamp,
                }),
            }).collect()
        } else {
            Vec::new()
        }
    }

    /// Complete a view change
    pub fn complete_view_change(&mut self, view: u64) -> Result<()> {
        if let Some(state) = self.active_view_changes.get_mut(&view) {
            state.complete();

            // Update history record
            if let Some(record) = self.view_change_history.iter_mut()
                .find(|r| r.to_view == view && r.completed_at.is_none()) {
                record.completed_at = Some(Utc::now());
                record.duration_ms = Some((Utc::now() - record.triggered_at).num_milliseconds() as u64);
                record.participating_nodes = state.supporting_nodes.iter().cloned().collect();
            }

            self.current_view = view;

            tracing::info!("✅ Completed view change to view {} with {} nodes",
                          view, state.supporting_nodes.len());
        }

        Ok(())
    }

    /// Cleanup expired view changes
    pub fn cleanup_expired(&mut self) -> Vec<u64> {
        let mut expired_views = Vec::new();

        self.active_view_changes.retain(|&view, state| {
            if state.is_expired() && !state.completed {
                expired_views.push(view);

                // Mark as failed in history
                if let Some(record) = self.view_change_history.iter_mut()
                    .find(|r| r.to_view == view && r.completed_at.is_none()) {
                    record.completed_at = Some(Utc::now());
                    record.duration_ms = Some((Utc::now() - record.triggered_at).num_milliseconds() as u64);
                }

                false
            } else {
                true
            }
        });

        if !expired_views.is_empty() {
            tracing::warn!("⏰ Expired view changes: {:?}", expired_views);
        }

        expired_views
    }

    /// Get view change statistics
    pub fn get_stats(&self) -> ViewChangeStats {
        let active_count = self.active_view_changes.len();
        let total_changes = self.view_change_history.len();
        let successful_changes = self.view_change_history.iter()
            .filter(|r| r.completed_at.is_some())
            .count();

        let average_duration = if successful_changes > 0 {
            self.view_change_history.iter()
                .filter_map(|r| r.duration_ms)
                .sum::<u64>() / successful_changes as u64
        } else {
            0
        };

        ViewChangeStats {
            current_view: self.current_view,
            active_view_changes: active_count,
            total_view_changes: total_changes,
            successful_view_changes: successful_changes,
            average_duration_ms: average_duration,
            last_view_change: self.view_change_history.last()
                .and_then(|r| r.completed_at)
                .unwrap_or_else(|| Utc::now()),
        }
    }

    /// Calculate view change timeout with exponential backoff
    fn calculate_view_change_timeout(&self, view: u64) -> u64 {
        let view_changes = view - self.current_view;
        let timeout = (self.base_timeout_ms as f64 * self.backoff_factor.powi(view_changes as i32)) as u64;
        timeout.min(self.max_timeout_ms)
    }

    /// Detect primary fault based on behavior
    pub fn detect_primary_fault(&self, evidence: &str) -> Option<ViewChangeTrigger> {
        Some(ViewChangeTrigger {
            reason: ViewChangeReason::PrimaryFault(evidence.to_string()),
            detected_at: Utc::now(),
            evidence: Some(evidence.to_string()),
            reporting_node: self.config.node_id,
        })
    }

    /// Get recent view change history
    pub fn get_recent_history(&self, limit: usize) -> Vec<&ViewChangeRecord> {
        let start = if self.view_change_history.len() > limit {
            self.view_change_history.len() - limit
        } else {
            0
        };

        self.view_change_history[start..].iter().collect()
    }

    /// Check if node is participating in view change
    pub fn is_participating_in_view_change(&self, view: u64, node_id: Uuid) -> bool {
        self.active_view_changes
            .get(&view)
            .map_or(false, |state| state.supporting_nodes.contains(&node_id))
    }
}

/// View change statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewChangeStats {
    pub current_view: u64,
    pub active_view_changes: usize,
    pub total_view_changes: usize,
    pub successful_view_changes: usize,
    pub average_duration_ms: u64,
    pub last_view_change: DateTime<Utc>,
}