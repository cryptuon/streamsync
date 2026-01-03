//! Program-specific parsers

use crate::{
    error::ParseError,
    types::*,
    detector::ProgramDetector,
    cache::ParseCache,
};
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use tracing::{debug, info, warn};

pub mod spl_token;
pub mod metaplex;
pub mod jupiter;
pub mod raydium;
pub mod orca;
pub mod serum;
pub mod solend;
pub mod marinade;
pub mod anchor;

/// Main program parser that orchestrates all specific parsers
pub struct ProgramParser {
    detector: ProgramDetector,
    cache: Arc<ParseCache>,
    config: ParseConfig,
    stats: ParseStats,
}

impl ProgramParser {
    pub fn new() -> Self {
        Self::with_config(ParseConfig::default())
    }

    pub fn with_config(config: ParseConfig) -> Self {
        Self {
            detector: ProgramDetector::new(),
            cache: Arc::new(ParseCache::new(config.max_cache_size)),
            config,
            stats: ParseStats {
                total_parsed: 0,
                successful_parses: 0,
                failed_parses: 0,
                cache_hits: 0,
                cache_misses: 0,
                average_parse_time_ms: 0.0,
                programs_detected: std::collections::HashMap::new(),
                last_updated: chrono::Utc::now(),
            },
        }
    }

    /// Parse transaction instruction data
    pub async fn parse_transaction_data(&mut self, data: &[u8]) -> Result<crate::types::ParseResult, ParseError> {
        let start_time = std::time::Instant::now();

        // This is a simplified parser - in practice you'd need full transaction context
        // For now, we'll demonstrate with mock data

        let result = self.parse_mock_data(data).await;

        let elapsed = start_time.elapsed();
        self.update_stats(&result, elapsed);

        result
    }

    /// Parse program instruction with full context
    pub async fn parse_instruction(
        &mut self,
        program_id: &Pubkey,
        instruction_data: &[u8],
        account_keys: &[Pubkey],
    ) -> Result<crate::types::ParseResult, ParseError> {
        let start_time = std::time::Instant::now();

        debug!(
            "Parsing instruction for program {} with {} bytes of data",
            program_id,
            instruction_data.len()
        );

        // Check cache first
        if self.config.cache_results {
            let cache_key = self.generate_cache_key(program_id, instruction_data, account_keys);
            if let Some(cached_result) = self.cache.get(&cache_key) {
                self.stats.cache_hits += 1;
                debug!("Cache hit for instruction parsing");
                return Ok(cached_result);
            }
            self.stats.cache_misses += 1;
        }

        // Detect program type
        let detection = self.detector.detect_program(program_id, instruction_data, account_keys)?;

        info!(
            "Detected program type: {:?} with confidence {:.2}",
            detection.program_type, detection.confidence
        );

        // Parse based on detected type
        let parse_result = match detection.program_type {
            ProgramType::SplToken => {
                spl_token::SplTokenParser::new().parse(program_id, instruction_data, account_keys).await
            }
            ProgramType::Metaplex => {
                metaplex::MetaplexParser::new().parse(program_id, instruction_data, account_keys).await
            }
            ProgramType::Jupiter => {
                jupiter::JupiterParser::new().parse(program_id, instruction_data, account_keys).await
            }
            ProgramType::Raydium => {
                raydium::RaydiumParser::new().parse(program_id, instruction_data, account_keys).await
            }
            ProgramType::Orca => {
                orca::OrcaParser::new().parse(program_id, instruction_data, account_keys).await
            }
            ProgramType::Serum => {
                serum::SerumParser::new().parse(program_id, instruction_data, account_keys).await
            }
            ProgramType::Solend => {
                solend::SolendParser::new().parse(program_id, instruction_data, account_keys).await
            }
            ProgramType::Marinade => {
                marinade::MarinadeParser::new().parse(program_id, instruction_data, account_keys).await
            }
            ProgramType::Anchor => {
                anchor::AnchorParser::new().parse(program_id, instruction_data, account_keys).await
            }
            ProgramType::System => {
                Ok(crate::types::ParseResult::Unknown(UnknownData {
                    program_id: *program_id,
                    instruction_data: instruction_data.to_vec(),
                    accounts: account_keys.to_vec(),
                    suspected_program_type: Some("System".to_string()),
                    confidence: 1.0,
                    raw_bytes: instruction_data.to_vec(),
                    parsing_hints: vec!["System program operations".to_string()],
                }))
            }
            ProgramType::Unknown => {
                Ok(crate::types::ParseResult::Unknown(UnknownData {
                    program_id: *program_id,
                    instruction_data: instruction_data.to_vec(),
                    accounts: account_keys.to_vec(),
                    suspected_program_type: None,
                    confidence: 0.0,
                    raw_bytes: instruction_data.to_vec(),
                    parsing_hints: vec!["Could not determine program type".to_string()],
                }))
            }
        };

        // Cache successful results
        if self.config.cache_results && parse_result.is_ok() {
            let cache_key = self.generate_cache_key(program_id, instruction_data, account_keys);
            if let Ok(ref result) = parse_result {
                self.cache.insert(cache_key, result.clone());
            }
        }

        let elapsed = start_time.elapsed();
        self.update_stats(&parse_result, elapsed);

        parse_result
    }

    /// Parse account data for supported programs
    pub async fn parse_account_data(
        &mut self,
        program_id: &Pubkey,
        account_data: &[u8],
    ) -> Result<crate::types::ParseResult, ParseError> {
        debug!(
            "Parsing account data for program {} with {} bytes",
            program_id,
            account_data.len()
        );

        // Account parsing is program-specific
        match self.detector.known_programs.get(program_id) {
            Some(ProgramType::SplToken) => {
                spl_token::SplTokenParser::new().parse_account(account_data).await
            }
            Some(ProgramType::Metaplex) => {
                metaplex::MetaplexParser::new().parse_account(account_data).await
            }
            _ => {
                warn!("Account parsing not supported for program: {}", program_id);
                Ok(crate::types::ParseResult::Unknown(UnknownData {
                    program_id: *program_id,
                    instruction_data: vec![],
                    accounts: vec![],
                    suspected_program_type: None,
                    confidence: 0.0,
                    raw_bytes: account_data.to_vec(),
                    parsing_hints: vec!["Account parsing not implemented for this program".to_string()],
                }))
            }
        }
    }

    /// Get parsing statistics
    pub fn get_stats(&self) -> &ParseStats {
        &self.stats
    }

    /// Clear the parsing cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    // Private helper methods

    async fn parse_mock_data(&mut self, data: &[u8]) -> Result<crate::types::ParseResult, ParseError> {
        // This is a mock implementation for demonstration
        // In practice, you'd extract program_id and instruction data from the transaction

        if data.is_empty() {
            return Err(ParseError::insufficient_data(1, 0));
        }

        // Mock: assume first byte indicates program type
        match data[0] {
            0 => Ok(crate::types::ParseResult::SplToken(self.create_mock_spl_token_data())),
            1 => Ok(crate::types::ParseResult::Metaplex(self.create_mock_metaplex_data())),
            2 => Ok(crate::types::ParseResult::Jupiter(self.create_mock_jupiter_data())),
            _ => Ok(crate::types::ParseResult::Unknown(UnknownData {
                program_id: Pubkey::new_unique(),
                instruction_data: data.to_vec(),
                accounts: vec![],
                suspected_program_type: None,
                confidence: 0.0,
                raw_bytes: data.to_vec(),
                parsing_hints: vec!["Unknown program type".to_string()],
            }))
        }
    }

    fn generate_cache_key(
        &self,
        program_id: &Pubkey,
        instruction_data: &[u8],
        account_keys: &[Pubkey],
    ) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        program_id.hash(&mut hasher);
        instruction_data.hash(&mut hasher);
        account_keys.hash(&mut hasher);
        format!("parse_{}", hasher.finish())
    }

    fn update_stats(&mut self, result: &Result<crate::types::ParseResult, ParseError>, elapsed: std::time::Duration) {
        self.stats.total_parsed += 1;

        match result {
            Ok(parse_result) => {
                self.stats.successful_parses += 1;

                let program_type = match parse_result {
                    crate::types::ParseResult::SplToken(_) => ProgramType::SplToken,
                    crate::types::ParseResult::Metaplex(_) => ProgramType::Metaplex,
                    crate::types::ParseResult::Jupiter(_) => ProgramType::Jupiter,
                    crate::types::ParseResult::Raydium(_) => ProgramType::Raydium,
                    crate::types::ParseResult::Orca(_) => ProgramType::Orca,
                    crate::types::ParseResult::Serum(_) => ProgramType::Serum,
                    crate::types::ParseResult::Solend(_) => ProgramType::Solend,
                    crate::types::ParseResult::Marinade(_) => ProgramType::Marinade,
                    crate::types::ParseResult::Anchor(_) => ProgramType::Anchor,
                    crate::types::ParseResult::Unknown(_) => ProgramType::Unknown,
                };

                *self.stats.programs_detected.entry(program_type).or_insert(0) += 1;
            }
            Err(_) => {
                self.stats.failed_parses += 1;
            }
        }

        // Update average parse time
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
        self.stats.average_parse_time_ms =
            (self.stats.average_parse_time_ms * (self.stats.total_parsed - 1) as f64 + elapsed_ms)
            / self.stats.total_parsed as f64;

        self.stats.last_updated = chrono::Utc::now();
    }

    // Mock data creation methods for demonstration
    fn create_mock_spl_token_data(&self) -> SplTokenData {
        SplTokenData {
            operation_type: SplTokenOperation::Transfer,
            mint: Pubkey::new_unique(),
            amount: 1000000,
            decimals: 6,
            from: Some(Pubkey::new_unique()),
            to: Some(Pubkey::new_unique()),
            authority: Some(Pubkey::new_unique()),
            multisig_signers: vec![],
            parsed_amount: rust_decimal::Decimal::new(1000000, 6),
            symbol: Some("USDC".to_string()),
            metadata: SplTokenMetadata {
                name: Some("USD Coin".to_string()),
                symbol: Some("USDC".to_string()),
                logo_uri: Some("https://example.com/usdc.png".to_string()),
                total_supply: Some(1000000000000),
                is_initialized: true,
                freeze_authority: None,
                mint_authority: Some(Pubkey::new_unique()),
            },
        }
    }

    fn create_mock_metaplex_data(&self) -> MetaplexData {
        MetaplexData {
            operation_type: MetaplexOperation::MintNft,
            mint: Pubkey::new_unique(),
            metadata_account: Pubkey::new_unique(),
            name: "Cool NFT #123".to_string(),
            symbol: "COOL".to_string(),
            uri: "https://example.com/nft/123.json".to_string(),
            creators: vec![Creator {
                address: Pubkey::new_unique(),
                verified: true,
                share: 100,
            }],
            seller_fee_basis_points: 500,
            collection: None,
            uses: None,
            is_compressed: false,
            tree_authority: None,
            leaf_id: None,
            attributes: std::collections::HashMap::new(),
        }
    }

    fn create_mock_jupiter_data(&self) -> JupiterData {
        JupiterData {
            operation_type: JupiterOperation::Swap,
            input_mint: Pubkey::new_unique(),
            output_mint: Pubkey::new_unique(),
            input_amount: 1000000,
            output_amount: 995000,
            minimum_output_amount: 990000,
            slippage_bps: 50,
            route: vec![],
            fees: vec![],
            price_impact_pct: Some(0.1),
            user_wallet: Pubkey::new_unique(),
            referrer: None,
        }
    }
}

impl Default for ProgramParser {
    fn default() -> Self {
        Self::new()
    }
}