use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{info, Level};
use tracing_subscriber;

mod node;
mod config;
mod consensus;
mod query_router;
mod economics;
mod gateway;
mod light_client;
mod rate_limiter;
mod revenue_sharing;

use crate::node::StreamSyncNode;
use crate::config::NodeConfig;
use crate::economics::{EconomicsManager, PaymentToken};
use crate::gateway::{PaymentGateway, GatewayConfig};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Parser)]
#[command(name = "streamsync")]
#[command(about = "StreamSync - Decentralized Solana Transaction Indexing Network")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a StreamSync node
    Start {
        /// Configuration file path
        #[arg(short, long, default_value = "node.toml")]
        config: String,

        /// Node role (primary, secondary, or observer)
        #[arg(short, long, default_value = "secondary")]
        role: String,

        /// Enable debug logging
        #[arg(short, long)]
        debug: bool,
    },
    /// Start payment gateway
    Gateway {
        /// Gateway configuration file path
        #[arg(short, long, default_value = "gateway.toml")]
        config: String,

        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Enable debug logging
        #[arg(short, long)]
        debug: bool,
    },
    /// Initialize a new node configuration
    Init {
        /// Output configuration file path
        #[arg(short, long, default_value = "node.toml")]
        output: String,
    },
    /// Show node status
    Status {
        /// Node RPC endpoint
        #[arg(short, long, default_value = "http://localhost:8080")]
        endpoint: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start { config, role, debug } => {
            // Initialize logging
            let level = if debug { Level::DEBUG } else { Level::INFO };
            tracing_subscriber::fmt()
                .with_max_level(level)
                .init();

            info!("Starting StreamSync node...");
            info!("Role: {}", role);
            info!("Config: {}", config);

            // Load configuration
            let node_config = NodeConfig::load(&config).await?;

            // Create and start node
            let mut node = StreamSyncNode::new(node_config, role).await?;
            node.start().await?;

            // Keep running until interrupted
            tokio::signal::ctrl_c().await?;
            info!("Shutting down...");
            node.shutdown().await?;
        }
        Commands::Gateway { config, port, debug } => {
            // Initialize logging
            let level = if debug { Level::DEBUG } else { Level::INFO };
            tracing_subscriber::fmt()
                .with_max_level(level)
                .init();

            info!("Starting StreamSync Payment Gateway...");
            info!("Port: {}", port);
            info!("Config: {}", config);

            // Create economics manager
            let (economics_manager, _economics_receiver) = EconomicsManager::new();
            let economics_manager = Arc::new(RwLock::new(economics_manager));

            // Create gateway configuration
            let gateway_config = GatewayConfig {
                stripe_secret_key: std::env::var("STRIPE_SECRET_KEY").ok(),
                solana_rpc_url: std::env::var("SOLANA_RPC_URL")
                    .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string()),
                treasury_wallet: std::env::var("TREASURY_WALLET")
                    .unwrap_or_else(|_| "StreamSyncTreasury1111111111111111111111".to_string()),
                supported_tokens: vec![
                    PaymentToken::STRM,
                    PaymentToken::SOL,
                    PaymentToken::USDC,
                ],
                minimum_payment_usd: 1.0,
            };

            // Create payment gateway
            let gateway = Arc::new(PaymentGateway::new(gateway_config, economics_manager));

            // Create router
            let app = PaymentGateway::create_router(gateway);

            // Start server
            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
            info!("Payment gateway listening on port {}", port);

            axum::serve(listener, app).await?;
        }
        Commands::Init { output } => {
            info!("Initializing new node configuration: {}", output);
            NodeConfig::create_default(&output).await?;
            println!("Configuration created at: {}", output);
        }
        Commands::Status { endpoint } => {
            info!("Checking node status at: {}", endpoint);
            println!("Status endpoint: {}", endpoint);
            println!("Note: Status check via HTTP endpoint not yet implemented");
            println!("Status information is available in the node logs during runtime");
        }
    }

    Ok(())
}