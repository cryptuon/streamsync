//! Economics and tokenomics for StreamSync network

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::broadcast;
use uuid::Uuid;
use anyhow::Result;

/// Payment token types supported by the network
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PaymentToken {
    /// Native STRM token
    STRM,
    /// SOL payments
    SOL,
    /// USDC payments
    USDC,
    /// Custom SPL token
    SPL(String), // Token mint address
}

/// API pricing tiers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingTier {
    pub name: String,
    pub requests_per_minute: u32,
    pub requests_per_month: u32,
    pub cost_per_request_strm: f64,
    pub cost_per_request_sol: f64,
    pub features: Vec<String>,
}

/// User account with credits and usage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAccount {
    pub user_id: Uuid,
    pub api_key: String,
    pub credits: HashMap<PaymentToken, f64>,
    pub tier: String,
    pub monthly_usage: u32,
    pub last_reset: i64,
    pub total_spent: HashMap<PaymentToken, f64>,
}

/// Request cost calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestCost {
    pub base_cost: f64,
    pub complexity_multiplier: f64,
    pub data_size_multiplier: f64,
    pub priority_multiplier: f64,
    pub total_cost: f64,
    pub token: PaymentToken,
}

/// Revenue sharing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueSharing {
    /// Percentage to network treasury (protocol development)
    pub treasury_percentage: f64,
    /// Percentage to node operators serving requests
    pub node_operator_percentage: f64,
    /// Percentage to data providers (Solana RPC nodes)
    pub data_provider_percentage: f64,
    /// Percentage to governance token holders
    pub governance_percentage: f64,
}

impl Default for RevenueSharing {
    fn default() -> Self {
        Self {
            treasury_percentage: 0.20, // 20%
            node_operator_percentage: 0.50, // 50%
            data_provider_percentage: 0.20, // 20%
            governance_percentage: 0.10, // 10%
        }
    }
}

/// Economics events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EconomicsEvent {
    PaymentReceived {
        user_id: Uuid,
        amount: f64,
        token: PaymentToken,
        timestamp: i64,
    },
    RequestCharged {
        user_id: Uuid,
        cost: RequestCost,
        timestamp: i64,
    },
    RevenueDistributed {
        total_amount: f64,
        token: PaymentToken,
        distribution: HashMap<String, f64>,
        timestamp: i64,
    },
    TierUpgraded {
        user_id: Uuid,
        old_tier: String,
        new_tier: String,
        timestamp: i64,
    },
}

/// Main economics manager
pub struct EconomicsManager {
    pub pricing_tiers: HashMap<String, PricingTier>,
    pub user_accounts: HashMap<Uuid, UserAccount>,
    revenue_sharing: RevenueSharing,
    event_sender: broadcast::Sender<EconomicsEvent>,
    pending_distributions: HashMap<PaymentToken, f64>,
}

impl EconomicsManager {
    pub fn new() -> (Self, broadcast::Receiver<EconomicsEvent>) {
        let (event_sender, event_receiver) = broadcast::channel(1000);

        let mut pricing_tiers = HashMap::new();

        // Free tier
        pricing_tiers.insert("free".to_string(), PricingTier {
            name: "Free".to_string(),
            requests_per_minute: 10,
            requests_per_month: 1000,
            cost_per_request_strm: 0.0,
            cost_per_request_sol: 0.0,
            features: vec!["Basic queries".to_string()],
        });

        // Basic tier
        pricing_tiers.insert("basic".to_string(), PricingTier {
            name: "Basic".to_string(),
            requests_per_minute: 100,
            requests_per_month: 50000,
            cost_per_request_strm: 0.001,
            cost_per_request_sol: 0.00001,
            features: vec![
                "Basic queries".to_string(),
                "Transaction history".to_string(),
                "Account lookups".to_string(),
            ],
        });

        // Pro tier
        pricing_tiers.insert("pro".to_string(), PricingTier {
            name: "Pro".to_string(),
            requests_per_minute: 1000,
            requests_per_month: 1000000,
            cost_per_request_strm: 0.0008,
            cost_per_request_sol: 0.000008,
            features: vec![
                "All basic features".to_string(),
                "Real-time webhooks".to_string(),
                "Advanced analytics".to_string(),
                "Priority support".to_string(),
            ],
        });

        // Enterprise tier
        pricing_tiers.insert("enterprise".to_string(), PricingTier {
            name: "Enterprise".to_string(),
            requests_per_minute: u32::MAX,
            requests_per_month: u32::MAX,
            cost_per_request_strm: 0.0005,
            cost_per_request_sol: 0.000005,
            features: vec![
                "All pro features".to_string(),
                "Unlimited requests".to_string(),
                "Custom integrations".to_string(),
                "Dedicated support".to_string(),
                "SLA guarantees".to_string(),
            ],
        });

        let manager = Self {
            pricing_tiers,
            user_accounts: HashMap::new(),
            revenue_sharing: RevenueSharing::default(),
            event_sender,
            pending_distributions: HashMap::new(),
        };

        (manager, event_receiver)
    }

    /// Create a new user account
    pub async fn create_account(&mut self, api_key: String, tier: String) -> Result<Uuid> {
        let user_id = Uuid::new_v4();

        let account = UserAccount {
            user_id,
            api_key,
            credits: HashMap::new(),
            tier: tier.clone(),
            monthly_usage: 0,
            last_reset: chrono::Utc::now().timestamp(),
            total_spent: HashMap::new(),
        };

        self.user_accounts.insert(user_id, account);
        Ok(user_id)
    }

    /// Add credits to user account
    pub async fn add_credits(&mut self, user_id: Uuid, amount: f64, token: PaymentToken) -> Result<()> {
        if let Some(account) = self.user_accounts.get_mut(&user_id) {
            *account.credits.entry(token.clone()).or_insert(0.0) += amount;

            let _ = self.event_sender.send(EconomicsEvent::PaymentReceived {
                user_id,
                amount,
                token,
                timestamp: chrono::Utc::now().timestamp(),
            });
        }
        Ok(())
    }

    /// Calculate request cost based on complexity and data size
    pub fn calculate_request_cost(&self,
        query_type: &str,
        data_size_kb: u64,
        priority: f32,
        user_tier: &str,
        token: PaymentToken
    ) -> RequestCost {
        let tier = self.pricing_tiers.get(user_tier).unwrap();

        // Base cost from tier
        let base_cost = match token {
            PaymentToken::STRM => tier.cost_per_request_strm,
            PaymentToken::SOL => tier.cost_per_request_sol,
            _ => tier.cost_per_request_sol, // Default to SOL pricing
        };

        // Complexity multiplier based on query type
        let complexity_multiplier = match query_type {
            "get_transaction" => 1.0,
            "get_account" => 1.2,
            "search_transactions" => 2.0,
            "get_token_accounts" => 1.5,
            "get_program_accounts" => 3.0,
            "complex_analytics" => 5.0,
            _ => 1.0,
        };

        // Data size multiplier (larger responses cost more)
        let data_size_multiplier = 1.0 + (data_size_kb as f64 / 1000.0) * 0.1;

        // Priority multiplier (higher priority costs more)
        let priority_multiplier = 1.0 + (priority as f64 - 1.0) * 0.5;

        let total_cost = base_cost * complexity_multiplier * data_size_multiplier * priority_multiplier;

        RequestCost {
            base_cost,
            complexity_multiplier,
            data_size_multiplier,
            priority_multiplier,
            total_cost,
            token,
        }
    }

    /// Charge user for request
    pub async fn charge_request(&mut self, user_id: Uuid, cost: RequestCost) -> Result<bool> {
        if let Some(account) = self.user_accounts.get_mut(&user_id) {
            let current_credits = account.credits.get(&cost.token).cloned().unwrap_or(0.0);

            if current_credits >= cost.total_cost {
                *account.credits.entry(cost.token.clone()).or_insert(0.0) -= cost.total_cost;
                *account.total_spent.entry(cost.token.clone()).or_insert(0.0) += cost.total_cost;
                account.monthly_usage += 1;

                // Add to pending distribution
                *self.pending_distributions.entry(cost.token.clone()).or_insert(0.0) += cost.total_cost;

                let _ = self.event_sender.send(EconomicsEvent::RequestCharged {
                    user_id,
                    cost: cost.clone(),
                    timestamp: chrono::Utc::now().timestamp(),
                });

                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Distribute pending revenue according to revenue sharing rules
    pub async fn distribute_revenue(&mut self, token: PaymentToken) -> Result<()> {
        if let Some(total_amount) = self.pending_distributions.remove(&token) {
            let mut distribution = HashMap::new();

            let treasury_amount = total_amount * self.revenue_sharing.treasury_percentage;
            let node_operator_amount = total_amount * self.revenue_sharing.node_operator_percentage;
            let data_provider_amount = total_amount * self.revenue_sharing.data_provider_percentage;
            let governance_amount = total_amount * self.revenue_sharing.governance_percentage;

            distribution.insert("treasury".to_string(), treasury_amount);
            distribution.insert("node_operators".to_string(), node_operator_amount);
            distribution.insert("data_providers".to_string(), data_provider_amount);
            distribution.insert("governance".to_string(), governance_amount);

            let _ = self.event_sender.send(EconomicsEvent::RevenueDistributed {
                total_amount,
                token: token.clone(),
                distribution: distribution.clone(),
                timestamp: chrono::Utc::now().timestamp(),
            });
        }
        Ok(())
    }

    /// Get user account info
    pub fn get_account(&self, user_id: &Uuid) -> Option<&UserAccount> {
        self.user_accounts.get(user_id)
    }

    /// Get pricing tier info
    pub fn get_pricing_tier(&self, tier: &str) -> Option<&PricingTier> {
        self.pricing_tiers.get(tier)
    }

    /// Check if user can make request (rate limiting)
    pub fn can_make_request(&self, user_id: &Uuid) -> bool {
        if let Some(account) = self.user_accounts.get(user_id) {
            if let Some(tier) = self.pricing_tiers.get(&account.tier) {
                return account.monthly_usage < tier.requests_per_month;
            }
        }
        false
    }

    /// Upgrade user tier
    pub async fn upgrade_tier(&mut self, user_id: Uuid, new_tier: String) -> Result<()> {
        if let Some(account) = self.user_accounts.get_mut(&user_id) {
            let old_tier = account.tier.clone();
            account.tier = new_tier.clone();

            let _ = self.event_sender.send(EconomicsEvent::TierUpgraded {
                user_id,
                old_tier,
                new_tier,
                timestamp: chrono::Utc::now().timestamp(),
            });
        }
        Ok(())
    }

    /// Get network statistics
    pub fn get_network_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();

        let total_users = self.user_accounts.len();
        let total_pending_revenue: f64 = self.pending_distributions.values().sum();

        let tier_distribution: HashMap<String, usize> = self.user_accounts
            .values()
            .fold(HashMap::new(), |mut acc, account| {
                *acc.entry(account.tier.clone()).or_insert(0) += 1;
                acc
            });

        stats.insert("total_users".to_string(), serde_json::Value::from(total_users));
        stats.insert("total_pending_revenue".to_string(), serde_json::Value::from(total_pending_revenue));
        stats.insert("tier_distribution".to_string(), serde_json::to_value(tier_distribution).unwrap());

        stats
    }
}