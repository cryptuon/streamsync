//! Network metrics and monitoring

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;

/// Network metrics collector
#[derive(Debug)]
pub struct NetworkMetrics {
    /// Connection metrics
    connection_metrics: Arc<ConnectionMetrics>,
    /// Message metrics
    message_metrics: Arc<MessageMetrics>,
    /// Bandwidth metrics
    bandwidth_metrics: Arc<BandwidthMetrics>,
    /// Error metrics
    error_metrics: Arc<ErrorMetrics>,
    /// Latency metrics
    latency_metrics: Arc<RwLock<LatencyMetrics>>,
    /// Per-peer metrics
    peer_metrics: Arc<RwLock<HashMap<String, PeerMetrics>>>,
    /// Collection start time
    start_time: Instant,
}

/// Connection-related metrics
#[derive(Debug, Default)]
pub struct ConnectionMetrics {
    /// Total connections established
    pub total_connections: AtomicU64,
    /// Current active connections
    pub active_connections: AtomicUsize,
    /// Total connection failures
    pub connection_failures: AtomicU64,
    /// Total connection timeouts
    pub connection_timeouts: AtomicU64,
    /// Average connection duration (milliseconds)
    pub avg_connection_duration_ms: AtomicU64,
    /// Peak concurrent connections
    pub peak_connections: AtomicUsize,
}

/// Message-related metrics
#[derive(Debug, Default)]
pub struct MessageMetrics {
    /// Total messages sent
    pub messages_sent: AtomicU64,
    /// Total messages received
    pub messages_received: AtomicU64,
    /// Total messages dropped
    pub messages_dropped: AtomicU64,
    /// Total broadcast messages
    pub broadcast_messages: AtomicU64,
    /// Total unicast messages
    pub unicast_messages: AtomicU64,
    /// Message send failures
    pub send_failures: AtomicU64,
    /// Message receive failures
    pub receive_failures: AtomicU64,
}

/// Bandwidth-related metrics
#[derive(Debug, Default)]
pub struct BandwidthMetrics {
    /// Total bytes sent
    pub bytes_sent: AtomicU64,
    /// Total bytes received
    pub bytes_received: AtomicU64,
    /// Current send rate (bytes/second)
    pub send_rate_bps: AtomicU64,
    /// Current receive rate (bytes/second)
    pub receive_rate_bps: AtomicU64,
    /// Peak send rate
    pub peak_send_rate_bps: AtomicU64,
    /// Peak receive rate
    pub peak_receive_rate_bps: AtomicU64,
}

/// Error-related metrics
#[derive(Debug, Default)]
pub struct ErrorMetrics {
    /// Total errors
    pub total_errors: AtomicU64,
    /// Network errors
    pub network_errors: AtomicU64,
    /// Timeout errors
    pub timeout_errors: AtomicU64,
    /// Authentication errors
    pub auth_errors: AtomicU64,
    /// Encryption errors
    pub encryption_errors: AtomicU64,
    /// Serialization errors
    pub serialization_errors: AtomicU64,
    /// Protocol errors
    pub protocol_errors: AtomicU64,
}

/// Latency-related metrics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LatencyMetrics {
    /// Average round-trip time (milliseconds)
    pub avg_rtt_ms: f64,
    /// Minimum round-trip time
    pub min_rtt_ms: f64,
    /// Maximum round-trip time
    pub max_rtt_ms: f64,
    /// 95th percentile latency
    pub p95_latency_ms: f64,
    /// 99th percentile latency
    pub p99_latency_ms: f64,
    /// Recent latency samples (for percentile calculation)
    pub recent_samples: Vec<f64>,
    /// Maximum samples to keep
    pub max_samples: usize,
}

/// Per-peer metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerMetrics {
    /// Peer address
    pub address: String,
    /// Messages sent to this peer
    pub messages_sent: u64,
    /// Messages received from this peer
    pub messages_received: u64,
    /// Bytes sent to this peer
    pub bytes_sent: u64,
    /// Bytes received from this peer
    pub bytes_received: u64,
    /// Connection attempts to this peer
    pub connection_attempts: u64,
    /// Successful connections
    pub successful_connections: u64,
    /// Failed connections
    pub failed_connections: u64,
    /// Average latency to this peer
    pub avg_latency_ms: f64,
    /// Last seen timestamp
    pub last_seen: SystemTime,
    /// Reliability score (0.0 to 1.0)
    pub reliability_score: f64,
    /// Error count for this peer
    pub error_count: u64,
}

impl Default for PeerMetrics {
    fn default() -> Self {
        Self {
            address: String::new(),
            messages_sent: 0,
            messages_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            connection_attempts: 0,
            successful_connections: 0,
            failed_connections: 0,
            avg_latency_ms: 0.0,
            last_seen: SystemTime::UNIX_EPOCH,
            reliability_score: 1.0,
            error_count: 0,
        }
    }
}

/// Snapshot of network metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetricsSnapshot {
    /// Collection timestamp
    pub timestamp: SystemTime,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Connection metrics
    pub connections: ConnectionMetricsSnapshot,
    /// Message metrics
    pub messages: MessageMetricsSnapshot,
    /// Bandwidth metrics
    pub bandwidth: BandwidthMetricsSnapshot,
    /// Error metrics
    pub errors: ErrorMetricsSnapshot,
    /// Latency metrics
    pub latency: LatencyMetrics,
    /// Top peers by message count
    pub top_peers: Vec<PeerMetrics>,
}

/// Connection metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMetricsSnapshot {
    pub total_connections: u64,
    pub active_connections: usize,
    pub connection_failures: u64,
    pub connection_timeouts: u64,
    pub avg_connection_duration_ms: u64,
    pub peak_connections: usize,
}

/// Message metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetricsSnapshot {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub messages_dropped: u64,
    pub broadcast_messages: u64,
    pub unicast_messages: u64,
    pub send_failures: u64,
    pub receive_failures: u64,
}

/// Bandwidth metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthMetricsSnapshot {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub send_rate_bps: u64,
    pub receive_rate_bps: u64,
    pub peak_send_rate_bps: u64,
    pub peak_receive_rate_bps: u64,
}

/// Error metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMetricsSnapshot {
    pub total_errors: u64,
    pub network_errors: u64,
    pub timeout_errors: u64,
    pub auth_errors: u64,
    pub encryption_errors: u64,
    pub serialization_errors: u64,
    pub protocol_errors: u64,
}

impl NetworkMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            connection_metrics: Arc::new(ConnectionMetrics::default()),
            message_metrics: Arc::new(MessageMetrics::default()),
            bandwidth_metrics: Arc::new(BandwidthMetrics::default()),
            error_metrics: Arc::new(ErrorMetrics::default()),
            latency_metrics: Arc::new(RwLock::new(LatencyMetrics::new())),
            peer_metrics: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }

    /// Record a new connection
    pub fn record_connection_established(&self) {
        self.connection_metrics.total_connections.fetch_add(1, Ordering::Relaxed);
        let current = self.connection_metrics.active_connections.fetch_add(1, Ordering::Relaxed) + 1;

        // Update peak connections
        let peak = self.connection_metrics.peak_connections.load(Ordering::Relaxed);
        if current > peak {
            self.connection_metrics.peak_connections.store(current, Ordering::Relaxed);
        }
    }

    /// Record a connection closed
    pub fn record_connection_closed(&self, duration: Duration) {
        self.connection_metrics.active_connections.fetch_sub(1, Ordering::Relaxed);

        // Update average duration (simple approach)
        let duration_ms = duration.as_millis() as u64;
        self.connection_metrics.avg_connection_duration_ms.store(duration_ms, Ordering::Relaxed);
    }

    /// Record a connection failure
    pub fn record_connection_failure(&self) {
        self.connection_metrics.connection_failures.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a connection timeout
    pub fn record_connection_timeout(&self) {
        self.connection_metrics.connection_timeouts.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a message sent
    pub fn record_message_sent(&self, peer: &str, bytes: u64, is_broadcast: bool) {
        self.message_metrics.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.bandwidth_metrics.bytes_sent.fetch_add(bytes, Ordering::Relaxed);

        if is_broadcast {
            self.message_metrics.broadcast_messages.fetch_add(1, Ordering::Relaxed);
        } else {
            self.message_metrics.unicast_messages.fetch_add(1, Ordering::Relaxed);
        }

        // Update peer metrics
        tokio::spawn({
            let peer_metrics = self.peer_metrics.clone();
            let peer = peer.to_string();
            async move {
                let mut metrics = peer_metrics.write().await;
                let peer_metric = metrics.entry(peer.clone()).or_insert_with(|| PeerMetrics {
                    address: peer,
                    ..Default::default()
                });
                peer_metric.messages_sent += 1;
                peer_metric.bytes_sent += bytes;
                peer_metric.last_seen = SystemTime::now();
            }
        });
    }

    /// Record a message received
    pub fn record_message_received(&self, peer: &str, bytes: u64) {
        self.message_metrics.messages_received.fetch_add(1, Ordering::Relaxed);
        self.bandwidth_metrics.bytes_received.fetch_add(bytes, Ordering::Relaxed);

        // Update peer metrics
        tokio::spawn({
            let peer_metrics = self.peer_metrics.clone();
            let peer = peer.to_string();
            async move {
                let mut metrics = peer_metrics.write().await;
                let peer_metric = metrics.entry(peer.clone()).or_insert_with(|| PeerMetrics {
                    address: peer,
                    ..Default::default()
                });
                peer_metric.messages_received += 1;
                peer_metric.bytes_received += bytes;
                peer_metric.last_seen = SystemTime::now();
            }
        });
    }

    /// Record a message dropped
    pub fn record_message_dropped(&self) {
        self.message_metrics.messages_dropped.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a send failure
    pub fn record_send_failure(&self, peer: &str) {
        self.message_metrics.send_failures.fetch_add(1, Ordering::Relaxed);

        tokio::spawn({
            let peer_metrics = self.peer_metrics.clone();
            let peer = peer.to_string();
            async move {
                let mut metrics = peer_metrics.write().await;
                let peer_metric = metrics.entry(peer.clone()).or_insert_with(|| PeerMetrics {
                    address: peer,
                    ..Default::default()
                });
                peer_metric.error_count += 1;

                // Update reliability score
                let total_messages = peer_metric.messages_sent + peer_metric.messages_received;
                if total_messages > 0 {
                    peer_metric.reliability_score =
                        (total_messages - peer_metric.error_count) as f64 / total_messages as f64;
                }
            }
        });
    }

    /// Record a receive failure
    pub fn record_receive_failure(&self) {
        self.message_metrics.receive_failures.fetch_add(1, Ordering::Relaxed);
    }

    /// Record latency measurement
    pub async fn record_latency(&self, peer: &str, latency_ms: f64) {
        let mut latency_metrics = self.latency_metrics.write().await;
        latency_metrics.add_sample(latency_ms);

        // Update peer latency
        let mut peer_metrics = self.peer_metrics.write().await;
        if let Some(peer_metric) = peer_metrics.get_mut(peer) {
            // Simple moving average
            peer_metric.avg_latency_ms = (peer_metric.avg_latency_ms + latency_ms) / 2.0;
        }
    }

    /// Record an error
    pub fn record_error(&self, error_type: &str) {
        self.error_metrics.total_errors.fetch_add(1, Ordering::Relaxed);

        match error_type {
            "network" => { self.error_metrics.network_errors.fetch_add(1, Ordering::Relaxed); },
            "timeout" => { self.error_metrics.timeout_errors.fetch_add(1, Ordering::Relaxed); },
            "auth" => { self.error_metrics.auth_errors.fetch_add(1, Ordering::Relaxed); },
            "encryption" => { self.error_metrics.encryption_errors.fetch_add(1, Ordering::Relaxed); },
            "serialization" => { self.error_metrics.serialization_errors.fetch_add(1, Ordering::Relaxed); },
            "protocol" => { self.error_metrics.protocol_errors.fetch_add(1, Ordering::Relaxed); },
            _ => {},
        }
    }

    /// Update bandwidth rates
    pub fn update_bandwidth_rates(&self, send_rate_bps: u64, receive_rate_bps: u64) {
        self.bandwidth_metrics.send_rate_bps.store(send_rate_bps, Ordering::Relaxed);
        self.bandwidth_metrics.receive_rate_bps.store(receive_rate_bps, Ordering::Relaxed);

        // Update peak rates
        let peak_send = self.bandwidth_metrics.peak_send_rate_bps.load(Ordering::Relaxed);
        if send_rate_bps > peak_send {
            self.bandwidth_metrics.peak_send_rate_bps.store(send_rate_bps, Ordering::Relaxed);
        }

        let peak_receive = self.bandwidth_metrics.peak_receive_rate_bps.load(Ordering::Relaxed);
        if receive_rate_bps > peak_receive {
            self.bandwidth_metrics.peak_receive_rate_bps.store(receive_rate_bps, Ordering::Relaxed);
        }
    }

    /// Get a snapshot of current metrics
    pub async fn snapshot(&self) -> NetworkMetricsSnapshot {
        let latency_metrics = self.latency_metrics.read().await.clone();
        let peer_metrics = self.peer_metrics.read().await;

        // Get top 10 peers by message count
        let mut top_peers: Vec<PeerMetrics> = peer_metrics.values().cloned().collect();
        top_peers.sort_by(|a, b| {
            (b.messages_sent + b.messages_received).cmp(&(a.messages_sent + a.messages_received))
        });
        top_peers.truncate(10);

        NetworkMetricsSnapshot {
            timestamp: SystemTime::now(),
            uptime_seconds: self.start_time.elapsed().as_secs(),
            connections: ConnectionMetricsSnapshot {
                total_connections: self.connection_metrics.total_connections.load(Ordering::Relaxed),
                active_connections: self.connection_metrics.active_connections.load(Ordering::Relaxed),
                connection_failures: self.connection_metrics.connection_failures.load(Ordering::Relaxed),
                connection_timeouts: self.connection_metrics.connection_timeouts.load(Ordering::Relaxed),
                avg_connection_duration_ms: self.connection_metrics.avg_connection_duration_ms.load(Ordering::Relaxed),
                peak_connections: self.connection_metrics.peak_connections.load(Ordering::Relaxed),
            },
            messages: MessageMetricsSnapshot {
                messages_sent: self.message_metrics.messages_sent.load(Ordering::Relaxed),
                messages_received: self.message_metrics.messages_received.load(Ordering::Relaxed),
                messages_dropped: self.message_metrics.messages_dropped.load(Ordering::Relaxed),
                broadcast_messages: self.message_metrics.broadcast_messages.load(Ordering::Relaxed),
                unicast_messages: self.message_metrics.unicast_messages.load(Ordering::Relaxed),
                send_failures: self.message_metrics.send_failures.load(Ordering::Relaxed),
                receive_failures: self.message_metrics.receive_failures.load(Ordering::Relaxed),
            },
            bandwidth: BandwidthMetricsSnapshot {
                bytes_sent: self.bandwidth_metrics.bytes_sent.load(Ordering::Relaxed),
                bytes_received: self.bandwidth_metrics.bytes_received.load(Ordering::Relaxed),
                send_rate_bps: self.bandwidth_metrics.send_rate_bps.load(Ordering::Relaxed),
                receive_rate_bps: self.bandwidth_metrics.receive_rate_bps.load(Ordering::Relaxed),
                peak_send_rate_bps: self.bandwidth_metrics.peak_send_rate_bps.load(Ordering::Relaxed),
                peak_receive_rate_bps: self.bandwidth_metrics.peak_receive_rate_bps.load(Ordering::Relaxed),
            },
            errors: ErrorMetricsSnapshot {
                total_errors: self.error_metrics.total_errors.load(Ordering::Relaxed),
                network_errors: self.error_metrics.network_errors.load(Ordering::Relaxed),
                timeout_errors: self.error_metrics.timeout_errors.load(Ordering::Relaxed),
                auth_errors: self.error_metrics.auth_errors.load(Ordering::Relaxed),
                encryption_errors: self.error_metrics.encryption_errors.load(Ordering::Relaxed),
                serialization_errors: self.error_metrics.serialization_errors.load(Ordering::Relaxed),
                protocol_errors: self.error_metrics.protocol_errors.load(Ordering::Relaxed),
            },
            latency: latency_metrics,
            top_peers,
        }
    }

    /// Get metrics for a specific peer
    pub async fn get_peer_metrics(&self, peer: &str) -> Option<PeerMetrics> {
        self.peer_metrics.read().await.get(peer).cloned()
    }

    /// Get all peer metrics
    pub async fn get_all_peer_metrics(&self) -> HashMap<String, PeerMetrics> {
        self.peer_metrics.read().await.clone()
    }

    /// Reset all metrics
    pub async fn reset(&self) {
        // Reset atomic counters
        self.connection_metrics.total_connections.store(0, Ordering::Relaxed);
        self.connection_metrics.active_connections.store(0, Ordering::Relaxed);
        self.connection_metrics.connection_failures.store(0, Ordering::Relaxed);
        self.connection_metrics.connection_timeouts.store(0, Ordering::Relaxed);
        self.connection_metrics.avg_connection_duration_ms.store(0, Ordering::Relaxed);
        self.connection_metrics.peak_connections.store(0, Ordering::Relaxed);

        self.message_metrics.messages_sent.store(0, Ordering::Relaxed);
        self.message_metrics.messages_received.store(0, Ordering::Relaxed);
        self.message_metrics.messages_dropped.store(0, Ordering::Relaxed);
        self.message_metrics.broadcast_messages.store(0, Ordering::Relaxed);
        self.message_metrics.unicast_messages.store(0, Ordering::Relaxed);
        self.message_metrics.send_failures.store(0, Ordering::Relaxed);
        self.message_metrics.receive_failures.store(0, Ordering::Relaxed);

        self.bandwidth_metrics.bytes_sent.store(0, Ordering::Relaxed);
        self.bandwidth_metrics.bytes_received.store(0, Ordering::Relaxed);
        self.bandwidth_metrics.send_rate_bps.store(0, Ordering::Relaxed);
        self.bandwidth_metrics.receive_rate_bps.store(0, Ordering::Relaxed);
        self.bandwidth_metrics.peak_send_rate_bps.store(0, Ordering::Relaxed);
        self.bandwidth_metrics.peak_receive_rate_bps.store(0, Ordering::Relaxed);

        self.error_metrics.total_errors.store(0, Ordering::Relaxed);
        self.error_metrics.network_errors.store(0, Ordering::Relaxed);
        self.error_metrics.timeout_errors.store(0, Ordering::Relaxed);
        self.error_metrics.auth_errors.store(0, Ordering::Relaxed);
        self.error_metrics.encryption_errors.store(0, Ordering::Relaxed);
        self.error_metrics.serialization_errors.store(0, Ordering::Relaxed);
        self.error_metrics.protocol_errors.store(0, Ordering::Relaxed);

        // Reset latency metrics
        *self.latency_metrics.write().await = LatencyMetrics::new();

        // Clear peer metrics
        self.peer_metrics.write().await.clear();
    }
}

impl LatencyMetrics {
    /// Create new latency metrics
    pub fn new() -> Self {
        Self {
            avg_rtt_ms: 0.0,
            min_rtt_ms: f64::INFINITY,
            max_rtt_ms: 0.0,
            p95_latency_ms: 0.0,
            p99_latency_ms: 0.0,
            recent_samples: Vec::new(),
            max_samples: 1000,
        }
    }

    /// Add a latency sample
    pub fn add_sample(&mut self, latency_ms: f64) {
        self.recent_samples.push(latency_ms);

        // Keep only recent samples
        if self.recent_samples.len() > self.max_samples {
            self.recent_samples.remove(0);
        }

        // Update min/max
        if latency_ms < self.min_rtt_ms {
            self.min_rtt_ms = latency_ms;
        }
        if latency_ms > self.max_rtt_ms {
            self.max_rtt_ms = latency_ms;
        }

        // Calculate average
        self.avg_rtt_ms = self.recent_samples.iter().sum::<f64>() / self.recent_samples.len() as f64;

        // Calculate percentiles
        if self.recent_samples.len() >= 10 {
            let mut sorted_samples = self.recent_samples.clone();
            sorted_samples.sort_by(|a, b| a.partial_cmp(b).unwrap());

            let p95_index = ((sorted_samples.len() - 1) as f64 * 0.95).round() as usize;
            let p99_index = ((sorted_samples.len() - 1) as f64 * 0.99).round() as usize;

            self.p95_latency_ms = sorted_samples[p95_index];
            self.p99_latency_ms = sorted_samples[p99_index];
        }
    }
}

impl Default for NetworkMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_metrics() {
        let metrics = NetworkMetrics::new();

        // Test connection establishment
        metrics.record_connection_established();
        metrics.record_connection_established();

        let snapshot = metrics.snapshot().await;
        assert_eq!(snapshot.connections.total_connections, 2);
        assert_eq!(snapshot.connections.active_connections, 2);

        // Test connection closure
        metrics.record_connection_closed(Duration::from_secs(5));
        let snapshot = metrics.snapshot().await;
        assert_eq!(snapshot.connections.active_connections, 1);
    }

    #[tokio::test]
    async fn test_message_metrics() {
        let metrics = NetworkMetrics::new();

        // Test message sending/receiving
        metrics.record_message_sent("peer1", 100, false);
        metrics.record_message_sent("peer2", 200, true);
        metrics.record_message_received("peer1", 150);

        let snapshot = metrics.snapshot().await;
        assert_eq!(snapshot.messages.messages_sent, 2);
        assert_eq!(snapshot.messages.messages_received, 1);
        assert_eq!(snapshot.messages.unicast_messages, 1);
        assert_eq!(snapshot.messages.broadcast_messages, 1);
        assert_eq!(snapshot.bandwidth.bytes_sent, 300);
        assert_eq!(snapshot.bandwidth.bytes_received, 150);
    }

    #[tokio::test]
    async fn test_error_metrics() {
        let metrics = NetworkMetrics::new();

        metrics.record_error("network");
        metrics.record_error("timeout");
        metrics.record_error("auth");

        let snapshot = metrics.snapshot().await;
        assert_eq!(snapshot.errors.total_errors, 3);
        assert_eq!(snapshot.errors.network_errors, 1);
        assert_eq!(snapshot.errors.timeout_errors, 1);
        assert_eq!(snapshot.errors.auth_errors, 1);
    }

    #[tokio::test]
    async fn test_latency_metrics() {
        let metrics = NetworkMetrics::new();

        // Record some latency samples
        metrics.record_latency("peer1", 10.0).await;
        metrics.record_latency("peer1", 20.0).await;
        metrics.record_latency("peer1", 30.0).await;

        let snapshot = metrics.snapshot().await;
        assert_eq!(snapshot.latency.avg_rtt_ms, 20.0);
        assert_eq!(snapshot.latency.min_rtt_ms, 10.0);
        assert_eq!(snapshot.latency.max_rtt_ms, 30.0);
    }

    #[tokio::test]
    async fn test_peer_metrics() {
        let metrics = NetworkMetrics::new();

        // Send messages to different peers
        metrics.record_message_sent("peer1", 100, false);
        metrics.record_message_sent("peer1", 200, false);
        metrics.record_message_received("peer1", 150);
        metrics.record_send_failure("peer1");

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await; // Wait for async updates

        let peer_metric = metrics.get_peer_metrics("peer1").await.unwrap();
        assert_eq!(peer_metric.messages_sent, 2);
        assert_eq!(peer_metric.messages_received, 1);
        assert_eq!(peer_metric.bytes_sent, 300);
        assert_eq!(peer_metric.bytes_received, 150);
        assert_eq!(peer_metric.error_count, 1);
    }

    #[tokio::test]
    async fn test_bandwidth_rates() {
        let metrics = NetworkMetrics::new();

        metrics.update_bandwidth_rates(1000, 2000);
        metrics.update_bandwidth_rates(1500, 1800); // Higher send rate

        let snapshot = metrics.snapshot().await;
        assert_eq!(snapshot.bandwidth.send_rate_bps, 1500);
        assert_eq!(snapshot.bandwidth.receive_rate_bps, 1800);
        assert_eq!(snapshot.bandwidth.peak_send_rate_bps, 1500);
        assert_eq!(snapshot.bandwidth.peak_receive_rate_bps, 2000);
    }

    #[tokio::test]
    async fn test_metrics_reset() {
        let metrics = NetworkMetrics::new();

        // Generate some metrics
        metrics.record_connection_established();
        metrics.record_message_sent("peer1", 100, false);
        metrics.record_error("network");

        let snapshot_before = metrics.snapshot().await;
        assert!(snapshot_before.connections.total_connections > 0);

        // Reset metrics
        metrics.reset().await;

        let snapshot_after = metrics.snapshot().await;
        assert_eq!(snapshot_after.connections.total_connections, 0);
        assert_eq!(snapshot_after.messages.messages_sent, 0);
        assert_eq!(snapshot_after.errors.total_errors, 0);
    }

    #[test]
    fn test_latency_percentiles() {
        let mut latency_metrics = LatencyMetrics::new();

        // Add samples to calculate percentiles
        for i in 1..=100 {
            latency_metrics.add_sample(i as f64);
        }

        assert!((latency_metrics.avg_rtt_ms - 50.5).abs() < 0.1);
        assert_eq!(latency_metrics.min_rtt_ms, 1.0);
        assert_eq!(latency_metrics.max_rtt_ms, 100.0);
        assert!((latency_metrics.p95_latency_ms - 95.0).abs() < 1.0);
        assert!((latency_metrics.p99_latency_ms - 99.0).abs() < 1.0);
    }
}