use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub name: String,
    pub columns: Vec<ColumnDefinition>,
    pub indexes: Vec<IndexDefinition>,
    pub constraints: Vec<ConstraintDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataType {
    Text,
    Integer,
    BigInt,
    Real,
    Boolean,
    Timestamp,
    Json,
    Blob,
    Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDefinition {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub index_type: IndexType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexType {
    BTree,
    Hash,
    Gin,
    Gist,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintDefinition {
    pub name: String,
    pub constraint_type: ConstraintType,
    pub columns: Vec<String>,
    pub reference_table: Option<String>,
    pub reference_columns: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintType {
    PrimaryKey,
    ForeignKey,
    Unique,
    Check(String),
    NotNull,
}

pub struct SchemaManager {
    schemas: HashMap<String, TableSchema>,
}

impl SchemaManager {
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
        }
    }

    pub fn initialize_solana_schema(&mut self) -> Result<()> {
        info!("Initializing Solana transaction schema");

        // Blocks table
        let blocks_schema = TableSchema {
            name: "blocks".to_string(),
            columns: vec![
                ColumnDefinition {
                    name: "slot".to_string(),
                    data_type: DataType::BigInt,
                    nullable: false,
                    default_value: None,
                },
                ColumnDefinition {
                    name: "blockhash".to_string(),
                    data_type: DataType::Text,
                    nullable: false,
                    default_value: None,
                },
                ColumnDefinition {
                    name: "parent_slot".to_string(),
                    data_type: DataType::BigInt,
                    nullable: false,
                    default_value: None,
                },
                ColumnDefinition {
                    name: "block_time".to_string(),
                    data_type: DataType::Timestamp,
                    nullable: true,
                    default_value: None,
                },
                ColumnDefinition {
                    name: "block_height".to_string(),
                    data_type: DataType::BigInt,
                    nullable: true,
                    default_value: None,
                },
                ColumnDefinition {
                    name: "transaction_count".to_string(),
                    data_type: DataType::Integer,
                    nullable: false,
                    default_value: Some("0".to_string()),
                },
                ColumnDefinition {
                    name: "successful_transactions".to_string(),
                    data_type: DataType::Integer,
                    nullable: false,
                    default_value: Some("0".to_string()),
                },
                ColumnDefinition {
                    name: "failed_transactions".to_string(),
                    data_type: DataType::Integer,
                    nullable: false,
                    default_value: Some("0".to_string()),
                },
                ColumnDefinition {
                    name: "total_fees".to_string(),
                    data_type: DataType::BigInt,
                    nullable: false,
                    default_value: Some("0".to_string()),
                },
                ColumnDefinition {
                    name: "leader".to_string(),
                    data_type: DataType::Text,
                    nullable: false,
                    default_value: None,
                },
                ColumnDefinition {
                    name: "created_at".to_string(),
                    data_type: DataType::Timestamp,
                    nullable: false,
                    default_value: Some("CURRENT_TIMESTAMP".to_string()),
                },
            ],
            indexes: vec![
                IndexDefinition {
                    name: "idx_blocks_slot".to_string(),
                    columns: vec!["slot".to_string()],
                    unique: true,
                    index_type: IndexType::BTree,
                },
                IndexDefinition {
                    name: "idx_blocks_block_time".to_string(),
                    columns: vec!["block_time".to_string()],
                    unique: false,
                    index_type: IndexType::BTree,
                },
                IndexDefinition {
                    name: "idx_blocks_leader".to_string(),
                    columns: vec!["leader".to_string()],
                    unique: false,
                    index_type: IndexType::Hash,
                },
            ],
            constraints: vec![
                ConstraintDefinition {
                    name: "pk_blocks".to_string(),
                    constraint_type: ConstraintType::PrimaryKey,
                    columns: vec!["slot".to_string()],
                    reference_table: None,
                    reference_columns: None,
                },
            ],
        };

        // Transactions table
        let transactions_schema = TableSchema {
            name: "transactions".to_string(),
            columns: vec![
                ColumnDefinition {
                    name: "id".to_string(),
                    data_type: DataType::Uuid,
                    nullable: false,
                    default_value: None,
                },
                ColumnDefinition {
                    name: "signature".to_string(),
                    data_type: DataType::Text,
                    nullable: false,
                    default_value: None,
                },
                ColumnDefinition {
                    name: "slot".to_string(),
                    data_type: DataType::BigInt,
                    nullable: false,
                    default_value: None,
                },
                ColumnDefinition {
                    name: "block_time".to_string(),
                    data_type: DataType::Timestamp,
                    nullable: true,
                    default_value: None,
                },
                ColumnDefinition {
                    name: "fee".to_string(),
                    data_type: DataType::BigInt,
                    nullable: false,
                    default_value: None,
                },
                ColumnDefinition {
                    name: "success".to_string(),
                    data_type: DataType::Boolean,
                    nullable: false,
                    default_value: None,
                },
                ColumnDefinition {
                    name: "compute_units_consumed".to_string(),
                    data_type: DataType::BigInt,
                    nullable: true,
                    default_value: None,
                },
                ColumnDefinition {
                    name: "log_messages".to_string(),
                    data_type: DataType::Json,
                    nullable: true,
                    default_value: None,
                },
                ColumnDefinition {
                    name: "created_at".to_string(),
                    data_type: DataType::Timestamp,
                    nullable: false,
                    default_value: Some("CURRENT_TIMESTAMP".to_string()),
                },
            ],
            indexes: vec![
                IndexDefinition {
                    name: "idx_transactions_signature".to_string(),
                    columns: vec!["signature".to_string()],
                    unique: true,
                    index_type: IndexType::Hash,
                },
                IndexDefinition {
                    name: "idx_transactions_slot".to_string(),
                    columns: vec!["slot".to_string()],
                    unique: false,
                    index_type: IndexType::BTree,
                },
                IndexDefinition {
                    name: "idx_transactions_block_time".to_string(),
                    columns: vec!["block_time".to_string()],
                    unique: false,
                    index_type: IndexType::BTree,
                },
                IndexDefinition {
                    name: "idx_transactions_success".to_string(),
                    columns: vec!["success".to_string()],
                    unique: false,
                    index_type: IndexType::Hash,
                },
            ],
            constraints: vec![
                ConstraintDefinition {
                    name: "pk_transactions".to_string(),
                    constraint_type: ConstraintType::PrimaryKey,
                    columns: vec!["id".to_string()],
                    reference_table: None,
                    reference_columns: None,
                },
                ConstraintDefinition {
                    name: "fk_transactions_slot".to_string(),
                    constraint_type: ConstraintType::ForeignKey,
                    columns: vec!["slot".to_string()],
                    reference_table: Some("blocks".to_string()),
                    reference_columns: Some(vec!["slot".to_string()]),
                },
            ],
        };

        // Accounts table
        let accounts_schema = TableSchema {
            name: "accounts".to_string(),
            columns: vec![
                ColumnDefinition {
                    name: "id".to_string(),
                    data_type: DataType::Uuid,
                    nullable: false,
                    default_value: None,
                },
                ColumnDefinition {
                    name: "transaction_id".to_string(),
                    data_type: DataType::Uuid,
                    nullable: false,
                    default_value: None,
                },
                ColumnDefinition {
                    name: "pubkey".to_string(),
                    data_type: DataType::Text,
                    nullable: false,
                    default_value: None,
                },
                ColumnDefinition {
                    name: "is_signer".to_string(),
                    data_type: DataType::Boolean,
                    nullable: false,
                    default_value: Some("false".to_string()),
                },
                ColumnDefinition {
                    name: "is_writable".to_string(),
                    data_type: DataType::Boolean,
                    nullable: false,
                    default_value: Some("false".to_string()),
                },
                ColumnDefinition {
                    name: "pre_balance".to_string(),
                    data_type: DataType::BigInt,
                    nullable: false,
                    default_value: Some("0".to_string()),
                },
                ColumnDefinition {
                    name: "post_balance".to_string(),
                    data_type: DataType::BigInt,
                    nullable: false,
                    default_value: Some("0".to_string()),
                },
            ],
            indexes: vec![
                IndexDefinition {
                    name: "idx_accounts_transaction_id".to_string(),
                    columns: vec!["transaction_id".to_string()],
                    unique: false,
                    index_type: IndexType::Hash,
                },
                IndexDefinition {
                    name: "idx_accounts_pubkey".to_string(),
                    columns: vec!["pubkey".to_string()],
                    unique: false,
                    index_type: IndexType::Hash,
                },
            ],
            constraints: vec![
                ConstraintDefinition {
                    name: "pk_accounts".to_string(),
                    constraint_type: ConstraintType::PrimaryKey,
                    columns: vec!["id".to_string()],
                    reference_table: None,
                    reference_columns: None,
                },
                ConstraintDefinition {
                    name: "fk_accounts_transaction".to_string(),
                    constraint_type: ConstraintType::ForeignKey,
                    columns: vec!["transaction_id".to_string()],
                    reference_table: Some("transactions".to_string()),
                    reference_columns: Some(vec!["id".to_string()]),
                },
            ],
        };

        // Instructions table
        let instructions_schema = TableSchema {
            name: "instructions".to_string(),
            columns: vec![
                ColumnDefinition {
                    name: "id".to_string(),
                    data_type: DataType::Uuid,
                    nullable: false,
                    default_value: None,
                },
                ColumnDefinition {
                    name: "transaction_id".to_string(),
                    data_type: DataType::Uuid,
                    nullable: false,
                    default_value: None,
                },
                ColumnDefinition {
                    name: "program_id".to_string(),
                    data_type: DataType::Text,
                    nullable: false,
                    default_value: None,
                },
                ColumnDefinition {
                    name: "accounts".to_string(),
                    data_type: DataType::Json,
                    nullable: false,
                    default_value: Some("[]".to_string()),
                },
                ColumnDefinition {
                    name: "data".to_string(),
                    data_type: DataType::Text,
                    nullable: false,
                    default_value: None,
                },
                ColumnDefinition {
                    name: "instruction_index".to_string(),
                    data_type: DataType::Integer,
                    nullable: false,
                    default_value: None,
                },
            ],
            indexes: vec![
                IndexDefinition {
                    name: "idx_instructions_transaction_id".to_string(),
                    columns: vec!["transaction_id".to_string()],
                    unique: false,
                    index_type: IndexType::Hash,
                },
                IndexDefinition {
                    name: "idx_instructions_program_id".to_string(),
                    columns: vec!["program_id".to_string()],
                    unique: false,
                    index_type: IndexType::Hash,
                },
            ],
            constraints: vec![
                ConstraintDefinition {
                    name: "pk_instructions".to_string(),
                    constraint_type: ConstraintType::PrimaryKey,
                    columns: vec!["id".to_string()],
                    reference_table: None,
                    reference_columns: None,
                },
                ConstraintDefinition {
                    name: "fk_instructions_transaction".to_string(),
                    constraint_type: ConstraintType::ForeignKey,
                    columns: vec!["transaction_id".to_string()],
                    reference_table: Some("transactions".to_string()),
                    reference_columns: Some(vec!["id".to_string()]),
                },
            ],
        };

        // Register schemas
        self.schemas.insert("blocks".to_string(), blocks_schema);
        self.schemas.insert("transactions".to_string(), transactions_schema);
        self.schemas.insert("accounts".to_string(), accounts_schema);
        self.schemas.insert("instructions".to_string(), instructions_schema);

        info!("Initialized {} table schemas for Solana data", self.schemas.len());
        Ok(())
    }

    pub fn get_schema(&self, table_name: &str) -> Option<&TableSchema> {
        self.schemas.get(table_name)
    }

    pub fn get_all_schemas(&self) -> &HashMap<String, TableSchema> {
        &self.schemas
    }

    pub fn generate_create_table_sql(&self, table_name: &str) -> Result<String> {
        let schema = self.get_schema(table_name)
            .ok_or_else(|| anyhow::anyhow!("Schema not found: {}", table_name))?;

        let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (\n", schema.name);

        // Add columns
        let columns: Vec<String> = schema.columns.iter().map(|col| {
            let data_type_str = match &col.data_type {
                DataType::Text => "TEXT",
                DataType::Integer => "INTEGER",
                DataType::BigInt => "BIGINT",
                DataType::Real => "REAL",
                DataType::Boolean => "BOOLEAN",
                DataType::Timestamp => "TIMESTAMP",
                DataType::Json => "JSON",
                DataType::Blob => "BLOB",
                DataType::Uuid => "UUID",
            };

            let nullable_str = if col.nullable { "" } else { " NOT NULL" };

            let default_str = if let Some(default) = &col.default_value {
                format!(" DEFAULT {}", default)
            } else {
                "".to_string()
            };

            format!("    {} {}{}{}", col.name, data_type_str, nullable_str, default_str)
        }).collect();

        sql.push_str(&columns.join(",\n"));

        // Add constraints
        for constraint in &schema.constraints {
            sql.push_str(",\n");
            match &constraint.constraint_type {
                ConstraintType::PrimaryKey => {
                    sql.push_str(&format!("    CONSTRAINT {} PRIMARY KEY ({})",
                        constraint.name, constraint.columns.join(", ")));
                }
                ConstraintType::ForeignKey => {
                    if let (Some(ref_table), Some(ref_cols)) = (&constraint.reference_table, &constraint.reference_columns) {
                        sql.push_str(&format!("    CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {} ({})",
                            constraint.name,
                            constraint.columns.join(", "),
                            ref_table,
                            ref_cols.join(", ")));
                    }
                }
                ConstraintType::Unique => {
                    sql.push_str(&format!("    CONSTRAINT {} UNIQUE ({})",
                        constraint.name, constraint.columns.join(", ")));
                }
                ConstraintType::Check(condition) => {
                    sql.push_str(&format!("    CONSTRAINT {} CHECK ({})",
                        constraint.name, condition));
                }
                ConstraintType::NotNull => {
                    // NOT NULL is handled in column definition
                }
            }
        }

        sql.push_str("\n);");
        Ok(sql)
    }

    pub fn generate_create_index_sql(&self, table_name: &str) -> Result<Vec<String>> {
        let schema = self.get_schema(table_name)
            .ok_or_else(|| anyhow::anyhow!("Schema not found: {}", table_name))?;

        let mut sqls = Vec::new();

        for index in &schema.indexes {
            let unique_str = if index.unique { "UNIQUE " } else { "" };
            let sql = format!(
                "CREATE {}INDEX IF NOT EXISTS {} ON {} ({});",
                unique_str,
                index.name,
                schema.name,
                index.columns.join(", ")
            );
            sqls.push(sql);
        }

        Ok(sqls)
    }
}