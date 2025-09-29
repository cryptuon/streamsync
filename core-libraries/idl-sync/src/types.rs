//! Core types for IDL synchronization

use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Generated IDL with confidence metrics and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedIDL {
    pub program_id: Pubkey,
    pub idl: IDLDefinition,
    pub confidence: IDLConfidence,
    pub network_consensus: NetworkConsensus,
    pub metadata: IDLMetadata,
}

/// Complete IDL definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IDLDefinition {
    pub version: String,
    pub name: String,
    pub instructions: Vec<InstructionDefinition>,
    pub accounts: Vec<AccountDefinition>,
    pub types: Vec<TypeDefinition>,
    pub events: Vec<EventDefinition>,
    pub errors: Vec<ErrorDefinition>,
    pub constants: Vec<ConstantDefinition>,
}

/// Instruction definition with behavioral patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionDefinition {
    pub name: String,
    pub discriminator: Vec<u8>,
    pub accounts: Vec<AccountRequirement>,
    pub args: Vec<ArgumentDefinition>,
    pub behavior_patterns: InstructionBehaviorPattern,
    pub frequency_stats: InstructionFrequencyStats,
}

/// Account definition inferred from state changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountDefinition {
    pub name: String,
    pub fields: Vec<FieldDefinition>,
    pub size: Option<usize>,
    pub discriminator: Option<Vec<u8>>,
    pub access_patterns: AccountAccessPattern,
}

/// Type definitions for complex data structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDefinition {
    pub name: String,
    pub type_kind: TypeKind,
    pub fields: Vec<FieldDefinition>,
    pub usage_patterns: TypeUsagePattern,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeKind {
    Struct,
    Enum,
    Alias,
    Option,
    Vec,
    Array { size: usize },
}

/// Event definitions from program logs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDefinition {
    pub name: String,
    pub fields: Vec<FieldDefinition>,
    pub discriminator: Vec<u8>,
    pub emission_patterns: EventEmissionPattern,
}

/// Error definitions from failed transactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDefinition {
    pub name: String,
    pub code: u32,
    pub message: String,
    pub occurrence_patterns: ErrorOccurrencePattern,
}

/// Constant definitions inferred from program behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstantDefinition {
    pub name: String,
    pub value: ConstantValue,
    pub usage_context: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstantValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    String(String),
    Bytes(Vec<u8>),
    Pubkey(Pubkey),
}

/// Field definition with type information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub name: String,
    pub field_type: FieldType,
    pub optional: bool,
    pub offset: Option<usize>,
    pub constraints: FieldConstraints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldType {
    Bool,
    U8, U16, U32, U64, U128,
    I8, I16, I32, I64, I128,
    F32, F64,
    String,
    Pubkey,
    Bytes,
    Option(Box<FieldType>),
    Vec(Box<FieldType>),
    Array(Box<FieldType>, usize),
    Defined(String), // Reference to custom type
}

/// Constraints on field values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldConstraints {
    pub min_value: Option<serde_json::Value>,
    pub max_value: Option<serde_json::Value>,
    pub allowed_values: Option<Vec<serde_json::Value>>,
    pub pattern: Option<String>, // Regex pattern for strings
}

/// Account requirements for instructions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountRequirement {
    pub name: String,
    pub is_mut: bool,
    pub is_signer: bool,
    pub is_optional: bool,
    pub account_type: Option<String>,
    pub constraints: AccountConstraints,
}

/// Constraints on accounts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountConstraints {
    pub owner: Option<Pubkey>,
    pub size: Option<usize>,
    pub data_requirements: Option<String>,
    pub balance_requirements: Option<u64>,
}

/// Argument definition for instruction parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgumentDefinition {
    pub name: String,
    pub arg_type: FieldType,
    pub constraints: FieldConstraints,
    pub usage_patterns: ArgumentUsagePattern,
}

/// Behavioral patterns for instructions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionBehaviorPattern {
    pub typical_gas_usage: Option<u64>,
    pub account_modifications: Vec<AccountModificationPattern>,
    pub typical_execution_time: Option<u64>, // microseconds
    pub success_rate: f64,
    pub common_error_codes: Vec<u32>,
}

/// Account modification patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountModificationPattern {
    pub account_index: usize,
    pub modification_type: AccountModificationType,
    pub typical_size_change: Option<i64>,
    pub field_modifications: Vec<FieldModificationPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccountModificationType {
    Create,
    Update,
    Delete,
    Resize,
    Transfer,
}

/// Field modification patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldModificationPattern {
    pub field_name: String,
    pub modification_frequency: f64,
    pub typical_change_magnitude: Option<f64>,
    pub change_distribution: ChangeDistribution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeDistribution {
    Constant,
    Linear,
    Exponential,
    Random,
    Seasonal,
}

/// Frequency statistics for instructions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionFrequencyStats {
    pub total_calls: u64,
    pub unique_callers: u64,
    pub calls_per_day_avg: f64,
    pub peak_usage_times: Vec<DateTime<Utc>>,
    pub seasonal_patterns: Option<SeasonalPattern>,
}

/// Seasonal usage patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonalPattern {
    pub daily_pattern: Vec<f64>, // 24 hours
    pub weekly_pattern: Vec<f64>, // 7 days
    pub monthly_pattern: Option<Vec<f64>>, // 30 days
}

/// Access patterns for accounts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountAccessPattern {
    pub read_frequency: f64,
    pub write_frequency: f64,
    pub concurrent_access_probability: f64,
    pub typical_lifetime: Option<u64>, // slots
    pub size_distribution: SizeDistribution,
}

/// Size distribution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeDistribution {
    pub min_size: usize,
    pub max_size: usize,
    pub avg_size: f64,
    pub percentiles: HashMap<u8, usize>, // percentile -> size
}

/// Type usage patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeUsagePattern {
    pub usage_frequency: f64,
    pub common_field_combinations: Vec<Vec<String>>,
    pub serialization_patterns: SerializationPattern,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerializationPattern {
    Borsh,
    Anchor,
    Custom(String),
    Mixed,
}

/// Event emission patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEmissionPattern {
    pub emission_frequency: f64,
    pub typical_triggers: Vec<String>,
    pub data_correlation: Vec<DataCorrelation>,
}

/// Data correlation between events and state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataCorrelation {
    pub field_name: String,
    pub correlation_strength: f64,
    pub correlation_type: CorrelationType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CorrelationType {
    Positive,
    Negative,
    Inverse,
    NonLinear,
}

/// Error occurrence patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorOccurrencePattern {
    pub frequency: f64,
    pub common_triggers: Vec<ErrorTrigger>,
    pub user_impact: ErrorImpact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorTrigger {
    pub trigger_type: TriggerType,
    pub description: String,
    pub probability: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerType {
    InvalidArgument,
    InsufficientFunds,
    AccountConstraintViolation,
    PermissionDenied,
    ResourceExhaustion,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorImpact {
    pub severity: ErrorSeverity,
    pub recoverability: bool,
    pub typical_resolution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Argument usage patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgumentUsagePattern {
    pub value_distribution: ValueDistribution,
    pub correlation_with_success: f64,
    pub typical_sources: Vec<ArgumentSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValueDistribution {
    Uniform,
    Normal { mean: f64, std_dev: f64 },
    Categorical { categories: HashMap<String, f64> },
    Range { min: f64, max: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArgumentSource {
    UserInput,
    ProgramState,
    ExternalAPI,
    Constant,
    Derived,
}

/// Confidence metrics for generated IDL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IDLConfidence {
    pub overall_confidence: f64,
    pub instruction_confidence: HashMap<String, f64>,
    pub account_confidence: HashMap<String, f64>,
    pub type_confidence: HashMap<String, f64>,
    pub confidence_factors: ConfidenceFactors,
}

/// Factors affecting confidence calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceFactors {
    pub sample_size: u64,
    pub observation_period: chrono::Duration,
    pub pattern_consistency: f64,
    pub cross_validation_score: f64,
    pub expert_validation_score: Option<f64>,
}

/// Statistics for account usage across transactions
#[derive(Debug, Clone, Default)]
pub struct AccountUsageStats {
    pub total_references: u64,
    pub usage_types: std::collections::HashSet<AccountUsageType>,
    pub access_patterns: Vec<AccountAccessPattern>,
}

/// Represents different types of account usage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccountUsageType {
    Read,
    Write,
    Create,
    Delete,
    Resize,
}

/// Network consensus information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConsensus {
    pub agreement_score: f64,
    pub participating_nodes: u32,
    pub consensus_timestamp: DateTime<Utc>,
    pub disagreement_areas: Vec<DisagreementArea>,
    pub consensus_method: ConsensusMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisagreementArea {
    pub area_type: DisagreementType,
    pub description: String,
    pub conflicting_interpretations: Vec<String>,
    pub resolution_confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DisagreementType {
    InstructionDefinition,
    AccountStructure,
    TypeDefinition,
    BehaviorPattern,
    FrequencyStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusMethod {
    Majority,
    WeightedVoting,
    ExpertValidation,
    Hybrid,
}

/// Metadata for generated IDL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IDLMetadata {
    pub generation_timestamp: DateTime<Utc>,
    pub generator_version: String,
    pub source_transactions: u64,
    pub analysis_period: chrono::Duration,
    pub update_history: Vec<IDLUpdate>,
    pub validation_results: Vec<ValidationResult>,
}

/// IDL update record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IDLUpdate {
    pub update_type: UpdateType,
    pub timestamp: DateTime<Utc>,
    pub changes: Vec<IDLChange>,
    pub trigger: UpdateTrigger,
    pub confidence_change: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateType {
    NewInstruction,
    ModifiedInstruction,
    NewAccount,
    ModifiedAccount,
    NewType,
    ConfidenceAdjustment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IDLChange {
    pub change_type: ChangeType,
    pub element_path: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: serde_json::Value,
    pub confidence_impact: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Addition,
    Modification,
    Removal,
    Rename,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateTrigger {
    NewTransaction,
    PatternChange,
    NetworkConsensus,
    ManualReview,
    ScheduledUpdate,
}

/// Validation results for IDL accuracy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub validation_type: ValidationType,
    pub result: ValidationOutcome,
    pub timestamp: DateTime<Utc>,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationType {
    TransactionReplay,
    StaticAnalysis,
    CrossReference,
    ExpertReview,
    AutomatedTesting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationOutcome {
    Pass,
    Fail,
    Warning,
    Unknown,
}

/// Configuration for IDL analysis
#[derive(Debug, Clone)]
pub struct IDLAnalysisConfig {
    pub min_sample_size: u64,
    pub confidence_threshold: f64,
    pub pattern_detection_sensitivity: f64,
    pub update_frequency: chrono::Duration,
    pub enable_real_time_updates: bool,
    pub network_consensus_required: bool,
    pub max_analysis_period: chrono::Duration,
}

impl Default for IDLAnalysisConfig {
    fn default() -> Self {
        Self {
            min_sample_size: 100,
            confidence_threshold: 0.8,
            pattern_detection_sensitivity: 0.7,
            update_frequency: chrono::Duration::minutes(5),
            enable_real_time_updates: true,
            network_consensus_required: true,
            max_analysis_period: chrono::Duration::days(30),
        }
    }
}

/// Pattern analysis result
#[derive(Debug, Clone)]
pub struct IDLPattern {
    pub pattern_type: PatternType,
    pub pattern_data: Vec<u8>,
    pub confidence: f64,
    pub frequency: u64,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum PatternType {
    InstructionSequence,
    AccountModification,
    DataStructure,
    ErrorCondition,
    EventEmission,
}

/// Instruction pattern analysis
#[derive(Debug, Clone)]
pub struct InstructionPattern {
    pub instruction_data: Vec<u8>,
    pub account_pattern: Vec<AccountUsage>,
    pub success_rate: f64,
    pub gas_usage_pattern: GasUsagePattern,
    pub timing_pattern: TimingPattern,
}

#[derive(Debug, Clone)]
pub struct AccountUsage {
    pub account_index: usize,
    pub usage_type: AccountUsageType,
    pub access_pattern: AccessPattern,
}


#[derive(Debug, Clone)]
pub struct AccessPattern {
    pub frequency: f64,
    pub typical_size: Option<usize>,
    pub modification_pattern: Option<ModificationPattern>,
}

#[derive(Debug, Clone)]
pub struct ModificationPattern {
    pub fields_modified: Vec<String>,
    pub modification_frequency: f64,
    pub typical_change_size: usize,
}

#[derive(Debug, Clone)]
pub struct GasUsagePattern {
    pub min_gas: u64,
    pub max_gas: u64,
    pub avg_gas: f64,
    pub gas_distribution: Vec<(u64, f64)>, // (gas_amount, frequency)
}

#[derive(Debug, Clone)]
pub struct TimingPattern {
    pub min_execution_time: u64, // microseconds
    pub max_execution_time: u64,
    pub avg_execution_time: f64,
    pub time_distribution: Vec<(u64, f64)>, // (time_us, frequency)
}

/// Account structure analysis result
#[derive(Debug, Clone)]
pub struct AccountStructure {
    pub account_type: String,
    pub typical_size: usize,
    pub field_layout: Vec<FieldLayout>,
    pub access_patterns: Vec<FieldAccessPattern>,
    pub lifecycle_pattern: AccountLifecyclePattern,
}

#[derive(Debug, Clone)]
pub struct FieldLayout {
    pub offset: usize,
    pub size: usize,
    pub field_type: InferredFieldType,
    pub constraints: InferredConstraints,
}

#[derive(Debug, Clone)]
pub enum InferredFieldType {
    Integer { bits: u8, signed: bool },
    FloatingPoint { bits: u8 },
    Boolean,
    PublicKey,
    String { max_length: Option<usize> },
    Bytes { length: Option<usize> },
    Array { element_type: Box<InferredFieldType>, length: usize },
    Option { inner_type: Box<InferredFieldType> },
    Enum { variants: Vec<String> },
    Struct { fields: Vec<FieldLayout> },
}

#[derive(Debug, Clone)]
pub struct InferredConstraints {
    pub min_value: Option<serde_json::Value>,
    pub max_value: Option<serde_json::Value>,
    pub required: bool,
    pub unique: bool,
    pub pattern: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FieldAccessPattern {
    pub field_offset: usize,
    pub read_frequency: f64,
    pub write_frequency: f64,
    pub correlation_with_other_fields: Vec<FieldCorrelation>,
}

#[derive(Debug, Clone)]
pub struct FieldCorrelation {
    pub other_field_offset: usize,
    pub correlation_strength: f64,
    pub correlation_type: CorrelationType,
}

#[derive(Debug, Clone)]
pub struct AccountLifecyclePattern {
    pub creation_triggers: Vec<String>,
    pub typical_lifetime: chrono::Duration,
    pub modification_frequency: f64,
    pub deletion_triggers: Vec<String>,
    pub archival_pattern: Option<ArchivalPattern>,
}

#[derive(Debug, Clone)]
pub struct ArchivalPattern {
    pub archival_threshold: chrono::Duration,
    pub archival_frequency: f64,
    pub archival_triggers: Vec<String>,
}