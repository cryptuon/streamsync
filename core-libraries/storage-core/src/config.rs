use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage backend type
    pub backend_type: StorageBackendType,

    /// Database file path for DuckDB
    pub database_path: PathBuf,

    /// Enable Write-Ahead Logging
    pub enable_wal: bool,

    /// Memory limit in MB for DuckDB
    pub memory_limit_mb: usize,

    /// Number of worker threads
    pub worker_threads: usize,

    /// Batch size for bulk insertions
    pub batch_insert_size: usize,

    /// Enable compression
    pub enable_compression: bool,

    /// Compression algorithm
    pub compression_algorithm: CompressionType,

    /// Cache configuration
    pub cache: CacheConfig,

    /// Retention policy
    pub retention: RetentionConfig,

    /// Backup configuration
    pub backup: BackupConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageBackendType {
    DuckDB,
    Memory,
    Hybrid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionType {
    None,
    Gzip,
    Zstd,
    Lz4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable in-memory caching
    pub enabled: bool,

    /// Cache size in MB
    pub size_mb: usize,

    /// Cache TTL in seconds
    pub ttl_seconds: u64,

    /// Maximum number of cached items
    pub max_items: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfig {
    /// Data retention period in days
    pub retention_days: u32,

    /// Enable automatic cleanup
    pub auto_cleanup: bool,

    /// Cleanup interval in hours
    pub cleanup_interval_hours: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Enable automatic backups
    pub enabled: bool,

    /// Backup directory
    pub backup_directory: PathBuf,

    /// Backup interval in hours
    pub backup_interval_hours: u32,

    /// Number of backups to retain
    pub max_backups: u32,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend_type: StorageBackendType::DuckDB,
            database_path: PathBuf::from("./data/streamsync.db"),
            enable_wal: true,
            memory_limit_mb: 2048,
            worker_threads: num_cpus::get(),
            batch_insert_size: 1000,
            enable_compression: true,
            compression_algorithm: CompressionType::Zstd,
            cache: CacheConfig::default(),
            retention: RetentionConfig::default(),
            backup: BackupConfig::default(),
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            size_mb: 256,
            ttl_seconds: 3600,
            max_items: 100000,
        }
    }
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            retention_days: 90,
            auto_cleanup: true,
            cleanup_interval_hours: 24,
        }
    }
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            backup_directory: PathBuf::from("./backups"),
            backup_interval_hours: 24,
            max_backups: 7,
        }
    }
}

impl StorageConfig {
    pub fn new(database_path: PathBuf) -> Self {
        Self {
            database_path,
            ..Default::default()
        }
    }

    pub fn with_backend(mut self, backend_type: StorageBackendType) -> Self {
        self.backend_type = backend_type;
        self
    }

    pub fn with_memory_limit(mut self, limit_mb: usize) -> Self {
        self.memory_limit_mb = limit_mb;
        self
    }

    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_insert_size = batch_size;
        self
    }

    pub fn with_compression(mut self, algorithm: CompressionType) -> Self {
        self.enable_compression = true;
        self.compression_algorithm = algorithm;
        self
    }

    pub fn get_cache_ttl(&self) -> Duration {
        Duration::from_secs(self.cache.ttl_seconds)
    }

    pub fn get_cleanup_interval(&self) -> Duration {
        Duration::from_secs(self.retention.cleanup_interval_hours as u64 * 3600)
    }

    pub fn get_backup_interval(&self) -> Duration {
        Duration::from_secs(self.backup.backup_interval_hours as u64 * 3600)
    }
}