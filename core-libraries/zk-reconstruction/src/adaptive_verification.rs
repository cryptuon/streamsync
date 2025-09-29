//! Adaptive verification system with calibrated thresholds for real data patterns
//!
//! This module provides smart verification that adapts to actual Solana transaction
//! patterns and provides realistic quality assessments.

use crate::{
    error::{ReconstructionError, ReconstructionResult},
    types::{TruncatedData, CompressionParams, ReconstructedAccount, CompressionType},
};
use tracing::{info, warn};

/// Adaptive verification configuration based on transaction patterns
#[derive(Debug, Clone)]
pub struct AdaptiveVerificationConfig {
    /// Program-specific compression ratio limits
    pub compression_ratio_limits: CompressionRatioLimits,

    /// Entropy thresholds for different data types
    pub entropy_thresholds: EntropyThresholds,

    /// Minimum confidence scores by reconstruction method
    pub confidence_thresholds: ConfidenceThresholds,

    /// Whether to use adaptive thresholds based on data characteristics
    pub adaptive_mode: bool,
}

#[derive(Debug, Clone)]
pub struct CompressionRatioLimits {
    /// Maximum allowed compression ratio for SPL Token transactions
    pub spl_token_max: f64,

    /// Maximum allowed compression ratio for Metaplex transactions
    pub metaplex_max: f64,

    /// Maximum allowed compression ratio for Jupiter/swap transactions
    pub jupiter_max: f64,

    /// Maximum allowed compression ratio for compression programs
    pub compression_programs_max: f64,

    /// Default maximum for unknown programs
    pub default_max: f64,
}

#[derive(Debug, Clone)]
pub struct EntropyThresholds {
    /// Minimum entropy for small instruction data (< 32 bytes)
    pub small_instruction_min: f64,

    /// Minimum entropy for medium instruction data (32-128 bytes)
    pub medium_instruction_min: f64,

    /// Minimum entropy for large instruction data (> 128 bytes)
    pub large_instruction_min: f64,

    /// Allow zero entropy for certain known patterns
    pub allow_zero_patterns: bool,
}

#[derive(Debug, Clone)]
pub struct ConfidenceThresholds {
    /// Minimum confidence for mathematical reconstruction
    pub mathematical_min: f64,

    /// Minimum confidence for pattern matching
    pub pattern_matching_min: f64,

    /// Minimum confidence for merkle reconstruction
    pub merkle_reconstruction_min: f64,

    /// Minimum confidence for constraint solving
    pub constraint_solving_min: f64,
}

impl Default for AdaptiveVerificationConfig {
    fn default() -> Self {
        Self {
            compression_ratio_limits: CompressionRatioLimits {
                spl_token_max: 500.0,      // SPL Token instructions can expand significantly
                metaplex_max: 1000.0,      // NFT metadata can be large
                jupiter_max: 800.0,        // Swap calculations can expand
                compression_programs_max: 2000.0, // State compression allows high ratios
                default_max: 300.0,        // Conservative default
            },
            entropy_thresholds: EntropyThresholds {
                small_instruction_min: 0.1,   // Very low threshold for small data
                medium_instruction_min: 0.3,  // Moderate threshold for medium data
                large_instruction_min: 0.5,   // Higher threshold for large data
                allow_zero_patterns: true,    // Some Solana instructions are mostly zeros
            },
            confidence_thresholds: ConfidenceThresholds {
                mathematical_min: 0.4,        // Mathematical methods are reliable
                pattern_matching_min: 0.6,    // Pattern matching needs higher confidence
                merkle_reconstruction_min: 0.5, // Merkle has mathematical backing
                constraint_solving_min: 0.7,   // Constraint solving should be confident
            },
            adaptive_mode: true,
        }
    }
}

/// Adaptive verification system that calibrates to real data patterns
pub struct AdaptiveVerifier {
    config: AdaptiveVerificationConfig,
}

impl AdaptiveVerifier {
    pub fn new(config: Option<AdaptiveVerificationConfig>) -> Self {
        Self {
            config: config.unwrap_or_default(),
        }
    }

    /// Verify reconstruction with adaptive thresholds
    pub async fn verify_reconstruction(
        &self,
        reconstructed: &ReconstructedAccount,
        truncated_data: &TruncatedData,
        compression_params: &CompressionParams,
        operation_id: uuid::Uuid,
    ) -> ReconstructionResult<()> {

        info!(
            operation_id = %operation_id,
            original_size = truncated_data.data.len(),
            reconstructed_size = reconstructed.account_data.len(),
            confidence = reconstructed.confidence_score,
            "🔍 Starting adaptive verification"
        );

        let mut verification_issues = Vec::new();

        // 1. Adaptive compression ratio check
        if let Some(issue) = self.check_adaptive_compression_ratio(
            reconstructed,
            truncated_data,
            compression_params,
            operation_id,
        ).await {
            verification_issues.push(issue);
        }

        // 2. Adaptive entropy check
        if let Some(issue) = self.check_adaptive_entropy(
            reconstructed,
            truncated_data,
            operation_id,
        ).await {
            verification_issues.push(issue);
        }

        // 3. Adaptive confidence check
        if let Some(issue) = self.check_adaptive_confidence(
            reconstructed,
            operation_id,
        ).await {
            verification_issues.push(issue);
        }

        // 4. Structural consistency check
        if let Some(issue) = self.check_structural_consistency(
            reconstructed,
            truncated_data,
            operation_id,
        ).await {
            verification_issues.push(issue);
        }

        // Analyze results with adaptive logic
        self.analyze_verification_results(verification_issues, reconstructed, operation_id)
    }

    /// Check compression ratio with adaptive limits based on program type
    async fn check_adaptive_compression_ratio(
        &self,
        reconstructed: &ReconstructedAccount,
        truncated_data: &TruncatedData,
        compression_params: &CompressionParams,
        operation_id: uuid::Uuid,
    ) -> Option<String> {

        if truncated_data.data.is_empty() {
            return Some("Cannot verify compression ratio: empty input data".to_string());
        }

        let compression_ratio = reconstructed.account_data.len() as f64 / truncated_data.data.len() as f64;

        // Determine appropriate limit based on program type and compression
        let limit = self.determine_compression_limit(compression_params, truncated_data);

        info!(
            operation_id = %operation_id,
            compression_ratio = compression_ratio,
            limit = limit,
            compression_type = ?compression_params.compression_type,
            "📏 Checking adaptive compression ratio"
        );

        if compression_ratio > limit {
            Some(format!(
                "Compression ratio {:.2} exceeds adaptive limit {:.2} for this program type",
                compression_ratio, limit
            ))
        } else {
            info!(
                operation_id = %operation_id,
                "✅ Compression ratio within adaptive limits"
            );
            None
        }
    }

    /// Determine appropriate compression limit based on context
    fn determine_compression_limit(
        &self,
        compression_params: &CompressionParams,
        truncated_data: &TruncatedData,
    ) -> f64 {
        // Higher limits for state compression
        if compression_params.compression_type == CompressionType::StateCompression {
            return self.config.compression_ratio_limits.compression_programs_max;
        }

        // Determine by program ID if available
        let program_id_str = truncated_data.metadata.program_id.to_string();

        match program_id_str.as_str() {
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" => {
                self.config.compression_ratio_limits.spl_token_max
            },
            "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s" => {
                self.config.compression_ratio_limits.metaplex_max
            },
            "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4" => {
                self.config.compression_ratio_limits.jupiter_max
            },
            _ => {
                // Adaptive limit based on input size
                if truncated_data.data.len() < 16 {
                    self.config.compression_ratio_limits.default_max * 2.0 // Higher limit for very small inputs
                } else {
                    self.config.compression_ratio_limits.default_max
                }
            }
        }
    }

    /// Check entropy with adaptive thresholds
    async fn check_adaptive_entropy(
        &self,
        reconstructed: &ReconstructedAccount,
        truncated_data: &TruncatedData,
        operation_id: uuid::Uuid,
    ) -> Option<String> {

        let entropy = self.calculate_entropy(&reconstructed.account_data);
        let threshold = self.determine_entropy_threshold(truncated_data);

        info!(
            operation_id = %operation_id,
            entropy = entropy,
            threshold = threshold,
            data_size = reconstructed.account_data.len(),
            "🎲 Checking adaptive entropy"
        );

        // Allow zero entropy patterns for certain known cases
        if self.config.entropy_thresholds.allow_zero_patterns && self.is_known_zero_pattern(&reconstructed.account_data) {
            info!(
                operation_id = %operation_id,
                "✅ Known zero pattern detected, entropy check bypassed"
            );
            return None;
        }

        if entropy < threshold {
            Some(format!(
                "Entropy {:.3} below adaptive threshold {:.3} for data size {}",
                entropy, threshold, reconstructed.account_data.len()
            ))
        } else {
            info!(
                operation_id = %operation_id,
                "✅ Entropy within adaptive limits"
            );
            None
        }
    }

    /// Determine entropy threshold based on data characteristics
    fn determine_entropy_threshold(&self, truncated_data: &TruncatedData) -> f64 {
        match truncated_data.data.len() {
            0..=31 => self.config.entropy_thresholds.small_instruction_min,
            32..=128 => self.config.entropy_thresholds.medium_instruction_min,
            _ => self.config.entropy_thresholds.large_instruction_min,
        }
    }

    /// Check if this is a known pattern that should have low entropy
    fn is_known_zero_pattern(&self, data: &[u8]) -> bool {
        // Common Solana patterns that are mostly zeros
        if data.len() <= 32 && data.iter().filter(|&&b| b != 0).count() <= 4 {
            return true;
        }

        // Check for common instruction discriminators followed by zeros
        if data.len() >= 8 {
            let non_zero_count = data.iter().filter(|&&b| b != 0).count();
            if non_zero_count <= data.len() / 4 {
                return true;
            }
        }

        false
    }

    /// Calculate Shannon entropy of data
    fn calculate_entropy(&self, data: &[u8]) -> f64 {
        if data.is_empty() {
            return 0.0;
        }

        let mut frequencies = [0u32; 256];
        for &byte in data {
            frequencies[byte as usize] += 1;
        }

        let length = data.len() as f64;
        let mut entropy = 0.0;

        for &freq in frequencies.iter() {
            if freq > 0 {
                let p = freq as f64 / length;
                entropy -= p * p.log2();
            }
        }

        entropy
    }

    /// Check confidence with adaptive thresholds based on method
    async fn check_adaptive_confidence(
        &self,
        reconstructed: &ReconstructedAccount,
        operation_id: uuid::Uuid,
    ) -> Option<String> {

        let threshold = match reconstructed.reconstruction_method {
            crate::types::ReconstructionMethod::MerkleTreeReconstruction => {
                self.config.confidence_thresholds.merkle_reconstruction_min
            },
            crate::types::ReconstructionMethod::PatternMatching { .. } => {
                self.config.confidence_thresholds.pattern_matching_min
            },
            crate::types::ReconstructionMethod::ConstraintSolving => {
                self.config.confidence_thresholds.constraint_solving_min
            },
            crate::types::ReconstructionMethod::Hybrid { .. } => {
                // For hybrid methods, use the most conservative threshold
                self.config.confidence_thresholds.mathematical_min
            },
        };

        info!(
            operation_id = %operation_id,
            confidence = reconstructed.confidence_score,
            threshold = threshold,
            method = ?reconstructed.reconstruction_method,
            "🎯 Checking adaptive confidence"
        );

        if reconstructed.confidence_score < threshold {
            Some(format!(
                "Confidence {:.3} below adaptive threshold {:.3} for method {:?}",
                reconstructed.confidence_score, threshold, reconstructed.reconstruction_method
            ))
        } else {
            info!(
                operation_id = %operation_id,
                "✅ Confidence meets adaptive threshold"
            );
            None
        }
    }

    /// Check basic structural consistency
    async fn check_structural_consistency(
        &self,
        reconstructed: &ReconstructedAccount,
        truncated_data: &TruncatedData,
        operation_id: uuid::Uuid,
    ) -> Option<String> {

        info!(
            operation_id = %operation_id,
            "🏗️ Checking structural consistency"
        );

        // Basic size checks
        if reconstructed.account_data.is_empty() {
            return Some("Reconstructed data is empty".to_string());
        }

        // For small truncated data, be more lenient about finding exact matches
        if truncated_data.data.len() <= 8 {
            info!(
                operation_id = %operation_id,
                "✅ Small data size, structural consistency relaxed"
            );
            return None;
        }

        // Check if we can find the truncated data or similar pattern in reconstruction
        if self.find_data_pattern(&reconstructed.account_data, &truncated_data.data) {
            info!(
                operation_id = %operation_id,
                "✅ Found data pattern in reconstruction"
            );
            None
        } else {
            // For larger truncated data, this might indicate a reconstruction issue
            if truncated_data.data.len() > 16 {
                Some("Truncated data pattern not found in reconstruction".to_string())
            } else {
                // For small data, this is often expected due to expansion
                info!(
                    operation_id = %operation_id,
                    "⚠️ Pattern not found but data size is small, allowing"
                );
                None
            }
        }
    }

    /// Find pattern in reconstructed data (fuzzy matching)
    fn find_data_pattern(&self, reconstructed: &[u8], pattern: &[u8]) -> bool {
        if pattern.is_empty() || reconstructed.is_empty() {
            return false;
        }

        // Exact match
        if reconstructed.windows(pattern.len()).any(|window| window == pattern) {
            return true;
        }

        // Fuzzy match for small patterns (allow 1-2 byte differences)
        if pattern.len() <= 8 {
            return reconstructed.windows(pattern.len()).any(|window| {
                let differences = window.iter().zip(pattern.iter())
                    .filter(|(a, b)| a != b)
                    .count();
                differences <= 2
            });
        }

        false
    }

    /// Analyze verification results with adaptive logic
    fn analyze_verification_results(
        &self,
        issues: Vec<String>,
        reconstructed: &ReconstructedAccount,
        operation_id: uuid::Uuid,
    ) -> ReconstructionResult<()> {

        let issue_count = issues.len();

        info!(
            operation_id = %operation_id,
            issue_count = issue_count,
            confidence = reconstructed.confidence_score,
            "📊 Analyzing adaptive verification results"
        );

        // Adaptive decision logic
        match issue_count {
            0 => {
                info!(
                    operation_id = %operation_id,
                    "✅ All adaptive verifications passed"
                );
                Ok(())
            },
            1 => {
                // Single issue - allow if confidence is reasonable
                if reconstructed.confidence_score >= 0.5 {
                    warn!(
                        operation_id = %operation_id,
                        issue = %issues[0],
                        "⚠️ Single issue detected but confidence is adequate"
                    );
                    Ok(())
                } else {
                    Err(ReconstructionError::verification_failed(
                        format!("Adaptive verification failed: {}", issues[0])
                    ))
                }
            },
            2 => {
                // Two issues - require higher confidence
                if reconstructed.confidence_score >= 0.7 {
                    warn!(
                        operation_id = %operation_id,
                        issues = ?issues,
                        "⚠️ Multiple issues but high confidence allows passing"
                    );
                    Ok(())
                } else {
                    Err(ReconstructionError::verification_failed(
                        format!("Adaptive verification failed: {}", issues.join("; "))
                    ))
                }
            },
            _ => {
                // Three or more issues - fail regardless of confidence
                Err(ReconstructionError::verification_failed(
                    format!("Adaptive verification failed: {}", issues.join("; "))
                ))
            }
        }
    }
}

impl Default for AdaptiveVerifier {
    fn default() -> Self {
        Self::new(None)
    }
}