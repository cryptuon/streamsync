//! Network Topology Management
//!
//! Stub implementation for network topology management

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTopology {
    nodes: HashMap<Uuid, SocketAddr>,
}

impl NetworkTopology {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub async fn add_peer(&mut self, node_id: Uuid, addr: SocketAddr) -> Result<()> {
        self.nodes.insert(node_id, addr);
        Ok(())
    }

    pub async fn remove_peer(&mut self, node_id: Uuid) -> Result<()> {
        self.nodes.remove(&node_id);
        Ok(())
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn diameter(&self) -> usize {
        // Stub: return 1 for simple topology
        if self.nodes.is_empty() { 0 } else { 1 }
    }

    pub fn clustering_coefficient(&self) -> f64 {
        // Stub: return 0.0 for simple topology
        0.0
    }
}

impl Default for NetworkTopology {
    fn default() -> Self {
        Self::new()
    }
}
