//! Abstract transport layer for consensus messages

use crate::types::NodeId;
use crate::messages::ConsensusMessage;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// A message to be sent over the transport layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Source node
    pub from: NodeId,
    /// Destination node (None for broadcast)
    pub to: Option<NodeId>,
    /// Message payload
    pub payload: ConsensusMessage,
    /// Message ID for deduplication
    pub message_id: String,
}

impl Message {
    /// Create a new message
    pub fn new(from: NodeId, to: Option<NodeId>, payload: ConsensusMessage) -> Self {
        Self {
            from,
            to,
            payload,
            message_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// Create a broadcast message
    pub fn broadcast(from: NodeId, payload: ConsensusMessage) -> Self {
        Self::new(from, None, payload)
    }

    /// Create a unicast message
    pub fn unicast(from: NodeId, to: NodeId, payload: ConsensusMessage) -> Self {
        Self::new(from, Some(to), payload)
    }

    /// Check if this is a broadcast message
    pub fn is_broadcast(&self) -> bool {
        self.to.is_none()
    }

    /// Get the size of the message in bytes (approximate)
    pub fn size(&self) -> usize {
        // Simplified size calculation
        std::mem::size_of::<NodeId>() * 2 +
        std::mem::size_of::<ConsensusMessage>() +
        self.message_id.len()
    }
}

/// Transport statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransportStats {
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received
    pub messages_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Number of send failures
    pub send_failures: u64,
    /// Number of receive failures
    pub receive_failures: u64,
    /// Average message latency in milliseconds
    pub average_latency_ms: f64,
    /// Number of connected peers
    pub connected_peers: usize,
}

/// Abstract transport layer trait
#[async_trait]
pub trait Transport: Send + Sync + Debug {
    /// Start the transport layer
    async fn start(&mut self) -> crate::Result<()>;

    /// Stop the transport layer
    async fn stop(&mut self) -> crate::Result<()>;

    /// Send a message to a specific node or broadcast to all
    async fn send(&self, message: Message) -> crate::Result<()>;

    /// Receive the next message
    async fn receive(&self) -> crate::Result<Message>;

    /// Get list of connected peers
    async fn connected_peers(&self) -> Vec<NodeId>;

    /// Check if a peer is connected
    async fn is_connected(&self, peer: NodeId) -> bool;

    /// Get transport statistics
    async fn stats(&self) -> TransportStats;

    /// Set message handler (optional, for event-driven transports)
    async fn set_message_handler(&mut self, _handler: Box<dyn MessageHandler>) -> crate::Result<()> {
        Ok(()) // Default implementation does nothing
    }
}

/// Message handler trait for event-driven transports
#[async_trait]
pub trait MessageHandler: Send + Sync {
    /// Handle an incoming message
    async fn handle_message(&self, message: Message) -> crate::Result<()>;
}

/// Mock transport for testing
#[derive(Debug)]
pub struct MockTransport {
    #[allow(dead_code)]
    node_id: NodeId,
    peers: Vec<NodeId>,
    message_queue: tokio::sync::Mutex<Vec<Message>>,
    stats: tokio::sync::RwLock<TransportStats>,
    running: tokio::sync::RwLock<bool>,
}

impl MockTransport {
    /// Create a new mock transport
    pub fn new(node_id: NodeId, peers: Vec<NodeId>) -> Self {
        Self {
            node_id,
            peers,
            message_queue: tokio::sync::Mutex::new(Vec::new()),
            stats: tokio::sync::RwLock::new(TransportStats::default()),
            running: tokio::sync::RwLock::new(false),
        }
    }

    /// Add a message to the receive queue (for testing)
    pub async fn add_message(&self, message: Message) {
        let message_size = message.size();
        let mut queue = self.message_queue.lock().await;
        queue.push(message);

        let mut stats = self.stats.write().await;
        stats.messages_received += 1;
        stats.bytes_received += message_size as u64;
    }

    /// Get number of messages in queue
    pub async fn queue_len(&self) -> usize {
        self.message_queue.lock().await.len()
    }
}

#[async_trait]
impl Transport for MockTransport {
    async fn start(&mut self) -> crate::Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(crate::ConsensusError::AlreadyRunning.into());
        }
        *running = true;
        Ok(())
    }

    async fn stop(&mut self) -> crate::Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(crate::ConsensusError::NotRunning.into());
        }
        *running = false;
        Ok(())
    }

    async fn send(&self, message: Message) -> crate::Result<()> {
        let running = self.running.read().await;
        if !*running {
            return Err(crate::ConsensusError::NotRunning.into());
        }

        // Simulate sending (just update stats)
        let message_size = message.size();
        let mut stats = self.stats.write().await;
        stats.messages_sent += 1;
        stats.bytes_sent += message_size as u64;

        // Simulate occasional failures
        if rand::random::<f64>() < 0.01 { // 1% failure rate
            stats.send_failures += 1;
            return Err(crate::ConsensusError::Transport {
                message: "Simulated send failure".to_string(),
            }.into());
        }

        Ok(())
    }

    async fn receive(&self) -> crate::Result<Message> {
        let running = self.running.read().await;
        if !*running {
            return Err(crate::ConsensusError::NotRunning.into());
        }
        drop(running);

        // Try to get a message from the queue
        let mut queue = self.message_queue.lock().await;
        if let Some(message) = queue.pop() {
            Ok(message)
        } else {
            // No messages available, would typically block or return timeout
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            Err(crate::ConsensusError::Timeout { timeout_ms: 10 }.into())
        }
    }

    async fn connected_peers(&self) -> Vec<NodeId> {
        self.peers.clone()
    }

    async fn is_connected(&self, peer: NodeId) -> bool {
        self.peers.contains(&peer)
    }

    async fn stats(&self) -> TransportStats {
        self.stats.read().await.clone()
    }
}

/// In-memory transport for testing multiple nodes
#[derive(Debug)]
pub struct InMemoryTransport {
    node_id: NodeId,
    shared_state: std::sync::Arc<tokio::sync::RwLock<InMemoryState>>,
    stats: tokio::sync::RwLock<TransportStats>,
    running: tokio::sync::RwLock<bool>,
}

impl Clone for InMemoryTransport {
    fn clone(&self) -> Self {
        Self {
            node_id: self.node_id,
            shared_state: self.shared_state.clone(),
            stats: tokio::sync::RwLock::new(TransportStats::default()),
            running: tokio::sync::RwLock::new(false),
        }
    }
}

#[derive(Debug)]
pub struct InMemoryState {
    /// Messages for each node
    messages: std::collections::HashMap<NodeId, Vec<Message>>,
    /// Connected nodes
    nodes: std::collections::HashSet<NodeId>,
}

impl InMemoryTransport {
    /// Create a new in-memory transport
    pub fn new(node_id: NodeId, shared_state: std::sync::Arc<tokio::sync::RwLock<InMemoryState>>) -> Self {
        Self {
            node_id,
            shared_state,
            stats: tokio::sync::RwLock::new(TransportStats::default()),
            running: tokio::sync::RwLock::new(false),
        }
    }

    /// Create a network of in-memory transports
    pub fn create_network(node_ids: Vec<NodeId>) -> std::collections::HashMap<NodeId, InMemoryTransport> {
        let shared_state = std::sync::Arc::new(tokio::sync::RwLock::new(InMemoryState {
            messages: std::collections::HashMap::new(),
            nodes: node_ids.iter().cloned().collect(),
        }));

        node_ids
            .into_iter()
            .map(|node_id| (node_id, InMemoryTransport::new(node_id, shared_state.clone())))
            .collect()
    }
}

#[async_trait]
impl Transport for InMemoryTransport {
    async fn start(&mut self) -> crate::Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(crate::ConsensusError::AlreadyRunning.into());
        }
        *running = true;

        // Initialize message queue for this node
        let mut state = self.shared_state.write().await;
        state.messages.entry(self.node_id).or_insert_with(Vec::new);

        Ok(())
    }

    async fn stop(&mut self) -> crate::Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(crate::ConsensusError::NotRunning.into());
        }
        *running = false;
        Ok(())
    }

    async fn send(&self, message: Message) -> crate::Result<()> {
        let running = self.running.read().await;
        if !*running {
            return Err(crate::ConsensusError::NotRunning.into());
        }
        drop(running);

        let message_size = message.size();
        let mut state = self.shared_state.write().await;

        if message.is_broadcast() {
            // Send to all nodes except sender
            let nodes_to_send: Vec<_> = state.nodes.iter()
                .filter(|&&node_id| node_id != self.node_id)
                .cloned()
                .collect();

            for node_id in nodes_to_send {
                let queue = state.messages.entry(node_id).or_insert_with(Vec::new);
                queue.push(message.clone());
            }
        } else if let Some(target) = message.to {
            // Send to specific node
            if state.nodes.contains(&target) {
                let queue = state.messages.entry(target).or_insert_with(Vec::new);
                queue.push(message);
            } else {
                return Err(crate::ConsensusError::Transport {
                    message: format!("Target node {} not found", target),
                }.into());
            }
        }
        drop(state);

        let mut stats = self.stats.write().await;
        stats.messages_sent += 1;
        stats.bytes_sent += message_size as u64;

        Ok(())
    }

    async fn receive(&self) -> crate::Result<Message> {
        let running = self.running.read().await;
        if !*running {
            return Err(crate::ConsensusError::NotRunning.into());
        }
        drop(running);

        // Try to get a message without holding the lock during sleep
        {
            let mut state = self.shared_state.write().await;
            let queue = state.messages.entry(self.node_id).or_insert_with(Vec::new);

            // Use remove(0) for FIFO order instead of pop() which is LIFO
            // This ensures messages are delivered in the order they were sent
            if !queue.is_empty() {
                let message = queue.remove(0);
                drop(state); // Release lock before updating stats
                let mut stats = self.stats.write().await;
                stats.messages_received += 1;
                stats.bytes_received += message.size() as u64;
                return Ok(message);
            }
            // Lock is released here when state goes out of scope
        }

        // Sleep WITHOUT holding the lock
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        Err(crate::ConsensusError::Timeout { timeout_ms: 1 }.into())
    }

    async fn connected_peers(&self) -> Vec<NodeId> {
        let state = self.shared_state.read().await;
        state.nodes.iter().filter(|&&id| id != self.node_id).cloned().collect()
    }

    async fn is_connected(&self, peer: NodeId) -> bool {
        let state = self.shared_state.read().await;
        state.nodes.contains(&peer)
    }

    async fn stats(&self) -> TransportStats {
        let stats = self.stats.read().await;
        let mut result = stats.clone();
        result.connected_peers = self.connected_peers().await.len();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::MessageType;
    use uuid::Uuid;

    #[test]
    fn test_message_creation() {
        let from = Uuid::new_v4();
        let to = Uuid::new_v4();
        let payload = ConsensusMessage {
            message_type: MessageType::PrePrepare,
            view: 0,
            sequence: 1,
            data: vec![],
            signature: None,
        };

        let broadcast = Message::broadcast(from, payload.clone());
        assert!(broadcast.is_broadcast());
        assert_eq!(broadcast.from, from);
        assert_eq!(broadcast.to, None);

        let unicast = Message::unicast(from, to, payload);
        assert!(!unicast.is_broadcast());
        assert_eq!(unicast.from, from);
        assert_eq!(unicast.to, Some(to));
    }

    #[tokio::test]
    async fn test_mock_transport() {
        let node_id = Uuid::new_v4();
        let peers = vec![Uuid::new_v4(), Uuid::new_v4()];
        let mut transport = MockTransport::new(node_id, peers);

        // Start transport
        assert!(transport.start().await.is_ok());

        // Send a message
        let payload = ConsensusMessage {
            message_type: MessageType::Prepare,
            view: 0,
            sequence: 1,
            data: vec![],
            signature: None,
        };
        let message = Message::broadcast(node_id, payload);
        assert!(transport.send(message).await.is_ok());

        // Check stats
        let stats = transport.stats().await;
        assert_eq!(stats.messages_sent, 1);

        // Stop transport
        assert!(transport.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_in_memory_transport() {
        let node1 = Uuid::new_v4();
        let node2 = Uuid::new_v4();
        let nodes = vec![node1, node2];

        let mut transports = InMemoryTransport::create_network(nodes);

        let mut transport1 = transports.remove(&node1).unwrap();
        let mut transport2 = transports.remove(&node2).unwrap();

        // Start both transports
        assert!(transport1.start().await.is_ok());
        assert!(transport2.start().await.is_ok());

        // Send message from node1 to node2
        let payload = ConsensusMessage {
            message_type: MessageType::Commit,
            view: 0,
            sequence: 1,
            data: vec![1, 2, 3],
            signature: None,
        };
        let message = Message::unicast(node1, node2, payload.clone());
        assert!(transport1.send(message).await.is_ok());

        // Receive message at node2
        let received = transport2.receive().await.unwrap();
        assert_eq!(received.from, node1);
        assert_eq!(received.to, Some(node2));
        assert_eq!(received.payload.data, vec![1, 2, 3]);

        // Test broadcast
        let broadcast_message = Message::broadcast(node2, payload);
        assert!(transport2.send(broadcast_message).await.is_ok());

        // Node1 should receive the broadcast
        let received_broadcast = transport1.receive().await.unwrap();
        assert_eq!(received_broadcast.from, node2);
        assert_eq!(received_broadcast.to, None);
    }
}