use anyhow::{anyhow, Result};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::time::interval;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::backend::{DuckDBBackend, StorageBackend};
use crate::cache::{CacheStats, StorageCache};
use crate::compression::{BatchCompressor, CompressionEngine, CompressionStats};
use crate::config::StorageConfig;
use crate::query::{QueryEngine, QueryResult, QueryStats};
use crate::schema::SchemaManager;

// Events for storage operations
#[derive(Debug, Clone)]
pub enum StorageEvent {
    RecordsInserted { table: String, count: usize },
    QueryExecuted { query_id: Uuid, execution_time_ms: u128 },
    CacheHit { key: String },
    CacheMiss { key: String },
    CompressionCompleted { stats: CompressionStats },
    MaintenanceStarted,
    MaintenanceCompleted { duration_ms: u128 },
}

#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_records: u64,
    pub total_queries: u64,
    pub cache_stats: Option<CacheStats>,
    pub query_stats: QueryStats,
    pub compression_ratio: f64,
    pub storage_size_bytes: u64,
    pub uptime_seconds: u64,
}

pub struct StorageManager {
    config: StorageConfig,
    backend: Arc<RwLock<dyn StorageBackend>>,
    schema_manager: SchemaManager,
    query_engine: QueryEngine,
    cache: Option<StorageCache>,
    compressor: Option<BatchCompressor>,
    event_sender: broadcast::Sender<StorageEvent>,
    running: Arc<RwLock<bool>>,
    start_time: std::time::Instant,
}

impl StorageManager {
    pub async fn new(config: StorageConfig) -> Result<Self> {
        info!("Initializing storage manager");

        // Initialize backend
        let mut backend_impl = DuckDBBackend::new();
        backend_impl.initialize(&config).await?;
        let backend: Arc<RwLock<dyn StorageBackend>> = Arc::new(RwLock::new(backend_impl));

        // Initialize schema manager
        let mut schema_manager = SchemaManager::new();
        schema_manager.initialize_solana_schema()?;

        // Initialize query engine
        let query_engine = QueryEngine::new(backend.clone());

        // Initialize cache if enabled
        let cache = if config.cache.enabled {
            let cache = StorageCache::new(config.cache.clone());
            Some(cache.clone())
        } else {
            None
        };

        // Initialize compressor if enabled
        let compressor = if config.enable_compression {
            Some(BatchCompressor::new(
                config.compression_algorithm.clone(),
                config.batch_insert_size,
            ))
        } else {
            None
        };

        // Set up event broadcasting
        let (event_sender, _) = broadcast::channel(1000);

        Ok(Self {
            config,
            backend,
            schema_manager,
            query_engine: if let Some(cache) = cache.clone() {
                query_engine.with_cache(cache)
            } else {
                query_engine
            },
            cache,
            compressor,
            event_sender,
            running: Arc::new(RwLock::new(false)),
            start_time: std::time::Instant::now(),
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(anyhow!("Storage manager is already running"));
        }

        info!("Starting storage manager");

        // Create database tables
        {
            let mut backend = self.backend.write().await;
            backend.create_tables(&self.schema_manager).await?;
        }

        // Start cache cleanup task if cache is enabled
        if let Some(cache) = &self.cache {
            cache.start_cleanup_task().await?;
        }

        // Start maintenance tasks
        // Note: Background maintenance tasks disabled for basic integration

        *running = true;
        info!("Storage manager started successfully");

        // Send startup event
        let _ = self.event_sender.send(StorageEvent::MaintenanceStarted);

        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Ok(());
        }

        info!("Stopping storage manager");

        // Close backend connection
        {
            let mut backend = self.backend.write().await;
            backend.close().await?;
        }

        *running = false;
        info!("Storage manager stopped");
        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    // Data insertion methods
    pub async fn insert_blocks(&self, blocks: Vec<serde_json::Value>) -> Result<usize> {
        if blocks.is_empty() {
            return Ok(0);
        }

        debug!("Inserting {} blocks", blocks.len());

        let processed_blocks = if let Some(compressor) = &self.compressor {
            // For demonstration, we don't actually compress individual records
            // but we could compress batches before storage
            blocks
        } else {
            blocks
        };

        let mut backend = self.backend.write().await;
        let count = backend.insert_batch("blocks", processed_blocks).await?;

        // Send event
        let _ = self.event_sender.send(StorageEvent::RecordsInserted {
            table: "blocks".to_string(),
            count,
        });

        debug!("Inserted {} blocks successfully", count);
        Ok(count)
    }

    pub async fn insert_transactions(&self, transactions: Vec<serde_json::Value>) -> Result<usize> {
        if transactions.is_empty() {
            return Ok(0);
        }

        debug!("Inserting {} transactions", transactions.len());

        let mut backend = self.backend.write().await;
        let count = backend.insert_batch("transactions", transactions).await?;

        // Send event
        let _ = self.event_sender.send(StorageEvent::RecordsInserted {
            table: "transactions".to_string(),
            count,
        });

        debug!("Inserted {} transactions successfully", count);
        Ok(count)
    }

    // Query methods
    pub async fn execute_query(&self, sql: &str) -> Result<QueryResult> {
        let result = self.query_engine.execute(sql).await?;

        // Send event
        let _ = self.event_sender.send(StorageEvent::QueryExecuted {
            query_id: result.query_id,
            execution_time_ms: result.execution_time_ms,
        });

        Ok(result)
    }

    pub async fn count_records(&self, table: &str) -> Result<u64> {
        self.query_engine.count_records(table).await
    }

    // Convenience query methods for Solana data
    pub async fn get_transactions_by_slot(&self, slot: u64) -> Result<QueryResult> {
        let sql = format!("SELECT * FROM transactions WHERE slot = {} ORDER BY block_time DESC", slot);
        self.execute_query(&sql).await
    }

    pub async fn get_transaction_by_signature(&self, signature: &str) -> Result<QueryResult> {
        let sql = format!("SELECT * FROM transactions WHERE signature = '{}' LIMIT 1", signature);
        self.execute_query(&sql).await
    }

    pub async fn get_recent_blocks(&self, limit: u32) -> Result<QueryResult> {
        let sql = format!("SELECT * FROM blocks ORDER BY slot DESC LIMIT {}", limit);
        self.execute_query(&sql).await
    }

    pub async fn get_transactions_by_program(&self, program_id: &str, limit: u32) -> Result<QueryResult> {
        let sql = format!(
            "SELECT t.* FROM transactions t JOIN instructions i ON t.id = i.transaction_id WHERE i.program_id = '{}' ORDER BY t.block_time DESC LIMIT {}",
            program_id, limit
        );
        self.execute_query(&sql).await
    }

    pub async fn get_account_activity(&self, pubkey: &str, limit: u32) -> Result<QueryResult> {
        let sql = format!(
            "SELECT t.* FROM transactions t JOIN accounts a ON t.id = a.transaction_id WHERE a.pubkey = '{}' ORDER BY t.block_time DESC LIMIT {}",
            pubkey, limit
        );
        self.execute_query(&sql).await
    }

    // Statistics and monitoring
    pub async fn get_storage_stats(&self) -> Result<StorageStats> {
        let query_stats = self.query_engine.get_query_stats().await;

        let cache_stats = if let Some(cache) = &self.cache {
            Some(cache.get_stats().await)
        } else {
            None
        };

        // Get total record counts
        let total_blocks = self.count_records("blocks").await.unwrap_or(0);
        let total_transactions = self.count_records("transactions").await.unwrap_or(0);
        let total_records = total_blocks + total_transactions;

        Ok(StorageStats {
            total_records,
            total_queries: query_stats.total_queries,
            cache_stats,
            query_stats,
            compression_ratio: 0.8, // Placeholder - would be calculated from actual compression stats
            storage_size_bytes: total_records * 1024, // Rough estimate
            uptime_seconds: self.start_time.elapsed().as_secs(),
        })
    }

    pub fn subscribe_to_events(&self) -> broadcast::Receiver<StorageEvent> {
        self.event_sender.subscribe()
    }

    // Maintenance operations
    pub async fn cleanup_old_data(&self) -> Result<u64> {
        info!("Starting data cleanup based on retention policy");

        let retention_days = self.config.retention.retention_days;
        let cutoff_timestamp = chrono::Utc::now() - chrono::Duration::days(retention_days as i64);

        let cleanup_sql = format!(
            "DELETE FROM transactions WHERE block_time < '{}'",
            cutoff_timestamp.to_rfc3339()
        );

        let mut backend = self.backend.write().await;
        let deleted_count = backend.execute(&cleanup_sql).await?;

        // Also cleanup orphaned blocks
        let block_cleanup_sql = format!(
            "DELETE FROM blocks WHERE block_time < '{}'",
            cutoff_timestamp.to_rfc3339()
        );
        let deleted_blocks = backend.execute(&block_cleanup_sql).await?;

        let total_deleted = deleted_count + deleted_blocks;
        info!("Cleaned up {} old records", total_deleted);

        Ok(total_deleted)
    }

    pub async fn optimize_database(&self) -> Result<()> {
        info!("Optimizing database");

        let mut backend = self.backend.write().await;

        // Run database optimization commands
        let optimize_commands = vec![
            "ANALYZE;",
            "VACUUM;",
        ];

        for command in optimize_commands {
            if let Err(e) = backend.execute(command).await {
                warn!("Optimization command failed: {}: {}", command, e);
            }
        }

        info!("Database optimization completed");
        Ok(())
    }


}