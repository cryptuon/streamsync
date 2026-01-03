//! Payment Gateway for StreamSync API access
//!
//! This module handles payment verification and API access control.
//! Supports both Solana (SOL, STRM, USDC) and Stripe (USD) payments.

use crate::economics::{EconomicsManager, PaymentToken};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::str::FromStr;
use tokio::sync::RwLock;
use uuid::Uuid;
use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Signature,
};
use solana_transaction_status::UiTransactionEncoding;
use tracing::{info, warn};

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

    /// Verify Solana payment by checking transaction on-chain
    async fn verify_solana_payment(&self, tx_signature: &str, expected_amount: f64, token: &PaymentToken) -> Result<bool> {
        info!("Verifying Solana payment: sig={}, amount={}, token={:?}", tx_signature, expected_amount, token);

        // Parse signature
        let signature = Signature::from_str(tx_signature)
            .map_err(|e| anyhow!("Invalid transaction signature: {}", e))?;

        // Create RPC client
        let rpc_client = RpcClient::new(self.config.solana_rpc_url.clone());

        // Parse treasury wallet
        let treasury = Pubkey::from_str(&self.config.treasury_wallet)
            .map_err(|e| anyhow!("Invalid treasury wallet: {}", e))?;

        // Fetch transaction with retry
        let tx_result = rpc_client.get_transaction(&signature, UiTransactionEncoding::Json);

        match tx_result {
            Ok(tx) => {
                // Verify transaction was successful
                if let Some(meta) = tx.transaction.meta {
                    if meta.err.is_some() {
                        warn!("Transaction {} failed on-chain", tx_signature);
                        return Ok(false);
                    }

                    // Get pre/post balances to verify transfer
                    let pre_balances = meta.pre_balances;
                    let post_balances = meta.post_balances;

                    // For SOL transfers, check native balance changes
                    if *token == PaymentToken::SOL {
                        // Find treasury account index in the transaction
                        if let Some(decoded) = &tx.transaction.transaction.decode() {
                            let account_keys = decoded.message.static_account_keys();
                            for (i, key) in account_keys.iter().enumerate() {
                                if *key == treasury {
                                    // Check balance increased
                                    if i < post_balances.len() && i < pre_balances.len() {
                                        let received = (post_balances[i] as i64 - pre_balances[i] as i64) as f64
                                            / 1_000_000_000.0; // lamports to SOL
                                        if received >= expected_amount * 0.99 { // Allow 1% slippage
                                            info!("Payment verified: received {} SOL", received);
                                            return Ok(true);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // For SPL tokens (STRM, USDC), check token account balance changes
                    if let Some(token_balances) = Option::<Vec<_>>::from(meta.post_token_balances) {
                        for token_balance in token_balances {
                            // Get owner from OptionSerializer
                            let owner: Option<String> = Option::from(token_balance.owner.clone());
                            if let Some(owner_str) = owner {
                                if owner_str == self.config.treasury_wallet {
                                    // Found treasury token account - verify mint matches expected token
                                    let expected_mint = match token {
                                        PaymentToken::STRM => "STRMxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
                                        PaymentToken::USDC => "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                                        _ => continue,
                                    };

                                    if token_balance.mint == expected_mint {
                                        if let Some(amount) = token_balance.ui_token_amount.ui_amount {
                                            if amount >= expected_amount * 0.99 {
                                                info!("Payment verified: received {} {:?}", amount, token);
                                                return Ok(true);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                warn!("Could not verify payment amount for tx {}", tx_signature);
                Ok(false)
            }
            Err(e) => {
                warn!("Failed to fetch transaction {}: {}", tx_signature, e);
                Err(anyhow!("Failed to fetch transaction: {}", e))
            }
        }
    }

    /// Verify Stripe payment using Stripe API
    async fn verify_stripe_payment(&self, payment_intent_id: &str, expected_amount: f64) -> Result<bool> {
        info!("Verifying Stripe payment: intent={}, amount={}", payment_intent_id, expected_amount);

        // Check if Stripe is configured
        let stripe_key = self.config.stripe_secret_key.as_ref()
            .ok_or_else(|| anyhow!("Stripe not configured"))?;

        // Call Stripe API to verify payment intent
        let client = reqwest::Client::new();
        let url = format!("https://api.stripe.com/v1/payment_intents/{}", payment_intent_id);

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", stripe_key))
            .send()
            .await
            .map_err(|e| anyhow!("Failed to call Stripe API: {}", e))?;

        if !response.status().is_success() {
            warn!("Stripe API returned error for intent {}", payment_intent_id);
            return Ok(false);
        }

        let payment_intent: serde_json::Value = response.json().await
            .map_err(|e| anyhow!("Failed to parse Stripe response: {}", e))?;

        // Verify payment status
        let status = payment_intent["status"].as_str().unwrap_or("");
        if status != "succeeded" {
            warn!("Payment intent {} status is {}, not succeeded", payment_intent_id, status);
            return Ok(false);
        }

        // Verify amount (Stripe amounts are in cents)
        let amount_cents = payment_intent["amount"].as_i64().unwrap_or(0);
        let amount_usd = amount_cents as f64 / 100.0;

        if amount_usd >= expected_amount * 0.99 {
            info!("Stripe payment verified: ${:.2}", amount_usd);
            Ok(true)
        } else {
            warn!("Payment amount ${:.2} less than expected ${:.2}", amount_usd, expected_amount);
            Ok(false)
        }
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