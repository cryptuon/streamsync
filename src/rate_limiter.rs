//! Rate limiting and usage tracking for StreamSync API

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use anyhow::Result;

/// Rate limiting strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RateLimitStrategy {
    /// Token bucket algorithm
    TokenBucket {
        capacity: u32,
        refill_rate: u32, // tokens per second
    },
    /// Fixed window counter
    FixedWindow {
        requests: u32,
        window_seconds: u64,
    },
    /// Sliding window log
    SlidingWindow {
        requests: u32,
        window_seconds: u64,
    },
    /// Leaky bucket algorithm
    LeakyBucket {
        capacity: u32,
        leak_rate: u32, // requests per second
    },
}

/// Rate limit configuration for different tiers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub tier: String,
    pub per_minute: RateLimitStrategy,
    pub per_hour: RateLimitStrategy,
    pub per_day: RateLimitStrategy,
    pub burst_allowance: u32,
}

/// Token bucket state
#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    capacity: u32,
    refill_rate: u32,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(capacity: u32, refill_rate: u32) -> Self {
        Self {
            tokens: capacity as f64,
            capacity,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn try_consume(&mut self, tokens: u32) -> bool {
        self.refill();

        if self.tokens >= tokens as f64 {
            self.tokens -= tokens as f64;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();

        let new_tokens = elapsed * self.refill_rate as f64;
        self.tokens = (self.tokens + new_tokens).min(self.capacity as f64);
        self.last_refill = now;
    }
}

/// Fixed window counter state
#[derive(Debug, Clone)]
struct FixedWindow {
    count: u32,
    window_start: Instant,
    requests: u32,
    window_duration: Duration,
}

impl FixedWindow {
    fn new(requests: u32, window_seconds: u64) -> Self {
        Self {
            count: 0,
            window_start: Instant::now(),
            requests,
            window_duration: Duration::from_secs(window_seconds),
        }
    }

    fn try_consume(&mut self, tokens: u32) -> bool {
        let now = Instant::now();

        // Reset window if expired
        if now.duration_since(self.window_start) >= self.window_duration {
            self.count = 0;
            self.window_start = now;
        }

        if self.count + tokens <= self.requests {
            self.count += tokens;
            true
        } else {
            false
        }
    }
}

/// Sliding window log state
#[derive(Debug, Clone)]
struct SlidingWindow {
    requests: Vec<Instant>,
    max_requests: u32,
    window_duration: Duration,
}

impl SlidingWindow {
    fn new(requests: u32, window_seconds: u64) -> Self {
        Self {
            requests: Vec::new(),
            max_requests: requests,
            window_duration: Duration::from_secs(window_seconds),
        }
    }

    fn try_consume(&mut self, tokens: u32) -> bool {
        let now = Instant::now();

        // Remove expired requests
        self.requests.retain(|&timestamp| {
            now.duration_since(timestamp) < self.window_duration
        });

        if self.requests.len() as u32 + tokens <= self.max_requests {
            for _ in 0..tokens {
                self.requests.push(now);
            }
            true
        } else {
            false
        }
    }
}

/// Rate limiter state for a user
#[derive(Debug, Clone)]
pub struct UserRateLimitState {
    pub user_id: Uuid,
    pub tier: String,
    per_minute: Box<dyn RateLimitAlgorithm + Send + Sync>,
    per_hour: Box<dyn RateLimitAlgorithm + Send + Sync>,
    per_day: Box<dyn RateLimitAlgorithm + Send + Sync>,
    burst_tokens: u32,
}

/// Rate limiting algorithm trait
pub trait RateLimitAlgorithm: std::fmt::Debug {
    fn try_consume(&mut self, tokens: u32) -> bool;
    fn clone_box(&self) -> Box<dyn RateLimitAlgorithm + Send + Sync>;
}

impl Clone for Box<dyn RateLimitAlgorithm + Send + Sync> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

impl RateLimitAlgorithm for TokenBucket {
    fn try_consume(&mut self, tokens: u32) -> bool {
        self.try_consume(tokens)
    }

    fn clone_box(&self) -> Box<dyn RateLimitAlgorithm + Send + Sync> {
        Box::new(self.clone())
    }
}

impl RateLimitAlgorithm for FixedWindow {
    fn try_consume(&mut self, tokens: u32) -> bool {
        self.try_consume(tokens)
    }

    fn clone_box(&self) -> Box<dyn RateLimitAlgorithm + Send + Sync> {
        Box::new(self.clone())
    }
}

impl RateLimitAlgorithm for SlidingWindow {
    fn try_consume(&mut self, tokens: u32) -> bool {
        self.try_consume(tokens)
    }

    fn clone_box(&self) -> Box<dyn RateLimitAlgorithm + Send + Sync> {
        Box::new(self.clone())
    }
}

/// Usage tracking metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageMetrics {
    pub user_id: Uuid,
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub requests_per_day: u32,
    pub total_requests: u64,
    pub data_transferred_mb: f64,
    pub average_response_time_ms: f64,
    pub error_rate: f64,
    pub last_request: i64,
}

/// Rate limiter and usage tracker
pub struct RateLimiter {
    user_states: Arc<RwLock<HashMap<Uuid, UserRateLimitState>>>,
    usage_metrics: Arc<RwLock<HashMap<Uuid, UsageMetrics>>>,
    configs: HashMap<String, RateLimitConfig>,
}

impl RateLimiter {
    pub fn new() -> Self {
        let mut configs = HashMap::new();

        // Free tier configuration
        configs.insert("free".to_string(), RateLimitConfig {
            tier: "free".to_string(),
            per_minute: RateLimitStrategy::TokenBucket {
                capacity: 10,
                refill_rate: 10, // 10 requests per minute
            },
            per_hour: RateLimitStrategy::FixedWindow {
                requests: 100,
                window_seconds: 3600,
            },
            per_day: RateLimitStrategy::FixedWindow {
                requests: 1000,
                window_seconds: 86400,
            },
            burst_allowance: 5,
        });

        // Basic tier configuration
        configs.insert("basic".to_string(), RateLimitConfig {
            tier: "basic".to_string(),
            per_minute: RateLimitStrategy::TokenBucket {
                capacity: 100,
                refill_rate: 100,
            },
            per_hour: RateLimitStrategy::FixedWindow {
                requests: 5000,
                window_seconds: 3600,
            },
            per_day: RateLimitStrategy::FixedWindow {
                requests: 50000,
                window_seconds: 86400,
            },
            burst_allowance: 50,
        });

        // Pro tier configuration
        configs.insert("pro".to_string(), RateLimitConfig {
            tier: "pro".to_string(),
            per_minute: RateLimitStrategy::TokenBucket {
                capacity: 1000,
                refill_rate: 1000,
            },
            per_hour: RateLimitStrategy::SlidingWindow {
                requests: 60000,
                window_seconds: 3600,
            },
            per_day: RateLimitStrategy::SlidingWindow {
                requests: 1000000,
                window_seconds: 86400,
            },
            burst_allowance: 500,
        });

        // Enterprise tier (unlimited)
        configs.insert("enterprise".to_string(), RateLimitConfig {
            tier: "enterprise".to_string(),
            per_minute: RateLimitStrategy::TokenBucket {
                capacity: u32::MAX,
                refill_rate: u32::MAX,
            },
            per_hour: RateLimitStrategy::TokenBucket {
                capacity: u32::MAX,
                refill_rate: u32::MAX,
            },
            per_day: RateLimitStrategy::TokenBucket {
                capacity: u32::MAX,
                refill_rate: u32::MAX,
            },
            burst_allowance: u32::MAX,
        });

        Self {
            user_states: Arc::new(RwLock::new(HashMap::new())),
            usage_metrics: Arc::new(RwLock::new(HashMap::new())),
            configs,
        }
    }

    /// Initialize or update user rate limit state
    pub async fn init_user(&self, user_id: Uuid, tier: &str) -> Result<()> {
        let config = self.configs.get(tier).ok_or_else(|| {
            anyhow::anyhow!("Unknown tier: {}", tier)
        })?;

        let per_minute = self.create_algorithm(&config.per_minute);
        let per_hour = self.create_algorithm(&config.per_hour);
        let per_day = self.create_algorithm(&config.per_day);

        let state = UserRateLimitState {
            user_id,
            tier: tier.to_string(),
            per_minute,
            per_hour,
            per_day,
            burst_tokens: config.burst_allowance,
        };

        let mut states = self.user_states.write().await;
        states.insert(user_id, state);

        // Initialize usage metrics
        let mut metrics = self.usage_metrics.write().await;
        metrics.entry(user_id).or_insert_with(|| UsageMetrics {
            user_id,
            requests_per_minute: 0,
            requests_per_hour: 0,
            requests_per_day: 0,
            total_requests: 0,
            data_transferred_mb: 0.0,
            average_response_time_ms: 0.0,
            error_rate: 0.0,
            last_request: chrono::Utc::now().timestamp(),
        });

        Ok(())
    }

    /// Check if user can make request (consumes tokens if allowed)
    pub async fn try_request(&self, user_id: Uuid, cost: u32) -> Result<bool> {
        let mut states = self.user_states.write().await;

        if let Some(state) = states.get_mut(&user_id) {
            // Try all time windows
            let minute_ok = state.per_minute.try_consume(cost);
            let hour_ok = state.per_hour.try_consume(cost);
            let day_ok = state.per_day.try_consume(cost);

            let allowed = minute_ok && hour_ok && day_ok;

            if allowed {
                // Update usage metrics
                let mut metrics = self.usage_metrics.write().await;
                if let Some(metric) = metrics.get_mut(&user_id) {
                    metric.total_requests += cost as u64;
                    metric.last_request = chrono::Utc::now().timestamp();
                }
            }

            Ok(allowed)
        } else {
            Ok(false) // User not initialized
        }
    }

    /// Update usage metrics after request completion
    pub async fn record_usage(
        &self,
        user_id: Uuid,
        response_time_ms: u64,
        data_size_mb: f64,
        is_error: bool,
    ) -> Result<()> {
        let mut metrics = self.usage_metrics.write().await;

        if let Some(metric) = metrics.get_mut(&user_id) {
            // Update response time (moving average)
            let alpha = 0.1; // Smoothing factor
            metric.average_response_time_ms =
                alpha * response_time_ms as f64 + (1.0 - alpha) * metric.average_response_time_ms;

            // Update data transfer
            metric.data_transferred_mb += data_size_mb;

            // Update error rate (moving average)
            let error_value = if is_error { 1.0 } else { 0.0 };
            metric.error_rate = alpha * error_value + (1.0 - alpha) * metric.error_rate;
        }

        Ok(())
    }

    /// Get user usage metrics
    pub async fn get_usage_metrics(&self, user_id: &Uuid) -> Option<UsageMetrics> {
        let metrics = self.usage_metrics.read().await;
        metrics.get(user_id).cloned()
    }

    /// Get all usage metrics (for admin/monitoring)
    pub async fn get_all_metrics(&self) -> HashMap<Uuid, UsageMetrics> {
        let metrics = self.usage_metrics.read().await;
        metrics.clone()
    }

    /// Update user tier
    pub async fn update_tier(&self, user_id: Uuid, new_tier: &str) -> Result<()> {
        self.init_user(user_id, new_tier).await
    }

    /// Create rate limiting algorithm from strategy
    fn create_algorithm(&self, strategy: &RateLimitStrategy) -> Box<dyn RateLimitAlgorithm + Send + Sync> {
        match strategy {
            RateLimitStrategy::TokenBucket { capacity, refill_rate } => {
                Box::new(TokenBucket::new(*capacity, *refill_rate))
            }
            RateLimitStrategy::FixedWindow { requests, window_seconds } => {
                Box::new(FixedWindow::new(*requests, *window_seconds))
            }
            RateLimitStrategy::SlidingWindow { requests, window_seconds } => {
                Box::new(SlidingWindow::new(*requests, *window_seconds))
            }
            RateLimitStrategy::LeakyBucket { capacity, leak_rate } => {
                // For simplicity, implement as token bucket
                Box::new(TokenBucket::new(*capacity, *leak_rate))
            }
        }
    }

    /// Get rate limit status for user
    pub async fn get_rate_limit_status(&self, user_id: &Uuid) -> Option<HashMap<String, serde_json::Value>> {
        let states = self.user_states.read().await;
        let metrics = self.usage_metrics.read().await;

        if let (Some(_state), Some(metric)) = (states.get(user_id), metrics.get(user_id)) {
            let mut status = HashMap::new();

            status.insert("tier".to_string(), serde_json::Value::String(metric.user_id.to_string()));
            status.insert("requests_per_minute".to_string(), serde_json::Value::Number(metric.requests_per_minute.into()));
            status.insert("requests_per_hour".to_string(), serde_json::Value::Number(metric.requests_per_hour.into()));
            status.insert("requests_per_day".to_string(), serde_json::Value::Number(metric.requests_per_day.into()));
            status.insert("total_requests".to_string(), serde_json::Value::Number(metric.total_requests.into()));
            status.insert("data_transferred_mb".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(metric.data_transferred_mb).unwrap_or_else(|| serde_json::Number::from(0))));
            status.insert("average_response_time_ms".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(metric.average_response_time_ms).unwrap_or_else(|| serde_json::Number::from(0))));
            status.insert("error_rate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(metric.error_rate).unwrap_or_else(|| serde_json::Number::from(0))));

            Some(status)
        } else {
            None
        }
    }

    /// Reset usage counters (for testing or manual reset)
    pub async fn reset_usage(&self, user_id: &Uuid) -> Result<()> {
        let mut metrics = self.usage_metrics.write().await;

        if let Some(metric) = metrics.get_mut(user_id) {
            metric.requests_per_minute = 0;
            metric.requests_per_hour = 0;
            metric.requests_per_day = 0;
            metric.data_transferred_mb = 0.0;
            metric.average_response_time_ms = 0.0;
            metric.error_rate = 0.0;
        }

        // Reinitialize rate limiting state
        let states = self.user_states.read().await;
        if let Some(state) = states.get(user_id) {
            let tier = state.tier.clone();
            drop(states);
            self.init_user(*user_id, &tier).await?;
        }

        Ok(())
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}