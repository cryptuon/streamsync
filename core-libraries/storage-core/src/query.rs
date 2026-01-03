use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::backend::{StorageBackend, TableStats};
use crate::cache::StorageCache;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub query_id: Uuid,
    pub rows: Vec<serde_json::Value>,
    pub row_count: usize,
    pub execution_time_ms: u128,
    pub cache_hit: bool,
    pub metadata: QueryMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetadata {
    pub tables_accessed: Vec<String>,
    pub index_usage: Vec<String>,
    pub estimated_cost: f64,
    pub actual_cost: f64,
}

#[derive(Debug, Clone)]
pub struct QueryStats {
    pub total_queries: u64,
    pub successful_queries: u64,
    pub failed_queries: u64,
    pub average_execution_time_ms: f64,
    pub cache_hit_rate: f64,
    pub slow_queries: u64,
    pub most_frequent_tables: HashMap<String, u64>,
}

pub struct QueryEngine {
    backend: Arc<RwLock<dyn StorageBackend>>,
    cache: Option<StorageCache>,
    stats: Arc<RwLock<QueryStats>>,
    slow_query_threshold_ms: u128,
}

impl QueryEngine {
    pub fn new(backend: Arc<RwLock<dyn StorageBackend>>) -> Self {
        Self {
            backend,
            cache: None,
            stats: Arc::new(RwLock::new(QueryStats {
                total_queries: 0,
                successful_queries: 0,
                failed_queries: 0,
                average_execution_time_ms: 0.0,
                cache_hit_rate: 0.0,
                slow_queries: 0,
                most_frequent_tables: HashMap::new(),
            })),
            slow_query_threshold_ms: 1000, // 1 second
        }
    }

    pub fn with_cache(mut self, cache: StorageCache) -> Self {
        self.cache = Some(cache);
        self
    }

    pub fn with_slow_query_threshold(mut self, threshold_ms: u128) -> Self {
        self.slow_query_threshold_ms = threshold_ms;
        self
    }

    pub async fn execute(&self, sql: &str) -> Result<QueryResult> {
        let query_id = Uuid::new_v4();
        let start_time = std::time::Instant::now();

        info!("Executing query {}: {}", query_id, sql);

        // Try cache first if enabled
        let cache_key = self.generate_cache_key(sql);
        if let Some(cache) = &self.cache {
            if let Some(cached_result) = cache.get(&cache_key).await {
                debug!("Query {} served from cache", query_id);
                self.update_query_stats(true, 0, true).await;

                if let Ok(result) = serde_json::from_value::<QueryResult>(cached_result) {
                    return Ok(QueryResult {
                        query_id,
                        cache_hit: true,
                        ..result
                    });
                }
            }
        }

        // Execute against backend
        let result = match self.execute_backend_query(sql, query_id).await {
            Ok(mut result) => {
                let execution_time = start_time.elapsed().as_millis();
                result.execution_time_ms = execution_time;

                // Cache successful results if cache is enabled
                if let Some(cache) = &self.cache {
                    let cache_value = serde_json::to_value(&result)?;
                    if let Err(e) = cache.put(cache_key, cache_value).await {
                        warn!("Failed to cache query result: {}", e);
                    }
                }

                self.update_query_stats(true, execution_time, false).await;
                self.track_table_usage(sql).await;

                if execution_time > self.slow_query_threshold_ms {
                    self.increment_slow_query_count().await;
                    warn!("Slow query detected ({}ms): {}", execution_time, sql);
                }

                result
            }
            Err(e) => {
                let execution_time = start_time.elapsed().as_millis();
                self.update_query_stats(false, execution_time, false).await;
                return Err(e);
            }
        };

        Ok(result)
    }

    async fn execute_backend_query(&self, sql: &str, query_id: Uuid) -> Result<QueryResult> {
        let backend = self.backend.read().await;
        let rows = backend.query(sql).await?;

        Ok(QueryResult {
            query_id,
            row_count: rows.len(),
            rows,
            execution_time_ms: 0, // Will be set by caller
            cache_hit: false,
            metadata: QueryMetadata {
                tables_accessed: self.extract_table_names(sql),
                index_usage: vec![], // Simplified - would need query plan analysis
                estimated_cost: 1.0,
                actual_cost: 1.0,
            },
        })
    }

    pub async fn execute_batch(&self, queries: Vec<String>) -> Result<Vec<QueryResult>> {
        let mut results = Vec::new();

        for sql in queries {
            match self.execute(&sql).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    warn!("Batch query failed: {}", e);
                    // Continue with other queries in the batch
                }
            }
        }

        info!("Executed batch of {} queries, {} successful", results.len(), results.len());
        Ok(results)
    }

    pub async fn get_table_stats(&self, table_name: &str) -> Result<TableStats> {
        let backend = self.backend.read().await;
        backend.get_table_stats(table_name).await
    }

    pub async fn count_records(&self, table_name: &str) -> Result<u64> {
        let backend = self.backend.read().await;
        backend.count_records(table_name).await
    }

    pub async fn get_query_stats(&self) -> QueryStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    async fn update_query_stats(&self, success: bool, execution_time_ms: u128, cache_hit: bool) {
        let mut stats = self.stats.write().await;
        stats.total_queries += 1;

        if success {
            stats.successful_queries += 1;
        } else {
            stats.failed_queries += 1;
        }

        // Update average execution time
        let total_successful = stats.successful_queries;
        if total_successful > 0 {
            stats.average_execution_time_ms = (stats.average_execution_time_ms * (total_successful - 1) as f64
                + execution_time_ms as f64) / total_successful as f64;
        }

        // Update cache hit rate (simplified calculation)
        if cache_hit {
            stats.cache_hit_rate = (stats.cache_hit_rate * (stats.total_queries - 1) as f64 + 100.0) / stats.total_queries as f64;
        } else {
            stats.cache_hit_rate = (stats.cache_hit_rate * (stats.total_queries - 1) as f64) / stats.total_queries as f64;
        }
    }

    async fn track_table_usage(&self, sql: &str) {
        let table_names = self.extract_table_names(sql);
        let mut stats = self.stats.write().await;

        for table_name in table_names {
            *stats.most_frequent_tables.entry(table_name).or_insert(0) += 1;
        }
    }

    async fn increment_slow_query_count(&self) {
        let mut stats = self.stats.write().await;
        stats.slow_queries += 1;
    }

    fn generate_cache_key(&self, sql: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        sql.hash(&mut hasher);
        format!("query:{:x}", hasher.finish())
    }

    fn extract_table_names(&self, sql: &str) -> Vec<String> {
        // Very simplified table name extraction from SQL
        // In a real implementation, you'd use a proper SQL parser
        let mut tables = Vec::new();
        let sql_lower = sql.to_lowercase();

        // Look for FROM clauses
        let patterns = ["from ", "join ", "update ", "into "];

        for pattern in &patterns {
            if let Some(pos) = sql_lower.find(pattern) {
                let after_pattern = &sql_lower[pos + pattern.len()..];
                if let Some(table_name) = after_pattern.split_whitespace().next() {
                    let clean_name = table_name
                        .trim_matches(|c: char| !c.is_alphanumeric() && c != '_')
                        .to_string();
                    if !clean_name.is_empty() && !tables.contains(&clean_name) {
                        tables.push(clean_name);
                    }
                }
            }
        }

        tables
    }
}

// Query builder for common Solana queries
pub struct QueryBuilder {
    table: String,
    conditions: Vec<String>,
    order_by: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
}

impl QueryBuilder {
    pub fn new(table: &str) -> Self {
        Self {
            table: table.to_string(),
            conditions: Vec::new(),
            order_by: None,
            limit: None,
            offset: None,
        }
    }

    pub fn where_condition(mut self, condition: &str) -> Self {
        self.conditions.push(condition.to_string());
        self
    }

    pub fn order_by(mut self, column: &str, direction: &str) -> Self {
        self.order_by = Some(format!("{} {}", column, direction));
        self
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn build_select(&self) -> String {
        let mut sql = format!("SELECT * FROM {}", self.table);

        if !self.conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&self.conditions.join(" AND "));
        }

        if let Some(order) = &self.order_by {
            sql.push_str(" ORDER BY ");
            sql.push_str(order);
        }

        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = self.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        sql
    }

    pub fn build_count(&self) -> String {
        let mut sql = format!("SELECT COUNT(*) as count FROM {}", self.table);

        if !self.conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&self.conditions.join(" AND "));
        }

        sql
    }

    // Convenience methods for common Solana queries
    pub fn by_slot_range(self, start_slot: u64, end_slot: u64) -> Self {
        self.where_condition(&format!("slot >= {} AND slot <= {}", start_slot, end_slot))
    }

    pub fn by_signature(self, signature: &str) -> Self {
        self.where_condition(&format!("signature = '{}'", signature))
    }

    pub fn successful_only(self) -> Self {
        self.where_condition("success = true")
    }

    pub fn recent_first(self) -> Self {
        self.order_by("block_time", "DESC")
    }
}