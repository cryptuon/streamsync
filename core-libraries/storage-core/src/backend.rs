use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

use crate::config::StorageConfig;
use crate::schema::SchemaManager;

#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn initialize(&mut self, config: &StorageConfig) -> Result<()>;
    async fn create_tables(&mut self, schema_manager: &SchemaManager) -> Result<()>;
    async fn insert_batch(&mut self, table: &str, records: Vec<serde_json::Value>) -> Result<usize>;
    async fn query(&self, sql: &str) -> Result<Vec<serde_json::Value>>;
    async fn count_records(&self, table: &str) -> Result<u64>;
    async fn get_table_stats(&self, table: &str) -> Result<TableStats>;
    async fn execute(&mut self, sql: &str) -> Result<u64>;
    async fn close(&mut self) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct TableStats {
    pub table_name: String,
    pub record_count: u64,
    pub size_bytes: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

// Simplified in-memory backend for demonstration
pub struct DuckDBBackend {
    tables: Arc<RwLock<HashMap<String, Vec<serde_json::Value>>>>,
    config: Option<StorageConfig>,
}

impl DuckDBBackend {
    pub fn new() -> Self {
        Self {
            tables: Arc::new(RwLock::new(HashMap::new())),
            config: None,
        }
    }
}

#[async_trait]
impl StorageBackend for DuckDBBackend {
    async fn initialize(&mut self, config: &StorageConfig) -> Result<()> {
        info!("Initializing simplified storage backend");
        self.config = Some(config.clone());
        info!("Storage backend initialized successfully");
        Ok(())
    }

    async fn create_tables(&mut self, schema_manager: &SchemaManager) -> Result<()> {
        info!("Creating database tables");
        let mut tables = self.tables.write().await;

        let schemas = schema_manager.get_all_schemas();
        for table_name in schemas.keys() {
            tables.insert(table_name.clone(), Vec::new());
            debug!("Created table: {}", table_name);
        }

        info!("Created {} tables successfully", schemas.len());
        Ok(())
    }

    async fn insert_batch(&mut self, table: &str, records: Vec<serde_json::Value>) -> Result<usize> {
        if records.is_empty() {
            return Ok(0);
        }

        let mut tables = self.tables.write().await;
        let table_data = tables.entry(table.to_string()).or_insert_with(Vec::new);

        let count = records.len();
        table_data.extend(records);

        debug!("Inserted {} records into table {}", count, table);
        Ok(count)
    }

    async fn query(&self, sql: &str) -> Result<Vec<serde_json::Value>> {
        debug!("Executing query: {}", sql);

        // Very simplified query parsing - just return empty for demo
        // In a real implementation, this would parse SQL and execute against data
        let tables = self.tables.read().await;

        if sql.contains("SELECT COUNT(*)") {
            if let Some(table_name) = extract_table_name_from_count(sql) {
                if let Some(table_data) = tables.get(&table_name) {
                    return Ok(vec![serde_json::json!({ "count": table_data.len() })]);
                }
            }
            return Ok(vec![serde_json::json!({ "count": 0 })]);
        }

        // For demo purposes, return empty results
        Ok(Vec::new())
    }

    async fn count_records(&self, table: &str) -> Result<u64> {
        let tables = self.tables.read().await;
        let count = tables.get(table).map(|data| data.len()).unwrap_or(0);
        Ok(count as u64)
    }

    async fn get_table_stats(&self, table: &str) -> Result<TableStats> {
        let count = self.count_records(table).await?;
        let size_bytes = count * 1024; // Rough estimate

        Ok(TableStats {
            table_name: table.to_string(),
            record_count: count,
            size_bytes,
            created_at: chrono::Utc::now(),
            last_updated: chrono::Utc::now(),
        })
    }

    async fn execute(&mut self, sql: &str) -> Result<u64> {
        debug!("Executing SQL: {}", sql);
        // Simplified - just return success
        Ok(1)
    }

    async fn close(&mut self) -> Result<()> {
        info!("Closing storage backend");
        self.config = None;
        Ok(())
    }
}

fn extract_table_name_from_count(sql: &str) -> Option<String> {
    // Very basic SQL parsing to extract table name from COUNT queries
    let sql_lower = sql.to_lowercase();
    if let Some(from_pos) = sql_lower.find("from ") {
        let after_from = &sql_lower[from_pos + 5..];
        let table_name = after_from
            .split_whitespace()
            .next()?
            .trim_matches(|c: char| !c.is_alphanumeric() && c != '_')
            .to_string();
        return Some(table_name);
    }
    None
}