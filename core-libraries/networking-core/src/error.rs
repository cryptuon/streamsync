//! Error types for networking operations

use std::net::SocketAddr;
use thiserror::Error;

/// Result type for networking operations
pub type Result<T> = std::result::Result<T, NetworkError>;

/// Comprehensive error types for networking operations
#[derive(Error, Debug, Clone, PartialEq)]
pub enum NetworkError {
    /// Connection-related errors
    #[error("Failed to connect to {address}: {reason}")]
    ConnectionFailed { address: SocketAddr, reason: String },

    #[error("Connection to {address} lost: {reason}")]
    ConnectionLost { address: SocketAddr, reason: String },

    #[error("Connection limit exceeded: {current}/{max}")]
    ConnectionLimitExceeded { current: usize, max: usize },

    #[error("Invalid peer address: {address}")]
    InvalidPeerAddress { address: String },

    /// Transport-related errors
    #[error("Transport not started")]
    TransportNotStarted,

    #[error("Transport already running")]
    TransportAlreadyRunning,

    #[error("Service already running")]
    AlreadyRunning,

    #[error("Unsupported transport type: {transport_type}")]
    UnsupportedTransport { transport_type: String },

    /// Message-related errors
    #[error("Message too large: {size} bytes (max: {max_size})")]
    MessageTooLarge { size: usize, max_size: usize },

    #[error("Failed to serialize message: {reason}")]
    SerializationError { reason: String },

    #[error("Failed to deserialize message: {reason}")]
    DeserializationError { reason: String },

    #[error("Message timeout after {timeout_ms}ms")]
    MessageTimeout { timeout_ms: u64 },

    /// Security-related errors
    #[error("Authentication failed for peer {peer}: {reason}")]
    AuthenticationFailed { peer: SocketAddr, reason: String },

    #[error("Encryption error: {reason}")]
    EncryptionError { reason: String },

    #[error("Certificate validation failed: {reason}")]
    CertificateError { reason: String },

    /// Configuration errors
    #[error("Invalid configuration: {field} = {value} ({reason})")]
    InvalidConfiguration { field: String, value: String, reason: String },

    #[error("Missing required configuration: {field}")]
    MissingConfiguration { field: String },

    /// Resource errors
    #[error("Resource exhausted: {resource} ({current}/{max})")]
    ResourceExhausted { resource: String, current: usize, max: usize },

    #[error("Insufficient bandwidth: {required_bps} required, {available_bps} available")]
    InsufficientBandwidth { required_bps: u64, available_bps: u64 },

    /// I/O errors
    #[error("I/O error: {reason}")]
    IoError { reason: String },

    #[error("Network interface error: {interface}, {reason}")]
    InterfaceError { interface: String, reason: String },

    /// Protocol errors
    #[error("Protocol version mismatch: expected {expected}, got {actual}")]
    ProtocolVersionMismatch { expected: u32, actual: u32 },

    #[error("Malformed message: {reason}")]
    MalformedMessage { reason: String },

    #[error("Unexpected message type: {message_type}")]
    UnexpectedMessageType { message_type: String },
}

impl NetworkError {
    /// Check if this error is recoverable (retryable)
    pub fn is_recoverable(&self) -> bool {
        match self {
            // Connection errors that can be retried
            NetworkError::ConnectionFailed { .. } => true,
            NetworkError::ConnectionLost { .. } => true,
            NetworkError::MessageTimeout { .. } => true,
            NetworkError::InsufficientBandwidth { .. } => true,
            NetworkError::IoError { .. } => true,
            NetworkError::InterfaceError { .. } => true,

            // Non-recoverable errors
            NetworkError::ConnectionLimitExceeded { .. } => false,
            NetworkError::InvalidPeerAddress { .. } => false,
            NetworkError::TransportNotStarted => false,
            NetworkError::TransportAlreadyRunning => false,
            NetworkError::UnsupportedTransport { .. } => false,
            NetworkError::MessageTooLarge { .. } => false,
            NetworkError::SerializationError { .. } => false,
            NetworkError::DeserializationError { .. } => false,
            NetworkError::AuthenticationFailed { .. } => false,
            NetworkError::EncryptionError { .. } => false,
            NetworkError::CertificateError { .. } => false,
            NetworkError::InvalidConfiguration { .. } => false,
            NetworkError::MissingConfiguration { .. } => false,
            NetworkError::ResourceExhausted { .. } => false,
            NetworkError::ProtocolVersionMismatch { .. } => false,
            NetworkError::MalformedMessage { .. } => false,
            NetworkError::UnexpectedMessageType { .. } => false,
            NetworkError::AlreadyRunning => false,
        }
    }

    /// Get error category for metrics and logging
    pub fn category(&self) -> &'static str {
        match self {
            NetworkError::ConnectionFailed { .. } |
            NetworkError::ConnectionLost { .. } |
            NetworkError::ConnectionLimitExceeded { .. } |
            NetworkError::InvalidPeerAddress { .. } => "connection",

            NetworkError::TransportNotStarted |
            NetworkError::TransportAlreadyRunning |
            NetworkError::UnsupportedTransport { .. } => "transport",

            NetworkError::MessageTooLarge { .. } |
            NetworkError::SerializationError { .. } |
            NetworkError::DeserializationError { .. } |
            NetworkError::MessageTimeout { .. } => "message",

            NetworkError::AuthenticationFailed { .. } |
            NetworkError::EncryptionError { .. } |
            NetworkError::CertificateError { .. } => "security",

            NetworkError::InvalidConfiguration { .. } |
            NetworkError::MissingConfiguration { .. } => "configuration",

            NetworkError::ResourceExhausted { .. } |
            NetworkError::InsufficientBandwidth { .. } => "resource",

            NetworkError::IoError { .. } |
            NetworkError::InterfaceError { .. } => "io",

            NetworkError::ProtocolVersionMismatch { .. } |
            NetworkError::MalformedMessage { .. } |
            NetworkError::UnexpectedMessageType { .. } => "protocol",
            NetworkError::AlreadyRunning => "state",
        }
    }

    /// Get severity level for logging
    pub fn severity(&self) -> &'static str {
        match self {
            // Critical errors that require immediate attention
            NetworkError::ResourceExhausted { .. } |
            NetworkError::ConnectionLimitExceeded { .. } |
            NetworkError::CertificateError { .. } => "critical",

            // High severity errors
            NetworkError::AuthenticationFailed { .. } |
            NetworkError::EncryptionError { .. } |
            NetworkError::ProtocolVersionMismatch { .. } |
            NetworkError::InvalidConfiguration { .. } |
            NetworkError::MissingConfiguration { .. } => "error",

            // Medium severity - operational issues
            NetworkError::ConnectionFailed { .. } |
            NetworkError::ConnectionLost { .. } |
            NetworkError::MessageTimeout { .. } |
            NetworkError::InsufficientBandwidth { .. } |
            NetworkError::IoError { .. } |
            NetworkError::InterfaceError { .. } => "warning",

            // Low severity - client/usage errors
            NetworkError::TransportNotStarted |
            NetworkError::TransportAlreadyRunning |
            NetworkError::UnsupportedTransport { .. } |
            NetworkError::InvalidPeerAddress { .. } |
            NetworkError::MessageTooLarge { .. } |
            NetworkError::SerializationError { .. } |
            NetworkError::DeserializationError { .. } |
            NetworkError::MalformedMessage { .. } |
            NetworkError::UnexpectedMessageType { .. } => "info",
            NetworkError::AlreadyRunning => "warn",
        }
    }
}

// Implement conversions from common error types
impl From<std::io::Error> for NetworkError {
    fn from(err: std::io::Error) -> Self {
        NetworkError::IoError {
            reason: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for NetworkError {
    fn from(err: serde_json::Error) -> Self {
        NetworkError::SerializationError {
            reason: err.to_string(),
        }
    }
}

impl From<bincode::Error> for NetworkError {
    fn from(err: bincode::Error) -> Self {
        NetworkError::SerializationError {
            reason: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_error_categorization() {
        let connection_error = NetworkError::ConnectionFailed {
            address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
            reason: "timeout".to_string(),
        };
        assert_eq!(connection_error.category(), "connection");
        assert!(connection_error.is_recoverable());
        assert_eq!(connection_error.severity(), "warning");

        let auth_error = NetworkError::AuthenticationFailed {
            peer: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
            reason: "invalid certificate".to_string(),
        };
        assert_eq!(auth_error.category(), "security");
        assert!(!auth_error.is_recoverable());
        assert_eq!(auth_error.severity(), "error");
    }

    #[test]
    fn test_error_from_io() {
        let io_error = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "connection refused");
        let network_error: NetworkError = io_error.into();
        assert_eq!(network_error.category(), "io");
        assert!(network_error.is_recoverable());
    }

    #[test]
    fn test_recoverable_errors() {
        let recoverable_errors = vec![
            NetworkError::ConnectionFailed {
                address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
                reason: "timeout".to_string(),
            },
            NetworkError::MessageTimeout { timeout_ms: 5000 },
            NetworkError::IoError { reason: "temporary failure".to_string() },
        ];

        for error in recoverable_errors {
            assert!(error.is_recoverable(), "Expected error to be recoverable: {:?}", error);
        }

        let non_recoverable_errors = vec![
            NetworkError::InvalidConfiguration {
                field: "max_connections".to_string(),
                value: "invalid".to_string(),
                reason: "not a number".to_string(),
            },
            NetworkError::AuthenticationFailed {
                peer: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
                reason: "invalid credentials".to_string(),
            },
        ];

        for error in non_recoverable_errors {
            assert!(!error.is_recoverable(), "Expected error to be non-recoverable: {:?}", error);
        }
    }
}