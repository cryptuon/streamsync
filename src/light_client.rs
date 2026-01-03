//! Light Client for StreamSync API consumers

use crate::economics::PaymentToken;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use anyhow::{Result, anyhow};
use uuid::Uuid;

/// Light client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightClientConfig {
    pub gateway_url: String,
    pub api_key: String,
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
    pub retry_delay_ms: u64,
}

/// Query request to StreamSync network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    pub query_type: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub priority: f32, // 1.0 = normal, 2.0 = high, 0.5 = low
}

/// Query response from StreamSync network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub request_id: String,
    pub processing_time_ms: u64,
    pub cost: f64,
    pub token: PaymentToken,
}

/// Account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub user_id: Uuid,
    pub tier: String,
    pub requests_this_month: u32,
    pub credits: HashMap<PaymentToken, f64>,
    pub total_spent: HashMap<PaymentToken, f64>,
}

/// Pricing tier information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingTier {
    pub name: String,
    pub requests_per_minute: u32,
    pub requests_per_month: u32,
    pub cost_per_request_strm: f64,
    pub cost_per_request_sol: f64,
    pub features: Vec<String>,
}

/// StreamSync Light Client
pub struct StreamSyncClient {
    config: LightClientConfig,
    http_client: Client,
}

impl StreamSyncClient {
    /// Create new light client
    pub fn new(config: LightClientConfig) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()?;

        Ok(Self {
            config,
            http_client,
        })
    }

    /// Create client from environment variables
    pub fn from_env() -> Result<Self> {
        let config = LightClientConfig {
            gateway_url: std::env::var("STREAMSYNC_GATEWAY_URL")
                .unwrap_or_else(|_| "https://api.streamsync.io".to_string()),
            api_key: std::env::var("STREAMSYNC_API_KEY")
                .map_err(|_| anyhow!("STREAMSYNC_API_KEY environment variable required"))?,
            timeout_seconds: std::env::var("STREAMSYNC_TIMEOUT")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
            retry_attempts: std::env::var("STREAMSYNC_RETRY_ATTEMPTS")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .unwrap_or(3),
            retry_delay_ms: std::env::var("STREAMSYNC_RETRY_DELAY")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()
                .unwrap_or(1000),
        };

        Self::new(config)
    }

    /// Execute query with retry logic
    pub async fn query(&self, request: QueryRequest) -> Result<QueryResponse> {
        let mut last_error = None;

        for attempt in 0..self.config.retry_attempts {
            match self.execute_query(&request).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.config.retry_attempts - 1 {
                        tokio::time::sleep(Duration::from_millis(self.config.retry_delay_ms)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("Query failed after retries")))
    }

    /// Execute single query attempt
    async fn execute_query(&self, request: &QueryRequest) -> Result<QueryResponse> {
        let url = format!("{}/api/v1/query", self.config.gateway_url);

        let response = self.http_client
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("content-type", "application/json")
            .json(request)
            .send()
            .await?;

        if response.status().is_success() {
            let query_response: QueryResponse = response.json().await?;
            Ok(query_response)
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow!("Query failed: {}", error_text))
        }
    }

    /// Get account information
    pub async fn get_account_info(&self) -> Result<AccountInfo> {
        let url = format!("{}/api/v1/account/usage", self.config.gateway_url);

        let response = self.http_client
            .get(&url)
            .header("x-api-key", &self.config.api_key)
            .send()
            .await?;

        if response.status().is_success() {
            let account_info: AccountInfo = response.json().await?;
            Ok(account_info)
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow!("Failed to get account info: {}", error_text))
        }
    }

    /// Get pricing information
    pub async fn get_pricing(&self) -> Result<HashMap<String, PricingTier>> {
        let url = format!("{}/api/v1/pricing", self.config.gateway_url);

        let response = self.http_client
            .get(&url)
            .send()
            .await?;

        if response.status().is_success() {
            #[derive(Deserialize)]
            struct PricingResponse {
                tiers: HashMap<String, PricingTier>,
            }

            let pricing_response: PricingResponse = response.json().await?;
            Ok(pricing_response.tiers)
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow!("Failed to get pricing: {}", error_text))
        }
    }

    /// Add credits to account
    pub async fn add_credits(&self, amount: f64, token: PaymentToken, tx_signature: Option<String>) -> Result<bool> {
        let url = format!("{}/api/v1/payment/add-credits", self.config.gateway_url);

        #[derive(Serialize)]
        struct PaymentRequest {
            amount: f64,
            token: PaymentToken,
            tx_signature: Option<String>,
        }

        let request = PaymentRequest {
            amount,
            token,
            tx_signature,
        };

        let response = self.http_client
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            #[derive(Deserialize)]
            struct PaymentResponse {
                success: bool,
            }

            let payment_response: PaymentResponse = response.json().await?;
            Ok(payment_response.success)
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow!("Failed to add credits: {}", error_text))
        }
    }

    // Convenience methods for common queries

    /// Get transaction by signature
    pub async fn get_transaction(&self, signature: &str, priority: Option<f32>) -> Result<QueryResponse> {
        let mut params = HashMap::new();
        params.insert("signature".to_string(), serde_json::Value::String(signature.to_string()));

        let request = QueryRequest {
            query_type: "get_transaction".to_string(),
            parameters: params,
            priority: priority.unwrap_or(1.0),
        };

        self.query(request).await
    }

    /// Get account information by address
    pub async fn get_account(&self, address: &str, priority: Option<f32>) -> Result<QueryResponse> {
        let mut params = HashMap::new();
        params.insert("address".to_string(), serde_json::Value::String(address.to_string()));

        let request = QueryRequest {
            query_type: "get_account".to_string(),
            parameters: params,
            priority: priority.unwrap_or(1.0),
        };

        self.query(request).await
    }

    /// Search transactions with filters
    pub async fn search_transactions(
        &self,
        filters: HashMap<String, serde_json::Value>,
        limit: Option<u32>,
        priority: Option<f32>,
    ) -> Result<QueryResponse> {
        let mut params = filters;
        if let Some(limit) = limit {
            params.insert("limit".to_string(), serde_json::Value::Number(limit.into()));
        }

        let request = QueryRequest {
            query_type: "search_transactions".to_string(),
            parameters: params,
            priority: priority.unwrap_or(1.0),
        };

        self.query(request).await
    }

    /// Get token accounts for owner
    pub async fn get_token_accounts(&self, owner: &str, mint: Option<&str>, priority: Option<f32>) -> Result<QueryResponse> {
        let mut params = HashMap::new();
        params.insert("owner".to_string(), serde_json::Value::String(owner.to_string()));

        if let Some(mint) = mint {
            params.insert("mint".to_string(), serde_json::Value::String(mint.to_string()));
        }

        let request = QueryRequest {
            query_type: "get_token_accounts".to_string(),
            parameters: params,
            priority: priority.unwrap_or(1.0),
        };

        self.query(request).await
    }

    /// Get program accounts
    pub async fn get_program_accounts(
        &self,
        program_id: &str,
        filters: Option<HashMap<String, serde_json::Value>>,
        priority: Option<f32>,
    ) -> Result<QueryResponse> {
        let mut params = HashMap::new();
        params.insert("program_id".to_string(), serde_json::Value::String(program_id.to_string()));

        if let Some(filters) = filters {
            params.insert("filters".to_string(), serde_json::to_value(filters)?);
        }

        let request = QueryRequest {
            query_type: "get_program_accounts".to_string(),
            parameters: params,
            priority: priority.unwrap_or(1.0),
        };

        self.query(request).await
    }

    /// Perform complex analytics query
    pub async fn analytics_query(
        &self,
        query_sql: &str,
        parameters: Option<HashMap<String, serde_json::Value>>,
        priority: Option<f32>,
    ) -> Result<QueryResponse> {
        let mut params = HashMap::new();
        params.insert("sql".to_string(), serde_json::Value::String(query_sql.to_string()));

        if let Some(parameters) = parameters {
            params.insert("parameters".to_string(), serde_json::to_value(parameters)?);
        }

        let request = QueryRequest {
            query_type: "complex_analytics".to_string(),
            parameters: params,
            priority: priority.unwrap_or(1.0),
        };

        self.query(request).await
    }

    /// Check service health
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/api/v1/health", self.config.gateway_url);

        let response = self.http_client
            .get(&url)
            .send()
            .await?;

        Ok(response.status().is_success())
    }
}

/// Builder for creating StreamSync client with custom configuration
pub struct ClientBuilder {
    config: LightClientConfig,
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self {
            config: LightClientConfig {
                gateway_url: "https://api.streamsync.io".to_string(),
                api_key: String::new(),
                timeout_seconds: 30,
                retry_attempts: 3,
                retry_delay_ms: 1000,
            },
        }
    }

    pub fn gateway_url(mut self, url: &str) -> Self {
        self.config.gateway_url = url.to_string();
        self
    }

    pub fn api_key(mut self, key: &str) -> Self {
        self.config.api_key = key.to_string();
        self
    }

    pub fn timeout(mut self, seconds: u64) -> Self {
        self.config.timeout_seconds = seconds;
        self
    }

    pub fn retry_attempts(mut self, attempts: u32) -> Self {
        self.config.retry_attempts = attempts;
        self
    }

    pub fn retry_delay(mut self, delay_ms: u64) -> Self {
        self.config.retry_delay_ms = delay_ms;
        self
    }

    pub fn build(self) -> Result<StreamSyncClient> {
        if self.config.api_key.is_empty() {
            return Err(anyhow!("API key is required"));
        }
        StreamSyncClient::new(self.config)
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_builder() {
        let client = ClientBuilder::new()
            .gateway_url("http://localhost:8080")
            .api_key("test_key")
            .timeout(60)
            .retry_attempts(5)
            .build();

        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.config.gateway_url, "http://localhost:8080");
        assert_eq!(client.config.api_key, "test_key");
        assert_eq!(client.config.timeout_seconds, 60);
        assert_eq!(client.config.retry_attempts, 5);
    }

    #[tokio::test]
    async fn test_query_request_creation() {
        let mut params = HashMap::new();
        params.insert("signature".to_string(), serde_json::Value::String("test_sig".to_string()));

        let request = QueryRequest {
            query_type: "get_transaction".to_string(),
            parameters: params,
            priority: 1.5,
        };

        assert_eq!(request.query_type, "get_transaction");
        assert_eq!(request.priority, 1.5);
        assert!(request.parameters.contains_key("signature"));
    }
}