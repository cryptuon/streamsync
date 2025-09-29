//! Pattern-based reconstruction for known compression patterns

use crate::{
    error::{ReconstructionError, ReconstructionResult},
    types::TruncatedData,
    cache::ReconstructionCache,
};

use std::sync::Arc;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternMatchResult {
    pub data: Vec<u8>,
    pub pattern_id: String,
    pub confidence: f64,
}

/// Fast pattern-based reconstruction for known compression patterns
pub struct PatternMatcher {
    cache: Arc<ReconstructionCache>,
    known_patterns: HashMap<String, CompressionPattern>,
    pattern_index: PatternIndex,
}

#[derive(Debug, Clone)]
struct CompressionPattern {
    id: String,
    pattern_bytes: Vec<u8>,
    reconstruction_template: ReconstructionTemplate,
    success_rate: f64,
}

#[derive(Debug, Clone)]
struct ReconstructionTemplate {
    fixed_parts: Vec<(usize, Vec<u8>)>, // (offset, data)
    variable_parts: Vec<(usize, usize)>, // (offset, length)
    total_size: usize,
}

struct PatternIndex {
    // Fast lookup index for pattern matching
    // Maps pattern signatures to pattern IDs
    signature_to_pattern: HashMap<u64, Vec<String>>,
}

impl PatternMatcher {
    pub fn new(cache: Arc<ReconstructionCache>) -> Self {
        Self {
            cache,
            known_patterns: HashMap::new(),
            pattern_index: PatternIndex {
                signature_to_pattern: HashMap::new(),
            },
        }
    }

    /// Check if we have a known pattern for this data
    pub fn has_pattern_for_data(&self, truncated_data: &TruncatedData) -> bool {
        let signature = self.compute_pattern_signature(&truncated_data.data);
        if let Some(pattern_ids) = self.pattern_index.signature_to_pattern.get(&signature) {
            // Also check for fuzzy matches with similar patterns
            return !pattern_ids.is_empty() || self.has_fuzzy_pattern_match(truncated_data);
        }
        self.has_fuzzy_pattern_match(truncated_data)
    }

    /// Check for fuzzy pattern matches using sliding window
    fn has_fuzzy_pattern_match(&self, truncated_data: &TruncatedData) -> bool {
        let data = &truncated_data.data;
        if data.len() < 32 { return false; }

        // Check patterns with sliding window approach
        for window_size in [32, 64, 128] {
            if data.len() >= window_size {
                for start in 0..=data.len() - window_size {
                    let window = &data[start..start + window_size];
                    let window_sig = self.compute_pattern_signature(window);

                    if self.pattern_index.signature_to_pattern.contains_key(&window_sig) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Try fast pattern matching (sub-millisecond)
    pub async fn try_fast_pattern_match(
        &self,
        truncated_data: &TruncatedData,
    ) -> ReconstructionResult<PatternMatchResult> {

        let signature = self.compute_pattern_signature(&truncated_data.data);

        // First try exact pattern match
        if let Some(pattern_ids) = self.pattern_index.signature_to_pattern.get(&signature) {
            // Sort patterns by success rate (highest first)
            let mut sorted_patterns: Vec<_> = pattern_ids.iter()
                .filter_map(|id| self.known_patterns.get(id))
                .collect();
            sorted_patterns.sort_by(|a, b| b.success_rate.partial_cmp(&a.success_rate).unwrap());

            for pattern in sorted_patterns {
                if let Ok(result) = self.apply_pattern(pattern, truncated_data) {
                    // Update pattern success statistics
                    self.update_pattern_statistics(&pattern.id, true);
                    return Ok(result);
                }
            }
        }

        // Try fuzzy pattern matching if exact match fails
        self.try_fuzzy_pattern_match_fast(truncated_data).await
    }

    /// Fast fuzzy pattern matching using efficient algorithms
    async fn try_fuzzy_pattern_match_fast(
        &self,
        truncated_data: &TruncatedData,
    ) -> ReconstructionResult<PatternMatchResult> {

        let data = &truncated_data.data;
        let mut best_match = None;
        let mut best_confidence = 0.0;

        // Use sliding window with multiple scales
        for window_size in [32, 64, 128, 256] {
            if data.len() < window_size { continue; }

            for start in (0..=data.len() - window_size).step_by(window_size / 4) {
                let window = &data[start..start + window_size];
                let window_sig = self.compute_pattern_signature(window);

                if let Some(pattern_ids) = self.pattern_index.signature_to_pattern.get(&window_sig) {
                    for pattern_id in pattern_ids {
                        if let Some(pattern) = self.known_patterns.get(pattern_id) {
                            // Calculate pattern similarity using advanced metrics
                            let similarity = self.calculate_advanced_similarity(&data, &pattern.pattern_bytes);

                            if similarity > 0.7 && similarity > best_confidence {
                                if let Ok(result) = self.apply_pattern_with_adaptation(pattern, truncated_data) {
                                    best_match = Some(result);
                                    best_confidence = similarity;
                                }
                            }
                        }
                    }
                }
            }
        }

        best_match.ok_or(ReconstructionError::pattern_matching_failed())
    }

    /// Update pattern success statistics
    fn update_pattern_statistics(&self, pattern_id: &str, success: bool) {
        // In a real implementation, this would update pattern success rates
        // and potentially adjust the pattern ranking
    }

    /// Full pattern-based reconstruction
    pub async fn reconstruct_from_pattern(
        &self,
        truncated_data: &TruncatedData,
    ) -> ReconstructionResult<PatternMatchResult> {

        // First try fast pattern match
        if let Ok(result) = self.try_fast_pattern_match(truncated_data).await {
            return Ok(result);
        }

        // If no exact pattern match, try fuzzy matching
        self.try_fuzzy_pattern_match(truncated_data).await
    }

    /// Try fuzzy pattern matching for similar patterns
    async fn try_fuzzy_pattern_match(
        &self,
        truncated_data: &TruncatedData,
    ) -> ReconstructionResult<PatternMatchResult> {

        let mut best_match = None;
        let mut best_confidence = 0.0;

        for pattern in self.known_patterns.values() {
            let similarity = self.calculate_pattern_similarity(
                &truncated_data.data,
                &pattern.pattern_bytes
            );

            if similarity > 0.8 && similarity > best_confidence {
                if let Ok(result) = self.apply_pattern_with_adaptation(pattern, truncated_data) {
                    best_match = Some(result);
                    best_confidence = similarity;
                }
            }
        }

        best_match.ok_or(ReconstructionError::pattern_matching_failed())
    }

    /// Apply a known pattern to reconstruct data
    fn apply_pattern(
        &self,
        pattern: &CompressionPattern,
        truncated_data: &TruncatedData,
    ) -> ReconstructionResult<PatternMatchResult> {

        let template = &pattern.reconstruction_template;
        let mut reconstructed = vec![0u8; template.total_size];

        // Copy fixed parts
        for (offset, data) in &template.fixed_parts {
            if *offset + data.len() <= reconstructed.len() {
                reconstructed[*offset..*offset + data.len()].copy_from_slice(data);
            }
        }

        // Fill variable parts from truncated data
        let mut truncated_offset = 0;
        for (offset, length) in &template.variable_parts {
            if truncated_offset + length <= truncated_data.data.len() &&
               *offset + length <= reconstructed.len() {

                reconstructed[*offset..*offset + length].copy_from_slice(
                    &truncated_data.data[truncated_offset..truncated_offset + length]
                );
                truncated_offset += length;
            }
        }

        Ok(PatternMatchResult {
            data: reconstructed,
            pattern_id: pattern.id.clone(),
            confidence: pattern.success_rate,
        })
    }

    /// Apply pattern with adaptation for fuzzy matches
    fn apply_pattern_with_adaptation(
        &self,
        pattern: &CompressionPattern,
        truncated_data: &TruncatedData,
    ) -> ReconstructionResult<PatternMatchResult> {

        // For now, use the same logic as exact pattern application
        // In the future, this could include:
        // - Size adjustments
        // - Offset corrections
        // - Partial pattern matching

        let mut result = self.apply_pattern(pattern, truncated_data)?;

        // Reduce confidence for adapted patterns
        result.confidence *= 0.8;

        Ok(result)
    }

    /// Compute a fast signature for pattern matching
    fn compute_pattern_signature(&self, data: &[u8]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // Hash first 256 bytes for pattern signature
        let sample_size = data.len().min(256);
        data[..sample_size].hash(&mut hasher);

        hasher.finish()
    }

    /// Calculate similarity between data and known pattern
    fn calculate_pattern_similarity(&self, data1: &[u8], data2: &[u8]) -> f64 {
        self.calculate_advanced_similarity(data1, data2)
    }

    /// Advanced similarity calculation using multiple metrics
    fn calculate_advanced_similarity(&self, data1: &[u8], data2: &[u8]) -> f64 {
        if data1.is_empty() || data2.is_empty() {
            return 0.0;
        }

        // Combine multiple similarity metrics
        let byte_similarity = self.calculate_byte_similarity(data1, data2);
        let structure_similarity = self.calculate_structure_similarity(data1, data2);
        let pattern_similarity = self.calculate_pattern_similarity_advanced(data1, data2);

        // Weighted combination
        (byte_similarity * 0.4) + (structure_similarity * 0.3) + (pattern_similarity * 0.3)
    }

    /// Direct byte-by-byte similarity
    fn calculate_byte_similarity(&self, data1: &[u8], data2: &[u8]) -> f64 {
        let max_len = data1.len().max(data2.len());
        let min_len = data1.len().min(data2.len());

        if max_len == 0 { return 1.0; }

        let mut matches = 0;
        for i in 0..min_len {
            if data1[i] == data2[i] {
                matches += 1;
            }
        }

        // Penalize for length differences
        let length_penalty = (max_len - min_len) as f64 / max_len as f64;
        let base_similarity = matches as f64 / max_len as f64;

        base_similarity * (1.0 - length_penalty * 0.5)
    }

    /// Structural similarity based on data layout patterns
    fn calculate_structure_similarity(&self, data1: &[u8], data2: &[u8]) -> f64 {
        // Check for similar patterns in data structure
        let sig1 = self.extract_structural_signature(data1);
        let sig2 = self.extract_structural_signature(data2);

        let common_features = sig1.iter()
            .zip(sig2.iter())
            .map(|(a, b)| if (a - b).abs() < 0.1 { 1.0 } else { 0.0 })
            .sum::<f64>();

        common_features / sig1.len() as f64
    }

    /// Extract structural signature from data
    fn extract_structural_signature(&self, data: &[u8]) -> Vec<f64> {
        let mut signature = Vec::new();

        // Byte frequency distribution
        let mut freq = [0u32; 256];
        for &byte in data {
            freq[byte as usize] += 1;
        }

        // Normalize frequencies
        let total = data.len() as f64;
        for count in freq.iter().take(16) { // Use first 16 for efficiency
            signature.push(*count as f64 / total);
        }

        // Entropy measure
        let entropy = freq.iter()
            .map(|&count| {
                if count > 0 {
                    let p = count as f64 / total;
                    -p * p.log2()
                } else {
                    0.0
                }
            })
            .sum::<f64>();
        signature.push(entropy / 8.0); // Normalize

        // Zero-run lengths
        let mut zero_runs = 0u32;
        let mut in_zero_run = false;
        for &byte in data {
            if byte == 0 {
                if !in_zero_run {
                    zero_runs += 1;
                    in_zero_run = true;
                }
            } else {
                in_zero_run = false;
            }
        }
        signature.push(zero_runs as f64 / (data.len() as f64 / 16.0));

        signature
    }

    /// Advanced pattern similarity using subsequence matching
    fn calculate_pattern_similarity_advanced(&self, data1: &[u8], data2: &[u8]) -> f64 {
        // Use longest common subsequence approach for pattern detection
        let lcs_length = self.longest_common_subsequence(data1, data2);
        let max_len = data1.len().max(data2.len());

        if max_len == 0 { return 1.0; }
        lcs_length as f64 / max_len as f64
    }

    /// Calculate longest common subsequence length
    fn longest_common_subsequence(&self, data1: &[u8], data2: &[u8]) -> usize {
        let (m, n) = (data1.len(), data2.len());
        if m == 0 || n == 0 { return 0; }

        // For performance, limit to reasonable sizes
        let (m, n) = (m.min(256), n.min(256));
        let data1 = &data1[..m];
        let data2 = &data2[..n];

        let mut dp = vec![vec![0; n + 1]; m + 1];

        for i in 1..=m {
            for j in 1..=n {
                if data1[i - 1] == data2[j - 1] {
                    dp[i][j] = dp[i - 1][j - 1] + 1;
                } else {
                    dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
                }
            }
        }

        dp[m][n]
    }

    /// Learn new patterns from successful reconstructions
    pub async fn learn_pattern(
        &mut self,
        truncated_data: &TruncatedData,
        reconstructed_data: &[u8],
        success_confidence: f64,
    ) -> ReconstructionResult<()> {

        if success_confidence < 0.9 {
            // Only learn from high-confidence reconstructions
            return Ok(());
        }

        let pattern_id = format!(
            "learned_{}_{}_{}",
            truncated_data.metadata.program_id,
            truncated_data.data.len(),
            blake3::hash(&truncated_data.data).to_hex()[..8].to_string()
        );

        let pattern = CompressionPattern {
            id: pattern_id.clone(),
            pattern_bytes: truncated_data.data.clone(),
            reconstruction_template: self.create_template(
                &truncated_data.data,
                reconstructed_data
            ),
            success_rate: success_confidence,
        };

        // Add to known patterns
        self.known_patterns.insert(pattern_id.clone(), pattern);

        // Update pattern index
        let signature = self.compute_pattern_signature(&truncated_data.data);
        self.pattern_index.signature_to_pattern
            .entry(signature)
            .or_insert_with(Vec::new)
            .push(pattern_id);

        Ok(())
    }

    /// Create a reconstruction template from example data
    fn create_template(
        &self,
        truncated: &[u8],
        reconstructed: &[u8],
    ) -> ReconstructionTemplate {

        // Simple template creation - more sophisticated analysis could be added
        ReconstructionTemplate {
            fixed_parts: vec![], // Would analyze for fixed patterns
            variable_parts: vec![(0, truncated.len())], // Simple: all truncated data is variable
            total_size: reconstructed.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::ReconstructionCache;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_pattern_matcher_creation() {
        let cache = Arc::new(ReconstructionCache::new(1000));
        let matcher = PatternMatcher::new(cache);

        // Test basic functionality
        assert_eq!(matcher.known_patterns.len(), 0);
    }

    #[test]
    fn test_pattern_signature() {
        let cache = Arc::new(ReconstructionCache::new(1000));
        let matcher = PatternMatcher::new(cache);

        let data1 = vec![1, 2, 3, 4, 5];
        let data2 = vec![1, 2, 3, 4, 5];
        let data3 = vec![1, 2, 3, 4, 6];

        let sig1 = matcher.compute_pattern_signature(&data1);
        let sig2 = matcher.compute_pattern_signature(&data2);
        let sig3 = matcher.compute_pattern_signature(&data3);

        assert_eq!(sig1, sig2);
        assert_ne!(sig1, sig3);
    }
}