//! Core types for ZK reconstruction

use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Truncated data from RPC logs (limited to 1KB)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TruncatedData {
    pub data: Vec<u8>,
    pub original_size_hint: Option<usize>,
    pub truncation_point: usize,
    pub metadata: TruncationMetadata,
}

/// Metadata about the truncation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TruncationMetadata {
    pub slot: u64,
    pub account: Pubkey,
    pub program_id: Pubkey,
    pub compression_type: CompressionType,
    pub truncation_timestamp: SystemTime,
}

/// Compression parameters for reconstruction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompressionParams {
    pub compression_type: CompressionType,
    pub merkle_tree_height: u32,
    pub leaf_count: u64,
    pub root_hash: [u8; 32],
    pub compression_program: Pubkey,
    pub additional_params: HashMap<String, Vec<u8>>,
}

impl Default for CompressionParams {
    fn default() -> Self {
        Self {
            compression_type: CompressionType::Standard,
            merkle_tree_height: 20,
            leaf_count: 0,
            root_hash: [0; 32],
            compression_program: Pubkey::default(),
            additional_params: HashMap::new(),
        }
    }
}

/// Types of compression used in Solana
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionType {
    /// Standard ZK compression
    Standard,
    /// Account compression with state trees
    StateCompression,
    /// Custom compression schemes
    Custom(String),
}

/// Reconstructed account data with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconstructedAccount {
    pub account_data: Vec<u8>,
    pub reconstruction_method: ReconstructionMethod,
    pub confidence_score: f64,
    pub verification_proof: Option<VerificationProof>,
    pub reconstruction_time: Duration,
    pub cache_hint: CacheHint,
}

/// Method used for reconstruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReconstructionMethod {
    MerkleTreeReconstruction,
    ConstraintSolving,
    PatternMatching { pattern_id: String },
    Hybrid { methods: Vec<ReconstructionMethod> },
}

/// Verification proof for reconstruction correctness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationProof {
    pub merkle_proof: Vec<[u8; 32]>,
    pub proof_type: ProofType,
    pub verification_data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProofType {
    MerkleInclusion,
    ZKProof,
    CryptographicHash,
}

/// Hint for caching strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheHint {
    pub cache_key: String,
    pub ttl: Duration,
    pub pattern_category: Option<String>,
    pub reuse_probability: f64,
}

/// Available reconstruction strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconstructionStrategy {
    /// Fast pattern matching for known patterns
    FastPattern,
    /// Mathematical reconstruction via constraint solving
    MathematicalReconstruction,
    /// Merkle tree reconstruction
    MerkleReconstruction,
    /// Hybrid approach combining multiple strategies
    Hybrid,
}

/// Complexity estimate for reconstruction
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComplexityEstimate {
    Trivial,    // < 1ms
    Low,        // 1-10ms
    Medium,     // 10-50ms
    High,       // 50-100ms
    VeryHigh,   // > 100ms
}

impl ComplexityEstimate {
    pub fn expected_duration(&self) -> Duration {
        match self {
            ComplexityEstimate::Trivial => Duration::from_millis(1),
            ComplexityEstimate::Low => Duration::from_millis(5),
            ComplexityEstimate::Medium => Duration::from_millis(25),
            ComplexityEstimate::High => Duration::from_millis(75),
            ComplexityEstimate::VeryHigh => Duration::from_millis(150),
        }
    }

    pub fn complexity_score(&self) -> f64 {
        match self {
            ComplexityEstimate::Trivial => 0.1,
            ComplexityEstimate::Low => 0.3,
            ComplexityEstimate::Medium => 0.5,
            ComplexityEstimate::High => 0.8,
            ComplexityEstimate::VeryHigh => 1.0,
        }
    }

    pub fn estimated_time_ms(&self) -> u64 {
        self.expected_duration().as_millis() as u64
    }

    pub fn from_data_size(size_bytes: usize) -> Self {
        match size_bytes {
            0..=1024 => ComplexityEstimate::Trivial,
            1025..=10240 => ComplexityEstimate::Low,
            10241..=102400 => ComplexityEstimate::Medium,
            102401..=1048576 => ComplexityEstimate::High,
            _ => ComplexityEstimate::VeryHigh,
        }
    }
}

/// Configuration for the reconstruction library
#[derive(Debug, Clone)]
pub struct ReconstructionConfig {
    pub max_reconstruction_time: Duration,
    pub max_input_size: usize,
    pub cache_size: usize,
    pub parallel_workers: usize,
    pub enable_pattern_learning: bool,
    pub verification_level: VerificationLevel,
}

impl Default for ReconstructionConfig {
    fn default() -> Self {
        Self {
            max_reconstruction_time: Duration::from_millis(100),
            max_input_size: 10 * 1024 * 1024, // 10MB max input
            cache_size: 10000,
            parallel_workers: num_cpus::get(),
            enable_pattern_learning: true,
            verification_level: VerificationLevel::Standard,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationLevel {
    None,
    Basic,
    Standard,
    Strict,
}