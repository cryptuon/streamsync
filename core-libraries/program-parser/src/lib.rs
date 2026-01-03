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
//! ```rust
//! use program_parser::ProgramParser;
//!
//! // Create a program parser
//! let parser = ProgramParser::new();
//! ```

pub mod error;
pub mod types;
pub mod parsers;
pub mod detector;
pub mod cache;

pub use error::{ParseError, ParseResult as Result};
pub use types::*;
pub use parsers::ProgramParser;