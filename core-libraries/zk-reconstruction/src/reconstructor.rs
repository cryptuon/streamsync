//! # Reconstruction Orchestrator
//!
//! This module contains the main [`ZKReconstructionLibrary`] which orchestrates the entire
//! reconstruction process. It intelligently selects and combines multiple reconstruction
//! strategies to achieve the highest success rate and confidence.
//!
//! ## Key Components
//!
//! - **Strategy Selection**: Automatically chooses the best reconstruction approach
//! - **Multi-stage Pipeline**: Combines fast pattern matching with deeper analysis
//! - **Confidence Assessment**: Provides quality metrics for reconstructed data
//! - **Performance Optimization**: Uses caching and parallel processing where beneficial
//!
//! ## Reconstruction Flow
//!
//! 1. **Fast Path**: Try pattern matching for known compression patterns
//! 2. **Complexity Analysis**: Assess the difficulty of reconstruction
//! 3. **Strategy Selection**: Choose the most appropriate reconstruction method
//! 4. **Execution**: Run the selected strategy with fallbacks
//! 5. **Verification**: Validate the reconstructed data integrity
//! 6. **Caching**: Store successful patterns for future use

use crate::{
    error::{ReconstructionError, ReconstructionResult, ErrorContext},
    types::{
        CompressionParams, TruncatedData, ReconstructedAccount, ReconstructionConfig,
        ReconstructionStrategy, ComplexityEstimate, ReconstructionMethod, CacheHint
    },
    cache::ReconstructionCache,
    strategies::{
        pattern_matcher::PatternMatcher,
        merkle_reconstructor::MerkleReconstructor,
        constraint_solver::ConstraintSolver,
    },
    verification::ReconstructionVerifier,
    adaptive_verification::AdaptiveVerifier,
};

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{debug, info, warn, error};

/// Main ZK reconstruction library
pub struct ZKReconstructionLibrary {
    config: ReconstructionConfig,
    cache: Arc<ReconstructionCache>,
    pattern_matcher: PatternMatcher,
    merkle_reconstructor: MerkleReconstructor,
    constraint_solver: ConstraintSolver,
    verifier: ReconstructionVerifier,
    adaptive_verifier: AdaptiveVerifier,
}

impl ZKReconstructionLibrary {
    /// Create a new reconstruction library with default configuration
    pub fn new() -> Self {
        Self::with_config(ReconstructionConfig::default())
    }

    /// Create a new reconstruction library with custom configuration
    pub fn with_config(config: ReconstructionConfig) -> Self {
        let cache = Arc::new(ReconstructionCache::new(config.cache_size));

        Self {
            config,
            cache: cache.clone(),
            pattern_matcher: PatternMatcher::new(cache.clone()),
            merkle_reconstructor: MerkleReconstructor::new(),
            constraint_solver: ConstraintSolver::new(),
            verifier: ReconstructionVerifier::new(),
            adaptive_verifier: AdaptiveVerifier::default(),
        }
    }

    /// Check if the library is ready for reconstruction
    pub fn is_ready(&self) -> bool {
        // For now, always ready. In the future, this might check:
        // - Pattern cache initialization
        // - Network connectivity for consensus
        // - Required dependencies
        true
    }

    /// Reconstruct complete account state from truncated compression data
    pub async fn reconstruct_compressed_account(
        &self,
        truncated_data: &TruncatedData,
        compression_params: &CompressionParams,
    ) -> ReconstructionResult<ReconstructedAccount> {

        let start_time = Instant::now();
        let operation_id = uuid::Uuid::new_v4();

        info!(
            operation_id = %operation_id,
            account = %truncated_data.metadata.account,
            data_size = truncated_data.data.len(),
            compression_type = ?compression_params.compression_type,
            "🔄 Starting ZK reconstruction"
        );

        // Create error context for this operation
        let error_context = ErrorContext::new("reconstruct_compressed_account")
            .with_data_size(truncated_data.data.len())
            .with_metadata("operation_id", operation_id.to_string())
            .with_metadata("account", truncated_data.metadata.account.to_string())
            .with_metadata("compression_type", format!("{:?}", compression_params.compression_type));

        // 1. Validate input data
        if truncated_data.data.is_empty() {
            return Err(ReconstructionError::insufficient_data_with_context(
                error_context.with_metadata("stage", "input_validation")
            ));
        }

        if truncated_data.data.len() > self.config.max_input_size {
            return Err(ReconstructionError::invalid_params_with_context(
                format!("Input data size {} exceeds maximum {}",
                    truncated_data.data.len(),
                    self.config.max_input_size),
                error_context.with_metadata("stage", "input_validation")
            ));
        }

        // 2. Check cache for known reconstruction
        debug!(
            operation_id = %operation_id,
            cache_key = ?self.cache.compute_cache_key(truncated_data, compression_params),
            "🔍 Checking reconstruction cache"
        );

        if let Some(cached_result) = self.cache.get_reconstruction(truncated_data, compression_params).await {
            let duration = start_time.elapsed();
            info!(
                operation_id = %operation_id,
                duration_ms = duration.as_millis(),
                confidence = cached_result.confidence_score,
                result_size = cached_result.account_data.len(),
                "✅ Cache hit - reconstruction completed"
            );
            return Ok(cached_result);
        }

        // 3. Analyze complexity and select strategy
        debug!(operation_id = %operation_id, "📊 Analyzing reconstruction complexity");
        let complexity = self.estimate_complexity(truncated_data, compression_params);
        let strategy = self.select_strategy(complexity, truncated_data, compression_params);

        info!(
            operation_id = %operation_id,
            strategy = ?strategy,
            complexity_score = complexity.complexity_score(),
            estimated_time_ms = complexity.estimated_time_ms(),
            "🎯 Selected reconstruction strategy"
        );

        // 4. Execute reconstruction with timeout
        let strategy_context = error_context
            .with_strategy(format!("{:?}", strategy))
            .with_metadata("stage", "execution")
            .with_metadata("complexity_score", complexity.complexity_score().to_string());

        let reconstruction_future = self.execute_reconstruction_strategy(
            strategy,
            truncated_data,
            compression_params,
            operation_id
        );

        let mut reconstructed = timeout(
            self.config.max_reconstruction_time,
            reconstruction_future
        ).await
            .map_err(|_| {
                warn!(
                    operation_id = %operation_id,
                    timeout_ms = self.config.max_reconstruction_time.as_millis(),
                    "⏰ Reconstruction timeout"
                );
                ReconstructionError::timeout_with_context(
                    self.config.max_reconstruction_time.as_millis() as u64,
                    strategy_context.clone()
                )
            })?
            .map_err(|e| {
                error!(
                    operation_id = %operation_id,
                    error = %e,
                    "❌ Reconstruction execution failed"
                );
                e
            })?;

        // 5. Verify reconstruction with adaptive thresholds
        debug!(operation_id = %operation_id, "🔍 Verifying reconstruction with adaptive thresholds");
        self.adaptive_verifier.verify_reconstruction(
            &reconstructed,
            truncated_data,
            compression_params,
            operation_id,
        ).await
        .map_err(|e| {
            error!(
                operation_id = %operation_id,
                error = %e,
                "❌ Adaptive verification failed"
            );
            e
        })?;

        // 6. Update reconstruction metadata
        let total_duration = start_time.elapsed();
        reconstructed.reconstruction_time = total_duration;
        reconstructed.cache_hint = self.generate_cache_hint(&reconstructed, truncated_data);

        // 7. Cache successful reconstruction
        debug!(operation_id = %operation_id, "💾 Caching successful reconstruction");
        self.cache.store_reconstruction(
            truncated_data,
            compression_params,
            &reconstructed
        ).await;

        info!(
            operation_id = %operation_id,
            duration_ms = total_duration.as_millis(),
            result_size = reconstructed.account_data.len(),
            confidence = reconstructed.confidence_score,
            strategy = ?strategy,
            "✅ ZK reconstruction completed successfully"
        );

        Ok(reconstructed)
    }

    /// Fast path for common reconstruction patterns
    pub async fn fast_reconstruct_common_patterns(
        &self,
        truncated_data: &TruncatedData,
    ) -> Option<ReconstructedAccount> {

        // Try pattern matching first (fastest path)
        match self.pattern_matcher.try_fast_pattern_match(truncated_data).await {
            Ok(result) => {
                debug!("Fast pattern match successful");
                Some(ReconstructedAccount {
                    account_data: result.data,
                    confidence_score: result.confidence,
                    reconstruction_method: crate::types::ReconstructionMethod::PatternMatching {
                        pattern_id: result.pattern_id.clone()
                    },
                    reconstruction_time: std::time::Duration::from_millis(1), // Fast path
                    verification_proof: None,
                    cache_hint: crate::types::CacheHint {
                        cache_key: format!("pattern_{}", result.pattern_id),
                        ttl: std::time::Duration::from_secs(300),
                        pattern_category: Some(result.pattern_id),
                        reuse_probability: 0.9,
                    },
                })
            },
            Err(e) => {
                debug!("Fast pattern match failed: {}", e);
                None
            }
        }
    }

    /// Estimate complexity of reconstruction
    fn estimate_complexity(
        &self,
        truncated_data: &TruncatedData,
        compression_params: &CompressionParams,
    ) -> ComplexityEstimate {
        // Base complexity on data size
        let size_complexity = ComplexityEstimate::from_data_size(truncated_data.data.len());

        // Adjust based on compression type
        let type_complexity = match compression_params.compression_type {
            crate::types::CompressionType::Standard => ComplexityEstimate::Low,
            crate::types::CompressionType::StateCompression => ComplexityEstimate::Medium,
            crate::types::CompressionType::Custom(_) => ComplexityEstimate::High,
        };

        // Adjust based on merkle tree size
        let tree_complexity = if compression_params.merkle_tree_height > 25 {
            ComplexityEstimate::High
        } else if compression_params.merkle_tree_height > 20 {
            ComplexityEstimate::Medium
        } else {
            ComplexityEstimate::Low
        };

        // Return the maximum complexity
        size_complexity.max(type_complexity).max(tree_complexity)
    }

    /// Select the best reconstruction strategy
    fn select_strategy(
        &self,
        complexity: ComplexityEstimate,
        truncated_data: &TruncatedData,
        compression_params: &CompressionParams,
    ) -> ReconstructionStrategy {
        // Try pattern matching first for known patterns
        if self.pattern_matcher.has_pattern_for_data(truncated_data) {
            return ReconstructionStrategy::FastPattern;
        }

        // Select based on complexity and data characteristics
        match complexity {
            ComplexityEstimate::Trivial | ComplexityEstimate::Low => {
                // For small trees, use merkle reconstruction
                if compression_params.merkle_tree_height < 15 {
                    ReconstructionStrategy::MerkleReconstruction
                } else {
                    ReconstructionStrategy::MathematicalReconstruction
                }
            },
            ComplexityEstimate::Medium => {
                ReconstructionStrategy::MathematicalReconstruction
            },
            ComplexityEstimate::High | ComplexityEstimate::VeryHigh => {
                ReconstructionStrategy::Hybrid
            },
        }
    }

    /// Execute the selected reconstruction strategy
    async fn execute_reconstruction_strategy(
        &self,
        strategy: ReconstructionStrategy,
        truncated_data: &TruncatedData,
        compression_params: &CompressionParams,
        operation_id: uuid::Uuid,
    ) -> ReconstructionResult<ReconstructedAccount> {

        debug!(
            operation_id = %operation_id,
            strategy = ?strategy,
            "🔧 Executing reconstruction strategy"
        );

        match strategy {
            ReconstructionStrategy::FastPattern => {
                let result = self.pattern_matcher.reconstruct_from_pattern(
                    truncated_data
                ).await?;

                Ok(ReconstructedAccount {
                    account_data: result.data,
                    reconstruction_method: ReconstructionMethod::PatternMatching {
                        pattern_id: result.pattern_id.clone()
                    },
                    confidence_score: result.confidence,
                    verification_proof: None, // Pattern matching doesn't generate proofs
                    reconstruction_time: Duration::from_millis(0), // Will be set later
                    cache_hint: CacheHint {
                        cache_key: String::new(), // Will be set later
                        ttl: Duration::from_secs(300),
                        pattern_category: Some(result.pattern_id),
                        reuse_probability: 0.9, // High reuse for patterns
                    },
                })
            },

            ReconstructionStrategy::MerkleReconstruction => {
                let result = self.merkle_reconstructor.reconstruct(
                    truncated_data,
                    compression_params
                ).await?;

                Ok(ReconstructedAccount {
                    account_data: result.account_data,
                    reconstruction_method: ReconstructionMethod::MerkleTreeReconstruction,
                    confidence_score: result.confidence_score,
                    verification_proof: Some(result.merkle_proof),
                    reconstruction_time: Duration::from_millis(0), // Will be set later
                    cache_hint: CacheHint {
                        cache_key: String::new(), // Will be set later
                        ttl: Duration::from_secs(60),
                        pattern_category: None,
                        reuse_probability: 0.3,
                    },
                })
            },

            ReconstructionStrategy::MathematicalReconstruction => {
                let result = self.constraint_solver.solve_reconstruction(
                    truncated_data,
                    compression_params
                ).await?;

                Ok(ReconstructedAccount {
                    account_data: result.reconstructed_data,
                    reconstruction_method: ReconstructionMethod::ConstraintSolving,
                    confidence_score: result.solution_confidence,
                    verification_proof: result.verification_proof,
                    reconstruction_time: Duration::from_millis(0), // Will be set later
                    cache_hint: CacheHint {
                        cache_key: String::new(), // Will be set later
                        ttl: Duration::from_secs(120),
                        pattern_category: None,
                        reuse_probability: 0.5,
                    },
                })
            },

            ReconstructionStrategy::Hybrid => {
                // Try multiple strategies and return the best result
                self.execute_hybrid_reconstruction(truncated_data, compression_params, operation_id).await
            },
        }
    }

    /// Execute hybrid reconstruction (multiple strategies)
    async fn execute_hybrid_reconstruction(
        &self,
        truncated_data: &TruncatedData,
        compression_params: &CompressionParams,
        operation_id: uuid::Uuid,
    ) -> ReconstructionResult<ReconstructedAccount> {

        let strategies = vec![
            ReconstructionStrategy::MerkleReconstruction,
            ReconstructionStrategy::MathematicalReconstruction,
        ];

        let mut best_result = None;
        let mut best_confidence = 0.0;

        for strategy in strategies {
            match Box::pin(self.execute_reconstruction_strategy(
                strategy,
                truncated_data,
                compression_params,
                operation_id
            )).await {
                Ok(result) => {
                    if result.confidence_score > best_confidence {
                        best_confidence = result.confidence_score;
                        best_result = Some(result);
                    }
                },
                Err(e) => {
                    warn!("Strategy {:?} failed: {}", strategy, e);
                }
            }
        }

        best_result.ok_or_else(|| {
            ReconstructionError::internal("All hybrid strategies failed")
        })
    }

    /// Generate cache hint for reconstruction result
    fn generate_cache_hint(
        &self,
        reconstructed: &ReconstructedAccount,
        truncated_data: &TruncatedData,
    ) -> CacheHint {
        let cache_key = format!(
            "{}:{}:{}",
            truncated_data.metadata.account,
            truncated_data.metadata.slot,
            blake3::hash(&truncated_data.data).to_hex()
        );

        let ttl = match reconstructed.reconstruction_method {
            ReconstructionMethod::PatternMatching { .. } => Duration::from_secs(300),
            ReconstructionMethod::MerkleTreeReconstruction => Duration::from_secs(60),
            ReconstructionMethod::ConstraintSolving => Duration::from_secs(120),
            ReconstructionMethod::Hybrid { .. } => Duration::from_secs(180),
        };

        let reuse_probability = match reconstructed.confidence_score {
            score if score > 0.9 => 0.8,
            score if score > 0.8 => 0.6,
            score if score > 0.7 => 0.4,
            _ => 0.2,
        };

        CacheHint {
            cache_key,
            ttl,
            pattern_category: None, // Will be set by pattern matcher if applicable
            reuse_probability,
        }
    }
}

impl Default for ZKReconstructionLibrary {
    fn default() -> Self {
        Self::new()
    }
}