//! # Automatic Program-Specific Parser
//!
//! A comprehensive library for automatically parsing and interpreting Solana program data.
//! This module provides intelligent parsing for major Solana programs including SPL Token,
//! Metaplex, Jupiter, Raydium, and many others.
//!
//! ## Overview
//!
//! Traditional Solana data analysis requires manual parsing or pre-built parsers for each
//! program. This library provides automatic detection and parsing with rich metadata extraction,
//! making it easy to understand any transaction or account data.
//!
//! ## Key Features
//!
//! - **Auto-Detection**: Automatically identify program types from account/transaction data
//! - **Rich Parsing**: Extract meaningful data structures and metadata
//! - **Performance**: High-speed parsing with intelligent caching
//! - **Extensible**: Easy to add new program parsers
//! - **Battle-tested**: Parsers for 20+ major Solana programs
//!
//! ## Supported Programs
//!
//! - **SPL Token**: Transfer, mint, burn, approve operations
//! - **Metaplex**: NFT metadata, collection management, royalties
//! - **Jupiter**: DEX aggregation, swap routing, price discovery
//! - **Raydium**: AMM pools, liquidity provision, farming
//! - **Orca**: Concentrated liquidity, whirlpools, position management
//! - **Serum**: Order book DEX, market making, trade execution
//! - **Solend**: Lending, borrowing, liquidations, collateral management
//! - **Marinade**: Liquid staking, mSOL operations
//! - **Anchor**: Program framework, IDL-based parsing
//! - And many more...
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use program_parser::{ProgramParser, ParseResult};
//! use solana_sdk::pubkey::Pubkey;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let parser = ProgramParser::new();
//!
//! // Parse transaction data
//! let transaction_data = vec![/* raw transaction bytes */];
//! let parsed = parser.parse_transaction_data(&transaction_data).await?;
//!
//! match parsed {
//!     ParseResult::SplToken(token_data) => {
//!         println!("SPL Token transfer: {} tokens", token_data.amount);
//!         println!("From: {} To: {}", token_data.from, token_data.to);
//!     },
//!     ParseResult::Metaplex(nft_data) => {
//!         println!("NFT: {} by {}", nft_data.name, nft_data.creator);
//!         println!("URI: {}", nft_data.uri);
//!     },
//!     ParseResult::Jupiter(swap_data) => {
//!         println!("Swap: {} {} for {} {}",
//!                  swap_data.input_amount, swap_data.input_mint,
//!                  swap_data.output_amount, swap_data.output_mint);
//!     },
//!     _ => println!("Parsed other program type"),
//! }
//! # Ok(())
//! # }
//! ```

pub mod error;
pub mod types;
pub mod parsers;
pub mod detector;
pub mod cache;

pub use error::{ParseError, ParseResult as Result};
pub use types::*;
pub use parsers::ProgramParser;