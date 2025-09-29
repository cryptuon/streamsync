//! Error types for IDL synchronization

use thiserror::Error;
use tracing::{error, warn};

pub type IDLResult<T> = Result<T, IDLError>;

/// Context information for IDL errors
#[derive(Debug, Clone)]
pub struct IDLErrorContext {
    pub operation: String,
    pub program_id: Option<String>,
    pub transaction_count: Option<usize>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl IDLErrorContext {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            program_id: None,
            transaction_count: None,
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_program_id(mut self, program_id: impl Into<String>) -> Self {
        self.program_id = Some(program_id.into());
        self
    }

    pub fn with_transaction_count(mut self, count: usize) -> Self {
        self.transaction_count = Some(count);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

#[derive(Error, Debug, Clone)]
pub enum IDLError {
    #[error("Transaction analysis failed: {details}")]
    TransactionAnalysisFailed {
        details: String,
        context: Option<IDLErrorContext>,
    },

    #[error("Pattern detection failed: {reason}")]
    PatternDetectionFailed {
        reason: String,
        context: Option<IDLErrorContext>,
    },

    #[error("IDL generation failed: {details}")]
    IDLGenerationFailed {
        details: String,
        context: Option<IDLErrorContext>,
    },

    #[error("Insufficient confidence: achieved {achieved:.2}, required {required:.2}")]
    InsufficientConfidence {
        achieved: f64,
        required: f64,
        context: Option<IDLErrorContext>,
    },

    #[error("Network consensus failed: {reason}")]
    NetworkConsensusFailed {
        reason: String,
        context: Option<IDLErrorContext>,
    },

    #[error("Invalid program behavior: {details}")]
    InvalidProgramBehavior {
        details: String,
        context: Option<IDLErrorContext>,
    },

    #[error("Instruction parsing failed: {reason}")]
    InstructionParsingFailed {
        reason: String,
        context: Option<IDLErrorContext>,
    },

    #[error("Account structure analysis failed: {details}")]
    AccountStructureAnalysisFailed {
        details: String,
        context: Option<IDLErrorContext>,
    },

    #[error("IDL update validation failed: {reason}")]
    IDLUpdateValidationFailed {
        reason: String,
        context: Option<IDLErrorContext>,
    },

    #[error("Real-time monitoring error: {details}")]
    RealTimeMonitoringError {
        details: String,
        context: Option<IDLErrorContext>,
    },

    #[error("Cache error: {details}")]
    CacheError {
        details: String,
        context: Option<IDLErrorContext>,
    },

    #[error("Serialization error: {details}")]
    SerializationError {
        details: String,
        context: Option<IDLErrorContext>,
    },

    #[error("Internal error: {details}")]
    Internal {
        details: String,
        context: Option<IDLErrorContext>,
    },
}

impl IDLError {
    pub fn transaction_analysis_failed(details: impl Into<String>) -> Self {
        let error = Self::TransactionAnalysisFailed {
            details: details.into(),
            context: None,
        };
        error.log_error();
        error
    }

    pub fn transaction_analysis_failed_with_context(details: impl Into<String>, context: IDLErrorContext) -> Self {
        let error = Self::TransactionAnalysisFailed {
            details: details.into(),
            context: Some(context),
        };
        error.log_error();
        error
    }

    pub fn pattern_detection_failed(reason: impl Into<String>) -> Self {
        let error = Self::PatternDetectionFailed {
            reason: reason.into(),
            context: None,
        };
        error.log_warning();
        error
    }

    pub fn pattern_detection_failed_with_context(reason: impl Into<String>, context: IDLErrorContext) -> Self {
        let error = Self::PatternDetectionFailed {
            reason: reason.into(),
            context: Some(context),
        };
        error.log_warning();
        error
    }

    pub fn idl_generation_failed(details: impl Into<String>) -> Self {
        let error = Self::IDLGenerationFailed {
            details: details.into(),
            context: None,
        };
        error.log_error();
        error
    }

    pub fn idl_generation_failed_with_context(details: impl Into<String>, context: IDLErrorContext) -> Self {
        let error = Self::IDLGenerationFailed {
            details: details.into(),
            context: Some(context),
        };
        error.log_error();
        error
    }

    pub fn insufficient_confidence(achieved: f64, required: f64) -> Self {
        let error = Self::InsufficientConfidence {
            achieved,
            required,
            context: None,
        };
        error.log_warning();
        error
    }

    pub fn insufficient_confidence_with_context(achieved: f64, required: f64, context: IDLErrorContext) -> Self {
        let error = Self::InsufficientConfidence {
            achieved,
            required,
            context: Some(context),
        };
        error.log_warning();
        error
    }

    pub fn network_consensus_failed(reason: impl Into<String>) -> Self {
        let error = Self::NetworkConsensusFailed {
            reason: reason.into(),
            context: None,
        };
        error.log_error();
        error
    }

    pub fn network_consensus_failed_with_context(reason: impl Into<String>, context: IDLErrorContext) -> Self {
        let error = Self::NetworkConsensusFailed {
            reason: reason.into(),
            context: Some(context),
        };
        error.log_error();
        error
    }

    pub fn invalid_program_behavior(details: impl Into<String>) -> Self {
        let error = Self::InvalidProgramBehavior {
            details: details.into(),
            context: None,
        };
        error.log_error();
        error
    }

    pub fn invalid_program_behavior_with_context(details: impl Into<String>, context: IDLErrorContext) -> Self {
        let error = Self::InvalidProgramBehavior {
            details: details.into(),
            context: Some(context),
        };
        error.log_error();
        error
    }

    pub fn instruction_parsing_failed(reason: impl Into<String>) -> Self {
        let error = Self::InstructionParsingFailed {
            reason: reason.into(),
            context: None,
        };
        error.log_warning();
        error
    }

    pub fn instruction_parsing_failed_with_context(reason: impl Into<String>, context: IDLErrorContext) -> Self {
        let error = Self::InstructionParsingFailed {
            reason: reason.into(),
            context: Some(context),
        };
        error.log_warning();
        error
    }

    pub fn account_structure_failed(details: impl Into<String>) -> Self {
        let error = Self::AccountStructureAnalysisFailed {
            details: details.into(),
            context: None,
        };
        error.log_error();
        error
    }

    pub fn account_structure_failed_with_context(details: impl Into<String>, context: IDLErrorContext) -> Self {
        let error = Self::AccountStructureAnalysisFailed {
            details: details.into(),
            context: Some(context),
        };
        error.log_error();
        error
    }

    pub fn update_validation_failed(reason: impl Into<String>) -> Self {
        let error = Self::IDLUpdateValidationFailed {
            reason: reason.into(),
            context: None,
        };
        error.log_error();
        error
    }

    pub fn update_validation_failed_with_context(reason: impl Into<String>, context: IDLErrorContext) -> Self {
        let error = Self::IDLUpdateValidationFailed {
            reason: reason.into(),
            context: Some(context),
        };
        error.log_error();
        error
    }

    pub fn real_time_monitoring_error(details: impl Into<String>) -> Self {
        let error = Self::RealTimeMonitoringError {
            details: details.into(),
            context: None,
        };
        error.log_warning();
        error
    }

    pub fn real_time_monitoring_error_with_context(details: impl Into<String>, context: IDLErrorContext) -> Self {
        let error = Self::RealTimeMonitoringError {
            details: details.into(),
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

    pub fn cache_error_with_context(details: impl Into<String>, context: IDLErrorContext) -> Self {
        let error = Self::CacheError {
            details: details.into(),
            context: Some(context),
        };
        error.log_warning();
        error
    }

    pub fn serialization_error(details: impl Into<String>) -> Self {
        let error = Self::SerializationError {
            details: details.into(),
            context: None,
        };
        error.log_warning();
        error
    }

    pub fn serialization_error_with_context(details: impl Into<String>, context: IDLErrorContext) -> Self {
        let error = Self::SerializationError {
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

    pub fn internal_with_context(details: impl Into<String>, context: IDLErrorContext) -> Self {
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
            Self::TransactionAnalysisFailed { details, context } => {
                if let Some(ctx) = context {
                    error!(
                        operation = %ctx.operation,
                        program_id = ?ctx.program_id,
                        transaction_count = ?ctx.transaction_count,
                        timestamp = %ctx.timestamp,
                        metadata = ?ctx.metadata,
                        "Transaction analysis failed: {}", details
                    );
                } else {
                    error!("Transaction analysis failed: {}", details);
                }
            }
            Self::IDLGenerationFailed { details, context } => {
                if let Some(ctx) = context {
                    error!(
                        operation = %ctx.operation,
                        program_id = ?ctx.program_id,
                        transaction_count = ?ctx.transaction_count,
                        timestamp = %ctx.timestamp,
                        metadata = ?ctx.metadata,
                        "IDL generation failed: {}", details
                    );
                } else {
                    error!("IDL generation failed: {}", details);
                }
            }
            Self::NetworkConsensusFailed { reason, context } => {
                if let Some(ctx) = context {
                    error!(
                        operation = %ctx.operation,
                        program_id = ?ctx.program_id,
                        transaction_count = ?ctx.transaction_count,
                        timestamp = %ctx.timestamp,
                        metadata = ?ctx.metadata,
                        "Network consensus failed: {}", reason
                    );
                } else {
                    error!("Network consensus failed: {}", reason);
                }
            }
            Self::InvalidProgramBehavior { details, context } => {
                if let Some(ctx) = context {
                    error!(
                        operation = %ctx.operation,
                        program_id = ?ctx.program_id,
                        transaction_count = ?ctx.transaction_count,
                        timestamp = %ctx.timestamp,
                        metadata = ?ctx.metadata,
                        "Invalid program behavior: {}", details
                    );
                } else {
                    error!("Invalid program behavior: {}", details);
                }
            }
            Self::AccountStructureAnalysisFailed { details, context } => {
                if let Some(ctx) = context {
                    error!(
                        operation = %ctx.operation,
                        program_id = ?ctx.program_id,
                        transaction_count = ?ctx.transaction_count,
                        timestamp = %ctx.timestamp,
                        metadata = ?ctx.metadata,
                        "Account structure analysis failed: {}", details
                    );
                } else {
                    error!("Account structure analysis failed: {}", details);
                }
            }
            Self::IDLUpdateValidationFailed { reason, context } => {
                if let Some(ctx) = context {
                    error!(
                        operation = %ctx.operation,
                        program_id = ?ctx.program_id,
                        transaction_count = ?ctx.transaction_count,
                        timestamp = %ctx.timestamp,
                        metadata = ?ctx.metadata,
                        "IDL update validation failed: {}", reason
                    );
                } else {
                    error!("IDL update validation failed: {}", reason);
                }
            }
            Self::Internal { details, context } => {
                if let Some(ctx) = context {
                    error!(
                        operation = %ctx.operation,
                        program_id = ?ctx.program_id,
                        transaction_count = ?ctx.transaction_count,
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
            Self::PatternDetectionFailed { reason, context } => {
                if let Some(ctx) = context {
                    warn!(
                        operation = %ctx.operation,
                        program_id = ?ctx.program_id,
                        transaction_count = ?ctx.transaction_count,
                        timestamp = %ctx.timestamp,
                        metadata = ?ctx.metadata,
                        "Pattern detection failed: {}", reason
                    );
                } else {
                    warn!("Pattern detection failed: {}", reason);
                }
            }
            Self::InsufficientConfidence { achieved, required, context } => {
                if let Some(ctx) = context {
                    warn!(
                        operation = %ctx.operation,
                        program_id = ?ctx.program_id,
                        transaction_count = ?ctx.transaction_count,
                        timestamp = %ctx.timestamp,
                        metadata = ?ctx.metadata,
                        achieved = %achieved,
                        required = %required,
                        "Insufficient confidence: achieved {:.2}, required {:.2}", achieved, required
                    );
                } else {
                    warn!("Insufficient confidence: achieved {:.2}, required {:.2}", achieved, required);
                }
            }
            Self::InstructionParsingFailed { reason, context } => {
                if let Some(ctx) = context {
                    warn!(
                        operation = %ctx.operation,
                        program_id = ?ctx.program_id,
                        transaction_count = ?ctx.transaction_count,
                        timestamp = %ctx.timestamp,
                        metadata = ?ctx.metadata,
                        "Instruction parsing failed: {}", reason
                    );
                } else {
                    warn!("Instruction parsing failed: {}", reason);
                }
            }
            Self::RealTimeMonitoringError { details, context } => {
                if let Some(ctx) = context {
                    warn!(
                        operation = %ctx.operation,
                        program_id = ?ctx.program_id,
                        transaction_count = ?ctx.transaction_count,
                        timestamp = %ctx.timestamp,
                        metadata = ?ctx.metadata,
                        "Real-time monitoring error: {}", details
                    );
                } else {
                    warn!("Real-time monitoring error: {}", details);
                }
            }
            Self::CacheError { details, context } => {
                if let Some(ctx) = context {
                    warn!(
                        operation = %ctx.operation,
                        program_id = ?ctx.program_id,
                        transaction_count = ?ctx.transaction_count,
                        timestamp = %ctx.timestamp,
                        metadata = ?ctx.metadata,
                        "Cache error: {}", details
                    );
                } else {
                    warn!("Cache error: {}", details);
                }
            }
            Self::SerializationError { details, context } => {
                if let Some(ctx) = context {
                    warn!(
                        operation = %ctx.operation,
                        program_id = ?ctx.program_id,
                        transaction_count = ?ctx.transaction_count,
                        timestamp = %ctx.timestamp,
                        metadata = ?ctx.metadata,
                        "Serialization error: {}", details
                    );
                } else {
                    warn!("Serialization error: {}", details);
                }
            }
            _ => {} // Other variants use log_error
        }
    }

    /// Get the context from this error if available
    pub fn context(&self) -> Option<&IDLErrorContext> {
        match self {
            Self::TransactionAnalysisFailed { context, .. } => context.as_ref(),
            Self::PatternDetectionFailed { context, .. } => context.as_ref(),
            Self::IDLGenerationFailed { context, .. } => context.as_ref(),
            Self::InsufficientConfidence { context, .. } => context.as_ref(),
            Self::NetworkConsensusFailed { context, .. } => context.as_ref(),
            Self::InvalidProgramBehavior { context, .. } => context.as_ref(),
            Self::InstructionParsingFailed { context, .. } => context.as_ref(),
            Self::AccountStructureAnalysisFailed { context, .. } => context.as_ref(),
            Self::IDLUpdateValidationFailed { context, .. } => context.as_ref(),
            Self::RealTimeMonitoringError { context, .. } => context.as_ref(),
            Self::CacheError { context, .. } => context.as_ref(),
            Self::SerializationError { context, .. } => context.as_ref(),
            Self::Internal { context, .. } => context.as_ref(),
        }
    }
}