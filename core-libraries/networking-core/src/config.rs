//! Network configuration management

use crate::error::{NetworkError, Result};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;

/// Network configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Address to bind the server to
    pub bind_address: SocketAddr,

    /// Maximum number of concurrent connections
    pub max_connections: usize,

    /// Connection timeout duration
    pub connection_timeout: Duration,

    /// Read timeout for socket operations
    pub read_timeout: Duration,

    /// Write timeout for socket operations
    pub write_timeout: Duration,

    /// Maximum message size in bytes
    pub max_message_size: usize,

    /// Buffer size for reading/writing
    pub buffer_size: usize,

    /// Keep-alive interval
    pub keep_alive_interval: Option<Duration>,

    /// Enable TCP_NODELAY
    pub tcp_nodelay: bool,

    /// Enable SO_REUSEADDR
    pub reuse_address: bool,

    /// TLS configuration
    pub tls_config: Option<TlsConfig>,

    /// Authentication configuration
    pub auth_config: Option<AuthConfig>,

    /// Compression settings
    pub compression: CompressionConfig,

    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,

    /// Retry configuration
    pub retry_config: RetryConfig,

    /// Health check configuration
    pub health_check: HealthCheckConfig,
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Path to certificate file
    pub cert_path: String,

    /// Path to private key file
    pub key_path: String,

    /// Path to CA certificate file
    pub ca_path: Option<String>,

    /// Require client certificates
    pub require_client_cert: bool,

    /// Allowed TLS versions
    pub allowed_versions: Vec<String>,

    /// Cipher suites
    pub cipher_suites: Vec<String>,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Authentication method
    pub method: AuthMethod,

    /// Token or key for authentication
    pub credentials: String,

    /// Authentication timeout
    pub timeout: Duration,
}

/// Authentication methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    /// No authentication
    None,
    /// Shared secret authentication
    SharedSecret,
    /// Token-based authentication
    Token,
    /// Mutual TLS authentication
    MutualTls,
}

/// Compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Enable compression
    pub enabled: bool,

    /// Compression algorithm
    pub algorithm: CompressionAlgorithm,

    /// Compression level (1-9)
    pub level: u8,

    /// Minimum message size to compress
    pub min_size: usize,
}

/// Compression algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    None,
    Gzip,
    Deflate,
    Lz4,
    Snappy,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Enable rate limiting
    pub enabled: bool,

    /// Messages per second limit
    pub messages_per_second: u64,

    /// Bytes per second limit
    pub bytes_per_second: u64,

    /// Burst size for token bucket
    pub burst_size: usize,
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: usize,

    /// Initial retry delay
    pub initial_delay: Duration,

    /// Maximum retry delay
    pub max_delay: Duration,

    /// Backoff multiplier
    pub backoff_multiplier: f64,

    /// Jitter to add to delays
    pub jitter: bool,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Enable health checks
    pub enabled: bool,

    /// Health check interval
    pub interval: Duration,

    /// Health check timeout
    pub timeout: Duration,

    /// Number of failed checks before marking unhealthy
    pub failure_threshold: usize,

    /// Number of successful checks before marking healthy
    pub success_threshold: usize,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:0".parse().unwrap(),
            max_connections: 1000,
            connection_timeout: Duration::from_secs(30),
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(30),
            max_message_size: 64 * 1024 * 1024, // 64MB
            buffer_size: 64 * 1024, // 64KB
            keep_alive_interval: Some(Duration::from_secs(60)),
            tcp_nodelay: true,
            reuse_address: true,
            tls_config: None,
            auth_config: None,
            compression: CompressionConfig::default(),
            rate_limit: RateLimitConfig::default(),
            retry_config: RetryConfig::default(),
            health_check: HealthCheckConfig::default(),
        }
    }
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            algorithm: CompressionAlgorithm::None,
            level: 6,
            min_size: 1024,
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            messages_per_second: 1000,
            bytes_per_second: 10 * 1024 * 1024, // 10MB/s
            burst_size: 100,
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(5),
            failure_threshold: 3,
            success_threshold: 2,
        }
    }
}

impl NetworkConfig {
    /// Create a new network configuration with the given bind address
    pub fn new(bind_address: SocketAddr) -> Self {
        Self {
            bind_address,
            ..Default::default()
        }
    }

    /// Set the bind address
    pub fn with_bind_address(mut self, address: SocketAddr) -> Self {
        self.bind_address = address;
        self
    }

    /// Set the maximum number of connections
    pub fn with_max_connections(mut self, max: usize) -> Self {
        self.max_connections = max;
        self
    }

    /// Set connection timeout
    pub fn with_connection_timeout(mut self, timeout: Duration) -> Self {
        self.connection_timeout = timeout;
        self
    }

    /// Set TLS configuration
    pub fn with_tls(mut self, tls_config: TlsConfig) -> Self {
        self.tls_config = Some(tls_config);
        self
    }

    /// Set authentication configuration
    pub fn with_auth(mut self, auth_config: AuthConfig) -> Self {
        self.auth_config = Some(auth_config);
        self
    }

    /// Enable compression
    pub fn with_compression(mut self, algorithm: CompressionAlgorithm, level: u8) -> Self {
        self.compression.enabled = true;
        self.compression.algorithm = algorithm;
        self.compression.level = level;
        self
    }

    /// Enable rate limiting
    pub fn with_rate_limit(mut self, messages_per_sec: u64, bytes_per_sec: u64) -> Self {
        self.rate_limit.enabled = true;
        self.rate_limit.messages_per_second = messages_per_sec;
        self.rate_limit.bytes_per_second = bytes_per_sec;
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.max_connections == 0 {
            return Err(NetworkError::InvalidConfiguration {
                field: "max_connections".to_string(),
                value: "0".to_string(),
                reason: "must be greater than 0".to_string(),
            });
        }

        if self.max_message_size == 0 {
            return Err(NetworkError::InvalidConfiguration {
                field: "max_message_size".to_string(),
                value: "0".to_string(),
                reason: "must be greater than 0".to_string(),
            });
        }

        if self.buffer_size == 0 {
            return Err(NetworkError::InvalidConfiguration {
                field: "buffer_size".to_string(),
                value: "0".to_string(),
                reason: "must be greater than 0".to_string(),
            });
        }

        if self.compression.enabled {
            if self.compression.level == 0 || self.compression.level > 9 {
                return Err(NetworkError::InvalidConfiguration {
                    field: "compression.level".to_string(),
                    value: self.compression.level.to_string(),
                    reason: "must be between 1 and 9".to_string(),
                });
            }
        }

        if self.retry_config.max_attempts == 0 {
            return Err(NetworkError::InvalidConfiguration {
                field: "retry_config.max_attempts".to_string(),
                value: "0".to_string(),
                reason: "must be greater than 0".to_string(),
            });
        }

        if self.retry_config.backoff_multiplier <= 0.0 {
            return Err(NetworkError::InvalidConfiguration {
                field: "retry_config.backoff_multiplier".to_string(),
                value: self.retry_config.backoff_multiplier.to_string(),
                reason: "must be greater than 0".to_string(),
            });
        }

        Ok(())
    }

    /// Create a test configuration with reasonable defaults
    pub fn test_config() -> Self {
        Self {
            bind_address: "127.0.0.1:0".parse().unwrap(),
            max_connections: 10,
            connection_timeout: Duration::from_millis(1000),
            read_timeout: Duration::from_millis(1000),
            write_timeout: Duration::from_millis(1000),
            max_message_size: 1024 * 1024, // 1MB
            buffer_size: 8192, // 8KB
            keep_alive_interval: Some(Duration::from_secs(10)),
            tcp_nodelay: true,
            reuse_address: true,
            tls_config: None,
            auth_config: None,
            compression: CompressionConfig::default(),
            rate_limit: RateLimitConfig::default(),
            retry_config: RetryConfig {
                max_attempts: 2,
                initial_delay: Duration::from_millis(10),
                max_delay: Duration::from_millis(100),
                backoff_multiplier: 1.5,
                jitter: false,
            },
            health_check: HealthCheckConfig {
                enabled: false,
                interval: Duration::from_secs(5),
                timeout: Duration::from_secs(1),
                failure_threshold: 2,
                success_threshold: 1,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NetworkConfig::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.max_connections, 1000);
        assert!(!config.compression.enabled);
        assert!(!config.rate_limit.enabled);
    }

    #[test]
    fn test_config_builder() {
        let config = NetworkConfig::new("127.0.0.1:8080".parse().unwrap())
            .with_max_connections(500)
            .with_compression(CompressionAlgorithm::Gzip, 6)
            .with_rate_limit(100, 1024 * 1024);

        assert_eq!(config.bind_address.port(), 8080);
        assert_eq!(config.max_connections, 500);
        assert!(config.compression.enabled);
        assert!(config.rate_limit.enabled);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_configurations() {
        let mut config = NetworkConfig::default();

        // Test zero max_connections
        config.max_connections = 0;
        assert!(config.validate().is_err());

        // Test invalid compression level
        config.max_connections = 100;
        config.compression.enabled = true;
        config.compression.level = 0;
        assert!(config.validate().is_err());

        config.compression.level = 10;
        assert!(config.validate().is_err());

        // Test zero retry attempts
        config.compression.level = 6;
        config.retry_config.max_attempts = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_test_config() {
        let config = NetworkConfig::test_config();
        assert!(config.validate().is_ok());
        assert_eq!(config.max_connections, 10);
        assert!(!config.health_check.enabled);
    }
}