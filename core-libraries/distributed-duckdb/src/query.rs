//! Query types and execution

use serde::{Deserialize, Serialize};

/// Query representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    pub sql: String,
}

/// Query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub data: Vec<u8>,
}