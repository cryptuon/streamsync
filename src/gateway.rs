//! Payment Gateway for StreamSync API access

use crate::economics::{EconomicsManager, PaymentToken, RequestCost};
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use anyhow::Result;

/// API key authentication
#[derive(Debug, Clone)]
pub struct ApiKey(pub String);

/// Request authentication middleware
pub async fn authenticate_request(
    headers: &HeaderMap,
    economics: &EconomicsManager,
) -> Result<Uuid, StatusCode> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Find user by API key
    for (user_id, account) in economics.user_accounts.iter() {
        if account.api_key == api_key {
            return Ok(*user_id);
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}

/// Payment gateway configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub stripe_secret_key: Option<String>,
    pub solana_rpc_url: String,
    pub treasury_wallet: String,
    pub supported_tokens: Vec<PaymentToken>,
    pub minimum_payment_usd: f64,
}

/// Payment request
#[derive(Debug, Deserialize)]
pub struct PaymentRequest {
    pub amount: f64,
    pub token: PaymentToken,
    pub user_id: Uuid,
    /// Solana transaction signature for crypto payments
    pub tx_signature: Option<String>,
    /// Stripe payment intent ID for fiat payments
    pub payment_intent_id: Option<String>,
}

/// Payment response
#[derive(Debug, Serialize)]
pub struct PaymentResponse {
    pub success: bool,
    pub message: String,
    pub credits_added: f64,
    pub new_balance: f64,
}

/// Account creation request
#[derive(Debug, Deserialize)]
pub struct CreateAccountRequest {
    pub email: String,
    pub tier: String,
}

/// Account creation response
#[derive(Debug, Serialize)]
pub struct CreateAccountResponse {
    pub user_id: Uuid,
    pub api_key: String,
    pub tier: String,
}

/// Usage statistics
#[derive(Debug, Serialize)]
pub struct UsageStats {
    pub requests_this_month: u32,
    pub total_spent: HashMap<PaymentToken, f64>,
    pub credits: HashMap<PaymentToken, f64>,
    pub tier: String,
}

/// Pricing information
#[derive(Debug, Serialize)]
pub struct PricingInfo {
    pub tiers: HashMap<String, serde_json::Value>,
    pub supported_tokens: Vec<PaymentToken>,
}

/// Payment gateway
pub struct PaymentGateway {
    config: GatewayConfig,
    economics: Arc<RwLock<EconomicsManager>>,
}

impl PaymentGateway {
    pub fn new(config: GatewayConfig, economics: Arc<RwLock<EconomicsManager>>) -> Self {
        Self { config, economics }
    }

    /// Create router for payment gateway endpoints
    pub fn create_router(gateway: Arc<Self>) -> Router {
        Router::new()
            .route("/api/v1/account/create", post(create_account))
            .route("/api/v1/account/usage", get(get_usage))
            .route("/api/v1/payment/add-credits", post(add_credits))
            .route("/api/v1/pricing", get(get_pricing))
            .route("/api/v1/health", get(health_check))
            .with_state(gateway)
    }

    /// Verify Solana payment
    async fn verify_solana_payment(&self, tx_signature: &str, expected_amount: f64, token: &PaymentToken) -> Result<bool> {
        // In a real implementation, this would:
        // 1. Query the Solana RPC to get transaction details
        // 2. Verify the transaction was successful
        // 3. Verify the amount and recipient match expectations
        // 4. Verify the token type matches

        // For now, return true as placeholder
        // TODO: Implement actual Solana transaction verification
        Ok(true)
    }

    /// Verify Stripe payment
    async fn verify_stripe_payment(&self, payment_intent_id: &str, expected_amount: f64) -> Result<bool> {
        // In a real implementation, this would:
        // 1. Call Stripe API to verify payment intent
        // 2. Check that payment was successful
        // 3. Verify amount matches

        // For now, return true as placeholder
        // TODO: Implement actual Stripe payment verification
        Ok(true)
    }
}

/// Create new user account
async fn create_account(
    State(gateway): State<Arc<PaymentGateway>>,
    Json(request): Json<CreateAccountRequest>,
) -> Result<Json<CreateAccountResponse>, StatusCode> {
    let api_key = format!("strm_{}", Uuid::new_v4().to_string().replace("-", ""));

    let mut economics = gateway.economics.write().await;

    match economics.create_account(api_key.clone(), request.tier.clone()).await {
        Ok(user_id) => Ok(Json(CreateAccountResponse {
            user_id,
            api_key,
            tier: request.tier,
        })),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Get user usage statistics
async fn get_usage(
    State(gateway): State<Arc<PaymentGateway>>,
    headers: HeaderMap,
) -> Result<Json<UsageStats>, StatusCode> {
    let economics = gateway.economics.read().await;
    let user_id = authenticate_request(&headers, &economics).await?;

    if let Some(account) = economics.get_account(&user_id) {
        Ok(Json(UsageStats {
            requests_this_month: account.monthly_usage,
            total_spent: account.total_spent.clone(),
            credits: account.credits.clone(),
            tier: account.tier.clone(),
        }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Add credits to user account
async fn add_credits(
    State(gateway): State<Arc<PaymentGateway>>,
    Json(request): Json<PaymentRequest>,
) -> Result<Json<PaymentResponse>, StatusCode> {
    // Verify payment based on type
    let payment_verified = match (&request.tx_signature, &request.payment_intent_id) {
        (Some(tx_sig), None) => {
            // Solana payment
            gateway.verify_solana_payment(tx_sig, request.amount, &request.token).await
                .map_err(|_| StatusCode::BAD_REQUEST)?
        }
        (None, Some(intent_id)) => {
            // Stripe payment
            if request.token != PaymentToken::STRM {
                return Err(StatusCode::BAD_REQUEST);
            }
            gateway.verify_stripe_payment(intent_id, request.amount).await
                .map_err(|_| StatusCode::BAD_REQUEST)?
        }
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    if !payment_verified {
        return Ok(Json(PaymentResponse {
            success: false,
            message: "Payment verification failed".to_string(),
            credits_added: 0.0,
            new_balance: 0.0,
        }));
    }

    let mut economics = gateway.economics.write().await;

    match economics.add_credits(request.user_id, request.amount, request.token.clone()).await {
        Ok(_) => {
            let new_balance = economics
                .get_account(&request.user_id)
                .and_then(|acc| acc.credits.get(&request.token))
                .cloned()
                .unwrap_or(0.0);

            Ok(Json(PaymentResponse {
                success: true,
                message: "Credits added successfully".to_string(),
                credits_added: request.amount,
                new_balance,
            }))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Get pricing information
async fn get_pricing(
    State(gateway): State<Arc<PaymentGateway>>,
) -> Json<PricingInfo> {
    let economics = gateway.economics.read().await;
    let mut tiers = HashMap::new();

    for (name, tier) in &economics.pricing_tiers {
        tiers.insert(name.clone(), serde_json::to_value(tier).unwrap());
    }

    Json(PricingInfo {
        tiers,
        supported_tokens: gateway.config.supported_tokens.clone(),
    })
}

/// Health check endpoint
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "streamsync-gateway",
        "version": "0.1.0"
    }))
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    headers: &HeaderMap,
    economics: &EconomicsManager,
) -> Result<(), StatusCode> {
    let user_id = authenticate_request(headers, economics).await?;

    if !economics.can_make_request(&user_id) {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(())
}

/// Charge for API request
pub async fn charge_for_request(
    user_id: Uuid,
    query_type: &str,
    data_size_kb: u64,
    priority: f32,
    economics: &mut EconomicsManager,
) -> Result<bool, StatusCode> {
    let account = economics.get_account(&user_id).ok_or(StatusCode::NOT_FOUND)?;

    // Determine payment token (prefer STRM, fallback to SOL)
    let token = if account.credits.get(&PaymentToken::STRM).unwrap_or(&0.0) > &0.0 {
        PaymentToken::STRM
    } else if account.credits.get(&PaymentToken::SOL).unwrap_or(&0.0) > &0.0 {
        PaymentToken::SOL
    } else {
        return Err(StatusCode::PAYMENT_REQUIRED);
    };

    let cost = economics.calculate_request_cost(
        query_type,
        data_size_kb,
        priority,
        &account.tier,
        token,
    );

    match economics.charge_request(user_id, cost).await {
        Ok(charged) => {
            if charged {
                Ok(true)
            } else {
                Err(StatusCode::PAYMENT_REQUIRED)
            }
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}