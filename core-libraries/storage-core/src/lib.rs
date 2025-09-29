pub mod backend;
pub mod config;
pub mod manager;
pub mod query;
pub mod schema;
pub mod compression;
pub mod cache;

pub use backend::{StorageBackend, DuckDBBackend};
pub use config::StorageConfig;
pub use manager::StorageManager;
pub use query::{QueryEngine, QueryResult, QueryBuilder};
pub use schema::{SchemaManager, TableSchema};
pub use cache::{StorageCache, CacheStats};