//! Error types for ZK reconstruction

use thiserror::Error;
use tracing::{error, warn};

pub type ReconstructionResult<T> = Result<T, ReconstructionError>;

/// Context information for reconstruction errors
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub operation: String,
    pub data_size: Option<usize>,
    pub strategy: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl ErrorContext {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            data_size: None,
            strategy: None,
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_data_size(mut self, size: usize) -> Self {
        self.data_size = Some(size);
        self
    }

    pub fn with_strategy(mut self, strategy: impl Into<String>) -> Self {
        self.strategy = Some(strategy.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

#[derive(Error, Debug, Clone)]
pub enum ReconstructionError {
    #[error("Invalid compression parameters: {details}")]
    InvalidCompressionParams {
        details: String,
        context: Option<ErrorContext>,
    },

    #[error("Truncated data is malformed: {reason}")]
    MalformedTruncatedData {
        reason: String,
        context: Option<ErrorContext>,
    },

    #[error("Merkle tree reconstruction failed: {details}")]
    MerkleReconstructionFailed {
        details: String,
        context: Option<ErrorContext>,
    },

    #[error("Constraint solving failed: {reason}")]
    ConstraintSolvingFailed {
        reason: String,
        context: Option<ErrorContext>,
    },

    #[error("Pattern matching failed: no known pattern for this data")]
    PatternMatchingFailed {
        context: Option<ErrorContext>,
    },

    #[error("Verification failed: {reason}")]
    VerificationFailed {
        reason: String,
        context: Option<ErrorContext>,
    },

    #[error("Insufficient data to perform reconstruction")]
    InsufficientData {
        context: Option<ErrorContext>,
    },

    #[error("Reconstruction timeout after {timeout_ms}ms")]
    Timeout {
        timeout_ms: u64,
        context: Option<ErrorContext>,
    },

    #[error("Cache error: {details}")]
    CacheError {
        details: String,
        context: Option<ErrorContext>,
    },

    #[error("Internal error: {details}")]
    Internal {
        details: String,
        context: Option<ErrorContext>,
    },
}

impl ReconstructionError {
    pub fn invalid_params(details: impl Into<String>) -> Self {
        let error = Self::InvalidCompressionParams {
            details: details.into(),
            context: None,
        };
        error.log_error();
        error
    }

    pub fn invalid_params_with_context(details: impl Into<String>, context: ErrorContext) -> Self {
        let error = Self::InvalidCompressionParams {
            details: details.into(),
            context: Some(context),
        };
        error.log_error();
        error
    }

    pub fn malformed_data(reason: impl Into<String>) -> Self {
        let error = Self::MalformedTruncatedData {
            reason: reason.into(),
            context: None,
        };
        error.log_error();
        error
    }

    pub fn malformed_data_with_context(reason: impl Into<String>, context: ErrorContext) -> Self {
        let error = Self::MalformedTruncatedData {
            reason: reason.into(),
            context: Some(context),
        };
        error.log_error();
        error
    }

    pub fn merkle_failed(details: impl Into<String>) -> Self {
        let error = Self::MerkleReconstructionFailed {
            details: details.into(),
            context: None,
        };
        error.log_error();
        error
    }

    pub fn merkle_failed_with_context(details: impl Into<String>, context: ErrorContext) -> Self {
        let error = Self::MerkleReconstructionFailed {
            details: details.into(),
            context: Some(context),
        };
        error.log_error();
        error
    }

    pub fn constraint_failed(reason: impl Into<String>) -> Self {
        let error = Self::ConstraintSolvingFailed {
            reason: reason.into(),
            context: None,
        };
        error.log_error();
        error
    }

    pub fn constraint_failed_with_context(reason: impl Into<String>, context: ErrorContext) -> Self {
        let error = Self::ConstraintSolvingFailed {
            reason: reason.into(),
            context: Some(context),
        };
        error.log_error();
        error
    }

    pub fn pattern_matching_failed() -> Self {
        let error = Self::PatternMatchingFailed { context: None };
        error.log_warning();
        error
    }

    pub fn pattern_matching_failed_with_context(context: ErrorContext) -> Self {
        let error = Self::PatternMatchingFailed {
            context: Some(context),
        };
        error.log_warning();
        error
    }

    pub fn verification_failed(reason: impl Into<String>) -> Self {
        let error = Self::VerificationFailed {
            reason: reason.into(),
            context: None,
        };
        error.log_error();
        error
    }

    pub fn verification_failed_with_context(reason: impl Into<String>, context: ErrorContext) -> Self {
        let error = Self::VerificationFailed {
            reason: reason.into(),
            context: Some(context),
        };
        error.log_error();
        error
    }

    pub fn insufficient_data() -> Self {
        let error = Self::InsufficientData { context: None };
        error.log_warning();
        error
    }

    pub fn insufficient_data_with_context(context: ErrorContext) -> Self {
        let error = Self::InsufficientData {
            context: Some(context),
        };
        error.log_warning();
        error
    }

    pub fn timeout(timeout_ms: u64) -> Self {
        let error = Self::Timeout {
            timeout_ms,
            context: None,
        };
        error.log_warning();
        error
    }

    pub fn timeout_with_context(timeout_ms: u64, context: ErrorContext) -> Self {
        let error = Self::Timeout {
            timeout_ms,
            context: Some(context),
        };
        error.log_warning();
        error
    }

    pub fn cache_error(details: impl Into<String>) -> Self {
        let error = Self::CacheError {
            details: details.into(),
            context: None,
        };
        error.log_warning();
        error
    }

    pub fn cache_error_with_context(details: impl Into<String>, context: ErrorContext) -> Self {
        let error = Self::CacheError {
            details: details.into(),
            context: Some(context),
        };
        error.log_warning();
        error
    }

    pub fn internal(details: impl Into<String>) -> Self {
        let error = Self::Internal {
            details: details.into(),
            context: None,
        };
        error.log_error();
        error
    }

    pub fn internal_with_context(details: impl Into<String>, context: ErrorContext) -> Self {
        let error = Self::Internal {
            details: details.into(),
            context: Some(context),
        };
        error.log_error();
        error
    }

    /// Log this error at the appropriate level
    pub fn log_error(&self) {
        match self {
            Self::InvalidCompressionParams { details, context } => {
                if let Some(ctx) = context {
                    error!(
                        operation = %ctx.operation,
                        data_size = ?ctx.data_size,
                        strategy = ?ctx.strategy,
                        timestamp = %ctx.timestamp,
                        metadata = ?ctx.metadata,
                        "Invalid compression parameters: {}", details
                    );
                } else {
                    error!("Invalid compression parameters: {}", details);
                }
            }
            Self::MalformedTruncatedData { reason, context } => {
                if let Some(ctx) = context {
                    error!(
                        operation = %ctx.operation,
                        data_size = ?ctx.data_size,
                        strategy = ?ctx.strategy,
                        timestamp = %ctx.timestamp,
                        metadata = ?ctx.metadata,
                        "Malformed truncated data: {}", reason
                    );
                } else {
                    error!("Malformed truncated data: {}", reason);
                }
            }
            Self::MerkleReconstructionFailed { details, context } => {
                if let Some(ctx) = context {
                    error!(
                        operation = %ctx.operation,
                        data_size = ?ctx.data_size,
                        strategy = ?ctx.strategy,
                        timestamp = %ctx.timestamp,
                        metadata = ?ctx.metadata,
                        "Merkle tree reconstruction failed: {}", details
                    );
                } else {
                    error!("Merkle tree reconstruction failed: {}", details);
                }
            }
            Self::ConstraintSolvingFailed { reason, context } => {
                if let Some(ctx) = context {
                    error!(
                        operation = %ctx.operation,
                        data_size = ?ctx.data_size,
                        strategy = ?ctx.strategy,
                        timestamp = %ctx.timestamp,
                        metadata = ?ctx.metadata,
                        "Constraint solving failed: {}", reason
                    );
                } else {
                    error!("Constraint solving failed: {}", reason);
                }
            }
            Self::VerificationFailed { reason, context } => {
                if let Some(ctx) = context {
                    error!(
                        operation = %ctx.operation,
                        data_size = ?ctx.data_size,
                        strategy = ?ctx.strategy,
                        timestamp = %ctx.timestamp,
                        metadata = ?ctx.metadata,
                        "Verification failed: {}", reason
                    );
                } else {
                    error!("Verification failed: {}", reason);
                }
            }
            Self::Internal { details, context } => {
                if let Some(ctx) = context {
                    error!(
                        operation = %ctx.operation,
                        data_size = ?ctx.data_size,
                        strategy = ?ctx.strategy,
                        timestamp = %ctx.timestamp,
                        metadata = ?ctx.metadata,
                        "Internal error: {}", details
                    );
                } else {
                    error!("Internal error: {}", details);
                }
            }
            _ => {} // Other variants use log_warning
        }
    }

    /// Log this error as a warning
    pub fn log_warning(&self) {
        match self {
            Self::PatternMatchingFailed { context } => {
                if let Some(ctx) = context {
                    warn!(
                        operation = %ctx.operation,
                        data_size = ?ctx.data_size,
                        strategy = ?ctx.strategy,
                        timestamp = %ctx.timestamp,
                        metadata = ?ctx.metadata,
                        "Pattern matching failed: no known pattern for this data"
                    );
                } else {
                    warn!("Pattern matching failed: no known pattern for this data");
                }
            }
            Self::InsufficientData { context } => {
                if let Some(ctx) = context {
                    warn!(
                        operation = %ctx.operation,
                        data_size = ?ctx.data_size,
                        strategy = ?ctx.strategy,
                        timestamp = %ctx.timestamp,
                        metadata = ?ctx.metadata,
                        "Insufficient data to perform reconstruction"
                    );
                } else {
                    warn!("Insufficient data to perform reconstruction");
                }
            }
            Self::Timeout { timeout_ms, context } => {
                if let Some(ctx) = context {
                    warn!(
                        operation = %ctx.operation,
                        data_size = ?ctx.data_size,
                        strategy = ?ctx.strategy,
                        timestamp = %ctx.timestamp,
                        metadata = ?ctx.metadata,
                        timeout_ms = %timeout_ms,
                        "Reconstruction timeout after {}ms", timeout_ms
                    );
                } else {
                    warn!("Reconstruction timeout after {}ms", timeout_ms);
                }
            }
            Self::CacheError { details, context } => {
                if let Some(ctx) = context {
                    warn!(
                        operation = %ctx.operation,
                        data_size = ?ctx.data_size,
                        strategy = ?ctx.strategy,
                        timestamp = %ctx.timestamp,
                        metadata = ?ctx.metadata,
                        "Cache error: {}", details
                    );
                } else {
                    warn!("Cache error: {}", details);
                }
            }
            _ => {} // Other variants use log_error
        }
    }

    /// Get the context from this error if available
    pub fn context(&self) -> Option<&ErrorContext> {
        match self {
            Self::InvalidCompressionParams { context, .. } => context.as_ref(),
            Self::MalformedTruncatedData { context, .. } => context.as_ref(),
            Self::MerkleReconstructionFailed { context, .. } => context.as_ref(),
            Self::ConstraintSolvingFailed { context, .. } => context.as_ref(),
            Self::PatternMatchingFailed { context } => context.as_ref(),
            Self::VerificationFailed { context, .. } => context.as_ref(),
            Self::InsufficientData { context } => context.as_ref(),
            Self::Timeout { context, .. } => context.as_ref(),
            Self::CacheError { context, .. } => context.as_ref(),
            Self::Internal { context, .. } => context.as_ref(),
        }
    }
}