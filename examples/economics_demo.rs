//! StreamSync Economics System Demo

use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

use streamsync_node::economics::{EconomicsManager, PaymentToken};
use streamsync_node::rate_limiter::RateLimiter;
use streamsync_node::revenue_sharing::{RevenueSharingManager, NodePerformance, RevenueModel};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("🚀 StreamSync Economics System Demo");
    println!("=====================================\n");

    // 1. Initialize Economics Manager
    println!("1. Initializing Economics Manager...");
    let (mut economics, economics_receiver) = EconomicsManager::new();

    // Create sample users
    let user1 = economics.create_account("strm_test_key_1".to_string(), "basic".to_string()).await?;
    let user2 = economics.create_account("strm_test_key_2".to_string(), "pro".to_string()).await?;
    println!("   ✓ Created users: {} (basic), {} (pro)", user1, user2);

    // Add credits
    economics.add_credits(user1, 100.0, PaymentToken::STRM).await?;
    economics.add_credits(user2, 50.0, PaymentToken::SOL).await?;
    println!("   ✓ Added credits to users");

    // 2. Test Rate Limiting
    println!("\n2. Testing Rate Limiting...");
    let rate_limiter = RateLimiter::new();

    rate_limiter.init_user(user1, "basic").await?;
    rate_limiter.init_user(user2, "pro").await?;

    // Test requests for basic user
    for i in 1..=5 {
        let allowed = rate_limiter.try_request(user1, 1).await?;
        println!("   Request {} for basic user: {}", i, if allowed { "✓" } else { "✗" });
    }

    // Test requests for pro user
    for i in 1..=10 {
        let allowed = rate_limiter.try_request(user2, 1).await?;
        if i % 2 == 0 {
            println!("   Request {} for pro user: {}", i, if allowed { "✓" } else { "✗" });
        }
    }

    // 3. Test Request Charging
    println!("\n3. Testing Request Charging...");

    // Calculate costs for different query types
    let basic_cost = economics.calculate_request_cost("get_transaction", 1, 1.0, "basic", PaymentToken::STRM);
    let complex_cost = economics.calculate_request_cost("complex_analytics", 100, 2.0, "pro", PaymentToken::SOL);

    println!("   Basic query cost: {:.6} STRM", basic_cost.total_cost);
    println!("   Complex query cost: {:.6} SOL", complex_cost.total_cost);

    // Charge users
    let charged1 = economics.charge_request(user1, basic_cost).await?;
    let charged2 = economics.charge_request(user2, complex_cost).await?;

    println!("   Basic user charged: {}", charged1);
    println!("   Pro user charged: {}", charged2);

    // 4. Test Revenue Sharing
    println!("\n4. Testing Revenue Sharing...");

    let revenue_model = RevenueModel::PerformanceBased {
        request_weight: 0.4,
        uptime_weight: 0.3,
        response_time_weight: 0.2,
        consensus_weight: 0.05,
        storage_weight: 0.05,
    };

    let (mut revenue_manager, _revenue_receiver) = RevenueSharingManager::new(
        revenue_model,
        economics_receiver,
    );

    // Add mock node performance data
    let node1 = Uuid::new_v4();
    let node2 = Uuid::new_v4();

    let performance1 = NodePerformance {
        node_id: node1,
        requests_served: 1000,
        data_transferred_mb: 500.0,
        uptime_percentage: 99.5,
        average_response_time_ms: 120.0,
        successful_requests: 990,
        failed_requests: 10,
        consensus_participation: 0.95,
        storage_provided_gb: 100.0,
        bandwidth_provided_mbps: 1000.0,
    };

    let performance2 = NodePerformance {
        node_id: node2,
        requests_served: 2000,
        data_transferred_mb: 1000.0,
        uptime_percentage: 98.8,
        average_response_time_ms: 95.0,
        successful_requests: 1980,
        failed_requests: 20,
        consensus_participation: 0.98,
        storage_provided_gb: 200.0,
        bandwidth_provided_mbps: 1500.0,
    };

    revenue_manager.update_performance(node1, performance1).await?;
    revenue_manager.update_performance(node2, performance2).await?;
    println!("   ✓ Updated node performance metrics");

    // Distribute revenue
    let period_id = Uuid::new_v4();
    let mut total_revenue = HashMap::new();
    total_revenue.insert(PaymentToken::STRM, 10.0);

    let rewards = revenue_manager.distribute_revenue(period_id, total_revenue).await?;
    println!("   ✓ Distributed revenue to {} nodes", rewards.len());

    for reward in rewards {
        println!("     Node {}: {:.6} STRM (score: {:.3})",
            reward.node_id.to_string()[..8].to_string(),
            reward.reward_amount,
            reward.performance_score);
    }

    // 5. Show Network Statistics
    println!("\n5. Network Statistics:");
    let economics_stats = economics.get_network_stats();
    let revenue_stats = revenue_manager.get_network_stats().await;

    println!("   Economics:");
    for (key, value) in economics_stats {
        println!("     {}: {}", key, value);
    }

    println!("   Revenue Sharing:");
    for (key, value) in revenue_stats {
        println!("     {}: {}", key, value);
    }

    // 6. Show User Account Info
    println!("\n6. User Account Information:");
    if let Some(account1) = economics.get_account(&user1) {
        println!("   User 1 ({}):", account1.tier);
        println!("     Monthly usage: {}", account1.monthly_usage);
        for (token, balance) in &account1.credits {
            println!("     Balance: {:.6} {:?}", balance, token);
        }
    }

    if let Some(account2) = economics.get_account(&user2) {
        println!("   User 2 ({}):", account2.tier);
        println!("     Monthly usage: {}", account2.monthly_usage);
        for (token, balance) in &account2.credits {
            println!("     Balance: {:.6} {:?}", balance, token);
        }
    }

    println!("\n🎉 Economics Demo Complete!");
    println!("The StreamSync network now supports:");
    println!("  • Multi-token payments (STRM, SOL, USDC)");
    println!("  • Tiered pricing with rate limiting");
    println!("  • Performance-based revenue sharing");
    println!("  • Real-time usage tracking");
    println!("  • Automated reward distribution");

    Ok(())
}