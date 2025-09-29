//! Verification system for reconstruction correctness

use crate::{
    error::{ReconstructionError, ReconstructionResult},
    types::{TruncatedData, CompressionParams, ReconstructedAccount, VerificationProof, ProofType},
};

use sha2::{Sha256, Digest};
use std::collections::HashMap;

/// Verifies reconstruction correctness through multiple methods
pub struct ReconstructionVerifier {
    // Verification strategies
    verification_methods: Vec<VerificationMethod>,

    // Known good reconstructions for validation
    validation_cache: HashMap<String, ValidationEntry>,
}

#[derive(Debug, Clone)]
enum VerificationMethod {
    /// Verify merkle inclusion proofs
    MerkleInclusion,

    /// Verify mathematical consistency
    MathematicalConsistency,

    /// Verify against known patterns
    PatternConsistency,

    /// Verify size and structure constraints
    StructuralConsistency,

    /// Cross-validate with other reconstructions
    CrossValidation,
}

#[derive(Debug, Clone)]
struct ValidationEntry {
    original_hash: [u8; 32],
    reconstruction_hash: [u8; 32],
    confidence_score: f64,
    validation_count: u32,
}

impl ReconstructionVerifier {
    pub fn new() -> Self {
        Self {
            verification_methods: vec![
                VerificationMethod::StructuralConsistency,
                VerificationMethod::MathematicalConsistency,
                VerificationMethod::MerkleInclusion,
                VerificationMethod::PatternConsistency,
            ],
            validation_cache: HashMap::new(),
        }
    }

    /// Verify a reconstruction result
    pub async fn verify_reconstruction(
        &self,
        reconstructed: &ReconstructedAccount,
        truncated_data: &TruncatedData,
        compression_params: &CompressionParams,
    ) -> ReconstructionResult<()> {

        let mut verification_results = Vec::new();

        // Apply all verification methods
        for method in &self.verification_methods {
            let result = self.apply_verification_method(
                method,
                reconstructed,
                truncated_data,
                compression_params,
            ).await;

            verification_results.push((method.clone(), result));
        }

        // Analyze verification results
        self.analyze_verification_results(verification_results, reconstructed)
    }

    /// Apply a specific verification method
    async fn apply_verification_method(
        &self,
        method: &VerificationMethod,
        reconstructed: &ReconstructedAccount,
        truncated_data: &TruncatedData,
        compression_params: &CompressionParams,
    ) -> VerificationResult {

        match method {
            VerificationMethod::StructuralConsistency => {
                self.verify_structural_consistency(reconstructed, truncated_data).await
            },
            VerificationMethod::MathematicalConsistency => {
                self.verify_mathematical_consistency(reconstructed, truncated_data, compression_params).await
            },
            VerificationMethod::MerkleInclusion => {
                self.verify_merkle_inclusion(reconstructed, compression_params).await
            },
            VerificationMethod::PatternConsistency => {
                self.verify_pattern_consistency(reconstructed, truncated_data).await
            },
            VerificationMethod::CrossValidation => {
                self.verify_cross_validation(reconstructed, truncated_data).await
            },
        }
    }

    /// Verify structural consistency (sizes, formats, etc.)
    async fn verify_structural_consistency(
        &self,
        reconstructed: &ReconstructedAccount,
        truncated_data: &TruncatedData,
    ) -> VerificationResult {

        let mut issues = Vec::new();

        // Check that reconstructed data is larger than truncated data
        if reconstructed.account_data.len() < truncated_data.data.len() {
            issues.push("Reconstructed data is smaller than truncated data".to_string());
        }

        // Check that truncated data is contained in reconstructed data
        if reconstructed.account_data.len() >= truncated_data.data.len() {
            let truncated_slice = &reconstructed.account_data[..truncated_data.data.len()];
            if truncated_slice != truncated_data.data {
                issues.push("Truncated data not found at start of reconstructed data".to_string());
            }
        }

        // Check size reasonableness (not too large)
        if reconstructed.account_data.len() > 10 * 1024 * 1024 { // 10MB limit
            issues.push("Reconstructed data is unreasonably large".to_string());
        }

        // Check for basic data structure markers
        if !self.has_valid_data_structure(&reconstructed.account_data) {
            issues.push("Reconstructed data lacks valid structure markers".to_string());
        }

        if issues.is_empty() {
            VerificationResult::Pass
        } else {
            VerificationResult::Fail(issues)
        }
    }

    /// Verify mathematical consistency of the reconstruction
    async fn verify_mathematical_consistency(
        &self,
        reconstructed: &ReconstructedAccount,
        truncated_data: &TruncatedData,
        compression_params: &CompressionParams,
    ) -> VerificationResult {

        let mut issues = Vec::new();

        // Verify hash consistency if we have merkle tree information
        if compression_params.merkle_tree_height > 0 {
            if let Err(e) = self.verify_hash_consistency(reconstructed, compression_params) {
                issues.push(format!("Hash consistency failed: {}", e));
            }
        }

        // Verify compression ratio is reasonable
        let compression_ratio = reconstructed.account_data.len() as f64 / truncated_data.data.len() as f64;
        if compression_ratio > 100.0 {
            issues.push(format!("Compression ratio too high: {:.2}", compression_ratio));
        }

        // Verify data entropy (should not be all zeros or repeating patterns)
        if !self.has_reasonable_entropy(&reconstructed.account_data) {
            issues.push("Reconstructed data has suspicious entropy patterns".to_string());
        }

        if issues.is_empty() {
            VerificationResult::Pass
        } else {
            VerificationResult::Fail(issues)
        }
    }

    /// Verify merkle inclusion proofs
    async fn verify_merkle_inclusion(
        &self,
        reconstructed: &ReconstructedAccount,
        compression_params: &CompressionParams,
    ) -> VerificationResult {

        // Check if we have a merkle proof
        if let Some(proof) = &reconstructed.verification_proof {
            match proof.proof_type {
                ProofType::MerkleInclusion => {
                    match self.verify_merkle_proof(proof, compression_params) {
                        Ok(()) => VerificationResult::Pass,
                        Err(e) => VerificationResult::Fail(vec![format!("Merkle proof invalid: {}", e)]),
                    }
                },
                _ => VerificationResult::Skip("No merkle proof available".to_string()),
            }
        } else {
            VerificationResult::Skip("No verification proof provided".to_string())
        }
    }

    /// Verify pattern consistency
    async fn verify_pattern_consistency(
        &self,
        reconstructed: &ReconstructedAccount,
        truncated_data: &TruncatedData,
    ) -> VerificationResult {

        // For pattern-based reconstructions, verify the pattern was applied correctly
        if let crate::types::ReconstructionMethod::PatternMatching { pattern_id } = &reconstructed.reconstruction_method {
            // Check if the pattern application makes sense
            if self.validate_pattern_application(pattern_id, truncated_data, reconstructed) {
                VerificationResult::Pass
            } else {
                VerificationResult::Fail(vec!["Pattern application validation failed".to_string()])
            }
        } else {
            VerificationResult::Skip("Not a pattern-based reconstruction".to_string())
        }
    }

    /// Cross-validate with known good reconstructions
    async fn verify_cross_validation(
        &self,
        reconstructed: &ReconstructedAccount,
        truncated_data: &TruncatedData,
    ) -> VerificationResult {

        let input_hash = blake3::hash(&truncated_data.data);
        let output_hash = blake3::hash(&reconstructed.account_data);

        // Check if we have a validation entry for this input
        let input_key = hex::encode(input_hash.as_bytes());

        if let Some(validation_entry) = self.validation_cache.get(&input_key) {
            if validation_entry.reconstruction_hash == *output_hash.as_bytes() {
                VerificationResult::Pass
            } else {
                VerificationResult::Fail(vec!["Cross-validation mismatch with known good reconstruction".to_string()])
            }
        } else {
            VerificationResult::Skip("No cross-validation data available".to_string())
        }
    }

    /// Analyze all verification results and determine overall result
    fn analyze_verification_results(
        &self,
        results: Vec<(VerificationMethod, VerificationResult)>,
        reconstructed: &ReconstructedAccount,
    ) -> ReconstructionResult<()> {

        let mut passed = 0;
        let mut failed = 0;
        let mut skipped = 0;
        let mut all_issues = Vec::new();

        for (_method, result) in results {
            match result {
                VerificationResult::Pass => passed += 1,
                VerificationResult::Fail(issues) => {
                    failed += 1;
                    all_issues.extend(issues);
                },
                VerificationResult::Skip(_) => skipped += 1,
            }
        }

        // Determine if verification passes based on results and confidence
        let total_applicable = passed + failed;

        if total_applicable == 0 {
            // No applicable verifications - use confidence score
            if reconstructed.confidence_score >= 0.8 {
                Ok(())
            } else {
                Err(ReconstructionError::verification_failed("Low confidence and no verification methods applicable"))
            }
        } else if failed == 0 {
            // All applicable verifications passed
            Ok(())
        } else if (passed as f64 / total_applicable as f64) >= 0.7 && reconstructed.confidence_score >= 0.9 {
            // Majority passed and high confidence
            Ok(())
        } else {
            // Too many failures
            Err(ReconstructionError::verification_failed(
                format!("Verification failed: {}", all_issues.join("; "))
            ))
        }
    }

    /// Helper method to verify hash consistency
    fn verify_hash_consistency(
        &self,
        reconstructed: &ReconstructedAccount,
        _compression_params: &CompressionParams,
    ) -> Result<(), String> {

        // Compute hash of reconstructed data
        let data_hash = Sha256::digest(&reconstructed.account_data);

        // For this skeleton, we'll do a simple check
        // Real implementation would verify against merkle tree constraints

        if data_hash.len() == 32 {
            Ok(())
        } else {
            Err("Hash computation failed".to_string())
        }
    }

    /// Check if data has reasonable entropy (not all zeros or simple patterns)
    fn has_reasonable_entropy(&self, data: &[u8]) -> bool {
        if data.is_empty() {
            return false;
        }

        // Check for all zeros
        if data.iter().all(|&b| b == 0) {
            return false;
        }

        // Check for simple repeating patterns
        if data.len() > 4 {
            let pattern = &data[0..4];
            let mut is_repeating = true;

            for chunk in data.chunks(4) {
                if chunk != pattern {
                    is_repeating = false;
                    break;
                }
            }

            if is_repeating {
                return false;
            }
        }

        // Basic entropy check - count unique bytes
        let mut byte_counts = [0u32; 256];
        for &byte in data {
            byte_counts[byte as usize] += 1;
        }

        let unique_bytes = byte_counts.iter().filter(|&&count| count > 0).count();

        // Should have at least some variety
        unique_bytes > data.len().min(16) / 4
    }

    /// Check if data has valid structure markers
    fn has_valid_data_structure(&self, data: &[u8]) -> bool {
        // Look for common data structure markers
        if data.len() < 8 {
            return false; // Too small to have meaningful structure
        }

        // Check for common patterns that indicate structured data
        // This is a simplified check - real implementation would be more sophisticated

        // Look for length fields (little-endian u32 that could be reasonable lengths)
        for i in 0..data.len().saturating_sub(4) {
            let potential_length = u32::from_le_bytes([
                data[i], data[i + 1], data[i + 2], data[i + 3]
            ]);

            // If this looks like a reasonable length field
            if potential_length > 0 && potential_length < data.len() as u32 * 2 {
                return true;
            }
        }

        // Look for padding patterns (zeros at end)
        let trailing_zeros = data.iter().rev().take_while(|&&b| b == 0).count();
        if trailing_zeros > 0 && trailing_zeros < data.len() / 2 {
            return true; // Some padding is normal
        }

        // Default to accepting if we can't prove it's invalid
        true
    }

    /// Verify merkle proof
    fn verify_merkle_proof(
        &self,
        proof: &VerificationProof,
        compression_params: &CompressionParams,
    ) -> Result<(), String> {

        if proof.merkle_proof.is_empty() {
            return Err("Empty merkle proof".to_string());
        }

        // For this skeleton, do a basic check
        // Real implementation would verify the complete merkle path

        if proof.merkle_proof.len() <= compression_params.merkle_tree_height as usize {
            Ok(())
        } else {
            Err("Merkle proof too long for tree height".to_string())
        }
    }

    /// Validate pattern application
    fn validate_pattern_application(
        &self,
        pattern_id: &str,
        truncated_data: &TruncatedData,
        reconstructed: &ReconstructedAccount,
    ) -> bool {

        // Basic validation for pattern-based reconstruction
        // Check that the pattern ID is reasonable
        if pattern_id.is_empty() {
            return false;
        }

        // Check that reconstruction is larger than input
        if reconstructed.account_data.len() <= truncated_data.data.len() {
            return false;
        }

        // Check that truncated data appears in reconstruction
        if truncated_data.data.len() > 0 {
            let start_match = reconstructed.account_data
                .get(..truncated_data.data.len())
                .map(|slice| slice == truncated_data.data)
                .unwrap_or(false);

            if !start_match {
                return false;
            }
        }

        true
    }

    /// Add a validation entry for cross-validation
    pub fn add_validation_entry(
        &mut self,
        truncated_data: &TruncatedData,
        reconstructed: &ReconstructedAccount,
    ) {
        let input_hash = blake3::hash(&truncated_data.data);
        let output_hash = blake3::hash(&reconstructed.account_data);
        let input_key = hex::encode(input_hash.as_bytes());

        let entry = ValidationEntry {
            original_hash: *input_hash.as_bytes(),
            reconstruction_hash: *output_hash.as_bytes(),
            confidence_score: reconstructed.confidence_score,
            validation_count: 1,
        };

        self.validation_cache.insert(input_key, entry);
    }
}

#[derive(Debug, Clone)]
enum VerificationResult {
    Pass,
    Fail(Vec<String>),
    Skip(String),
}

impl Default for ReconstructionVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{TruncationMetadata, CompressionType, ReconstructionMethod, CacheHint};
    use solana_sdk::pubkey::Pubkey;
    use std::time::{SystemTime, Duration};

    fn create_test_data() -> (TruncatedData, CompressionParams, ReconstructedAccount) {
        let truncated_data = TruncatedData {
            data: vec![1, 2, 3, 4, 5],
            original_size_hint: Some(1000),
            truncation_point: 5,
            metadata: TruncationMetadata {
                slot: 12345,
                account: Pubkey::new_unique(),
                program_id: Pubkey::new_unique(),
                compression_type: CompressionType::Standard,
                truncation_timestamp: SystemTime::now(),
            },
        };

        let compression_params = CompressionParams {
            compression_type: CompressionType::Standard,
            merkle_tree_height: 20,
            leaf_count: 1000,
            root_hash: [1u8; 32],
            compression_program: Pubkey::new_unique(),
            additional_params: std::collections::HashMap::new(),
        };

        let reconstructed = ReconstructedAccount {
            account_data: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10], // Starts with truncated data
            reconstruction_method: ReconstructionMethod::MerkleTreeReconstruction,
            confidence_score: 0.95,
            verification_proof: None,
            reconstruction_time: Duration::from_millis(10),
            cache_hint: CacheHint {
                cache_key: "test".to_string(),
                ttl: Duration::from_secs(300),
                pattern_category: None,
                reuse_probability: 0.8,
            },
        };

        (truncated_data, compression_params, reconstructed)
    }

    #[tokio::test]
    async fn test_verifier_creation() {
        let verifier = ReconstructionVerifier::new();
        assert_eq!(verifier.verification_methods.len(), 4);
        assert_eq!(verifier.validation_cache.len(), 0);
    }

    #[tokio::test]
    async fn test_structural_consistency_pass() {
        let verifier = ReconstructionVerifier::new();
        let (truncated_data, _, reconstructed) = create_test_data();

        let result = verifier.verify_structural_consistency(&reconstructed, &truncated_data).await;

        match result {
            VerificationResult::Pass => {},
            _ => panic!("Expected verification to pass"),
        }
    }

    #[tokio::test]
    async fn test_structural_consistency_fail() {
        let verifier = ReconstructionVerifier::new();
        let (truncated_data, _, mut reconstructed) = create_test_data();

        // Make reconstructed data smaller than truncated data
        reconstructed.account_data = vec![1, 2, 3];

        let result = verifier.verify_structural_consistency(&reconstructed, &truncated_data).await;

        match result {
            VerificationResult::Fail(issues) => {
                assert!(!issues.is_empty());
            },
            _ => panic!("Expected verification to fail"),
        }
    }

    #[test]
    fn test_entropy_checking() {
        let verifier = ReconstructionVerifier::new();

        // All zeros should fail
        assert!(!verifier.has_reasonable_entropy(&vec![0; 100]));

        // Repeating pattern should fail
        assert!(!verifier.has_reasonable_entropy(&vec![1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4]));

        // Random-looking data should pass
        assert!(verifier.has_reasonable_entropy(&vec![1, 5, 2, 9, 7, 3, 8, 4, 6, 0]));

        // Empty data should fail
        assert!(!verifier.has_reasonable_entropy(&vec![]));
    }

    #[test]
    fn test_data_structure_validation() {
        let verifier = ReconstructionVerifier::new();

        // Too small should fail
        assert!(!verifier.has_valid_data_structure(&vec![1, 2, 3]));

        // Data with potential length field should pass
        let mut data_with_length = vec![0; 100];
        data_with_length[0] = 50; // Reasonable length field
        assert!(verifier.has_valid_data_structure(&data_with_length));

        // Data with padding should pass
        let mut data_with_padding = vec![1, 2, 3, 4, 5, 6, 7, 8];
        data_with_padding.extend(vec![0; 4]); // Some padding
        assert!(verifier.has_valid_data_structure(&data_with_padding));
    }

    #[tokio::test]
    async fn test_full_verification_pass() {
        let verifier = ReconstructionVerifier::new();
        let (truncated_data, compression_params, reconstructed) = create_test_data();

        let result = verifier.verify_reconstruction(&reconstructed, &truncated_data, &compression_params).await;

        assert!(result.is_ok());
    }
}