//! Gossip Protocol Implementation
//!
//! Stub implementation for gossip protocol

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::peer::Peer;
use super::NetworkConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipProtocol {
    running: bool,
}

impl GossipProtocol {
    pub fn new(_config: NetworkConfig, _peers: Arc<RwLock<HashMap<Uuid, Peer>>>) -> Result<Self> {
        Ok(Self { running: false })
    }

    pub async fn start(&mut self) -> Result<()> {
        self.running = true;
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        self.running = false;
        Ok(())
    }
}
