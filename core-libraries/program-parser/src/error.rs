//! Error types for program parsing

use thiserror::Error;
use solana_sdk::pubkey::Pubkey;

pub type ParseResult<T> = Result<T, ParseError>;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Invalid program ID: {program_id}")]
    InvalidProgramId { program_id: Pubkey },

    #[error("Unsupported program: {program_id}")]
    UnsupportedProgram { program_id: Pubkey },

    #[error("Invalid instruction data: {reason}")]
    InvalidInstructionData { reason: String },

    #[error("Account parsing failed: {account} - {reason}")]
    AccountParsingFailed {
        account: Pubkey,
        reason: String,
    },

    #[error("Insufficient data: expected {expected} bytes, got {actual}")]
    InsufficientData { expected: usize, actual: usize },

    #[error("Invalid discriminator: expected {expected:?}, got {actual:?}")]
    InvalidDiscriminator {
        expected: Vec<u8>,
        actual: Vec<u8>,
    },

    #[error("Metadata lookup failed: {mint} - {reason}")]
    MetadataLookupFailed {
        mint: Pubkey,
        reason: String,
    },

    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Timeout: operation took longer than {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("Program detection failed: {reason}")]
    DetectionFailed { reason: String },

    #[error("Parser not found for program: {program_id}")]
    ParserNotFound { program_id: Pubkey },

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl ParseError {
    pub fn invalid_program_id(program_id: Pubkey) -> Self {
        Self::InvalidProgramId { program_id }
    }

    pub fn unsupported_program(program_id: Pubkey) -> Self {
        Self::UnsupportedProgram { program_id }
    }

    pub fn invalid_instruction_data<S: Into<String>>(reason: S) -> Self {
        Self::InvalidInstructionData {
            reason: reason.into(),
        }
    }

    pub fn account_parsing_failed<S: Into<String>>(account: Pubkey, reason: S) -> Self {
        Self::AccountParsingFailed {
            account,
            reason: reason.into(),
        }
    }

    pub fn insufficient_data(expected: usize, actual: usize) -> Self {
        Self::InsufficientData { expected, actual }
    }

    pub fn invalid_discriminator(expected: Vec<u8>, actual: Vec<u8>) -> Self {
        Self::InvalidDiscriminator { expected, actual }
    }

    pub fn metadata_lookup_failed<S: Into<String>>(mint: Pubkey, reason: S) -> Self {
        Self::MetadataLookupFailed {
            mint,
            reason: reason.into(),
        }
    }

    pub fn network<S: Into<String>>(reason: S) -> Self {
        Self::Network(reason.into())
    }

    pub fn cache<S: Into<String>>(reason: S) -> Self {
        Self::Cache(reason.into())
    }

    pub fn timeout(timeout_ms: u64) -> Self {
        Self::Timeout { timeout_ms }
    }

    pub fn detection_failed<S: Into<String>>(reason: S) -> Self {
        Self::DetectionFailed {
            reason: reason.into(),
        }
    }

    pub fn parser_not_found(program_id: Pubkey) -> Self {
        Self::ParserNotFound { program_id }
    }

    pub fn configuration<S: Into<String>>(reason: S) -> Self {
        Self::Configuration(reason.into())
    }

    pub fn internal<S: Into<String>>(reason: S) -> Self {
        Self::Internal(reason.into())
    }
}