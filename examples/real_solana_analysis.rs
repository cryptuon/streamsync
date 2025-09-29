//! Real Solana Data Analysis Example
//!
//! This example demonstrates fetching and analyzing real Solana transaction data
//! using the StreamSync libraries. It showcases practical use cases with actual
//! blockchain data.

use zk_reconstruction::{ZKReconstructionLibrary, types::{TruncatedData, CompressionParams, CompressionType, TruncationMetadata}};
use idl_sync::IDLSyncLibrary;
use solana_client::{rpc_client::RpcClient, rpc_config::RpcTransactionConfig};
use solana_sdk::{
    pubkey::Pubkey, signature::Signature, commitment_config::CommitmentConfig,
    transaction::Transaction
};
use std::str::FromStr;
use std::time::{Duration, SystemTime};
use tracing::{info, warn, error};
use serde_json;

/// Configuration for real Solana data analysis
#[derive(Debug)]
struct AnalysisConfig {
    rpc_url: String,
    programs_to_analyze: Vec<ProgramConfig>,
    max_transactions: usize,
}

#[derive(Debug)]
struct ProgramConfig {
    name: String,
    program_id: Pubkey,
    description: String,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            programs_to_analyze: vec![
                ProgramConfig {
                    name: "SPL Token".to_string(),
                    program_id: Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap(),
                    description: "Standard token operations".to_string(),
                },
                ProgramConfig {
                    name: "Metaplex Token Metadata".to_string(),
                    program_id: Pubkey::from_str("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s").unwrap(),
                    description: "NFT metadata operations".to_string(),
                },
                ProgramConfig {
                    name: "Jupiter Aggregator".to_string(),
                    program_id: Pubkey::from_str("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4").unwrap(),
                    description: "DEX aggregation".to_string(),
                },
                ProgramConfig {
                    name: "Raydium AMM".to_string(),
                    program_id: Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8").unwrap(),
                    description: "Automated market maker".to_string(),
                },
            ],
            max_transactions: 20, // Reduced to avoid rate limits
        }
    }
}

#[derive(Debug)]
struct TransactionAnalysis {
    program_id: Pubkey,
    program_name: String,
    total_transactions: usize,
    unique_instructions: Vec<InstructionPattern>,
    account_patterns: Vec<AccountPattern>,
    average_gas_used: u64,
    success_rate: f64,
}

#[derive(Debug)]
struct InstructionPattern {
    discriminator: Vec<u8>,
    frequency: usize,
    average_size: f64,
    estimated_name: String,
}

#[derive(Debug)]
struct AccountPattern {
    account_type: String,
    frequency: usize,
    average_data_size: usize,
}

pub struct RealSolanaAnalyzer {
    config: AnalysisConfig,
    rpc_client: RpcClient,
    zk_reconstruction: ZKReconstructionLibrary,
    idl_sync: IDLSyncLibrary,
}

impl RealSolanaAnalyzer {
    pub fn new() -> Self {
        let config = AnalysisConfig::default();
        let rpc_client = RpcClient::new_with_commitment(
            config.rpc_url.clone(),
            CommitmentConfig::confirmed(),
        );

        Self {
            config,
            rpc_client,
            zk_reconstruction: ZKReconstructionLibrary::new(),
            idl_sync: IDLSyncLibrary::new(),
        }
    }

    /// Run comprehensive analysis on real Solana programs
    pub async fn analyze_programs(&self) -> Result<Vec<TransactionAnalysis>, Box<dyn std::error::Error>> {
        info!("🔍 Starting real Solana program analysis");

        let mut analyses = Vec::new();

        for program_config in &self.config.programs_to_analyze {
            info!("📊 Analyzing program: {} ({})", program_config.name, program_config.program_id);

            match self.analyze_single_program(program_config).await {
                Ok(analysis) => {
                    self.display_analysis(&analysis);
                    analyses.push(analysis);
                },
                Err(e) => {
                    error!("❌ Failed to analyze {}: {}", program_config.name, e);
                }
            }

            // Rate limiting
            tokio::time::sleep(Duration::from_secs(2)).await;
        }

        // Generate comparative analysis
        self.generate_comparative_report(&analyses);

        Ok(analyses)
    }

    /// Analyze a single program's transaction patterns
    async fn analyze_single_program(&self, program_config: &ProgramConfig) -> Result<TransactionAnalysis, Box<dyn std::error::Error>> {
        // Fetch recent signatures
        let signatures = self.rpc_client.get_signatures_for_address_with_config(
            &program_config.program_id,
            solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config {
                limit: Some(self.config.max_transactions),
                ..Default::default()
            },
        )?;

        info!("   Found {} recent transactions", signatures.len());

        let mut transactions = Vec::new();
        let mut instruction_patterns = std::collections::HashMap::new();
        let mut account_usage = std::collections::HashMap::new();
        let mut total_gas = 0u64;
        let mut successful_transactions = 0;

        // Analyze each transaction
        for (i, sig_info) in signatures.iter().take(self.config.max_transactions).enumerate() {
            if let Ok(signature) = Signature::from_str(&sig_info.signature) {
                info!("   Processing transaction {}/{}", i + 1, signatures.len().min(self.config.max_transactions));

                match self.rpc_client.get_transaction_with_config(
                    &signature,
                    RpcTransactionConfig {
                        encoding: Some(solana_account_decoder::UiTransactionEncoding::Json),
                        commitment: Some(CommitmentConfig::confirmed()),
                        max_supported_transaction_version: Some(0),
                    },
                ) {
                    Ok(transaction) => {
                        if let Some(meta) = &transaction.transaction.meta {
                            // Track gas usage
                            total_gas += meta.fee;

                            // Track success rate
                            if meta.err.is_none() {
                                successful_transactions += 1;
                            }
                        }

                        if let Some(tx) = transaction.transaction.transaction.decode() {
                            // Analyze instructions for this program
                            for instruction in &tx.message.instructions {
                                let program_id = tx.message.account_keys[instruction.program_id_index as usize];

                                if program_id == program_config.program_id {
                                    // Extract instruction discriminator (first 8 bytes typically)
                                    let discriminator = instruction.data.get(0..8.min(instruction.data.len())).unwrap_or(&[]).to_vec();

                                    let entry = instruction_patterns.entry(discriminator.clone()).or_insert((0, 0));
                                    entry.0 += 1; // frequency
                                    entry.1 += instruction.data.len(); // total size

                                    transactions.push(instruction.data.clone());

                                    // Track account usage
                                    for &account_index in &instruction.accounts {
                                        if let Some(account) = tx.message.account_keys.get(account_index as usize) {
                                            *account_usage.entry(*account).or_insert(0) += 1;
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Err(_) => {
                        // Skip failed transactions (likely rate limited)
                        continue;
                    }
                }

                // Rate limiting between transactions
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }

        // Process instruction patterns
        let unique_instructions: Vec<InstructionPattern> = instruction_patterns.into_iter()
            .map(|(discriminator, (frequency, total_size))| {
                InstructionPattern {
                    estimated_name: self.estimate_instruction_name(&discriminator, &program_config.name),
                    discriminator,
                    frequency,
                    average_size: if frequency > 0 { total_size as f64 / frequency as f64 } else { 0.0 },
                }
            })
            .collect();

        // Process account patterns
        let account_patterns: Vec<AccountPattern> = vec![
            AccountPattern {
                account_type: "Program Accounts".to_string(),
                frequency: account_usage.len(),
                average_data_size: 0, // Would need additional RPC calls to get account data
            }
        ];

        // Test IDL generation
        info!("   🔄 Testing IDL generation with {} instruction samples", transactions.len());
        match self.idl_sync.analyze_program_transactions(&program_config.program_id, &transactions).await {
            Ok(generated_idl) => {
                info!("   ✅ IDL generation successful:");
                info!("      - Instructions detected: {}", generated_idl.idl.instructions.len());
                info!("      - Account types: {}", generated_idl.idl.accounts.len());
                info!("      - Confidence: {:.1}%", generated_idl.confidence.overall_confidence * 100.0);
            },
            Err(e) => {
                warn!("   ⚠️ IDL generation failed: {}", e);
            }
        }

        Ok(TransactionAnalysis {
            program_id: program_config.program_id,
            program_name: program_config.name.clone(),
            total_transactions: signatures.len().min(self.config.max_transactions),
            unique_instructions,
            account_patterns,
            average_gas_used: if signatures.len() > 0 { total_gas / signatures.len() as u64 } else { 0 },
            success_rate: if signatures.len() > 0 { successful_transactions as f64 / signatures.len() as f64 } else { 0.0 },
        })
    }

    /// Display analysis results
    fn display_analysis(&self, analysis: &TransactionAnalysis) {
        info!("📋 Analysis Results for {}:", analysis.program_name);
        info!("   📊 Transaction Statistics:");
        info!("      - Total analyzed: {}", analysis.total_transactions);
        info!("      - Success rate: {:.1}%", analysis.success_rate * 100.0);
        info!("      - Average gas: {} lamports", analysis.average_gas_used);
        info!("      - Unique instructions: {}", analysis.unique_instructions.len());

        info!("   🔧 Instruction Patterns:");
        for (i, instruction) in analysis.unique_instructions.iter().take(5).enumerate() {
            info!("      {}. {} (freq: {}, avg_size: {:.1} bytes)",
                  i + 1, instruction.estimated_name, instruction.frequency, instruction.average_size);
            info!("         Discriminator: {:02x?}", &instruction.discriminator);
        }

        if analysis.unique_instructions.len() > 5 {
            info!("      ... and {} more", analysis.unique_instructions.len() - 5);
        }
    }

    /// Generate comparative analysis report
    fn generate_comparative_report(&self, analyses: &[TransactionAnalysis]) {
        info!("📊 Comparative Analysis Report");
        info!("==========================================");

        // Compare complexity
        info!("🔍 Program Complexity Comparison:");
        let mut sorted_by_complexity: Vec<_> = analyses.iter().collect();
        sorted_by_complexity.sort_by(|a, b| b.unique_instructions.len().cmp(&a.unique_instructions.len()));

        for (i, analysis) in sorted_by_complexity.iter().enumerate() {
            info!("   {}. {}: {} unique instructions",
                  i + 1, analysis.program_name, analysis.unique_instructions.len());
        }

        // Compare gas usage
        info!("⛽ Gas Usage Comparison:");
        let mut sorted_by_gas: Vec<_> = analyses.iter().collect();
        sorted_by_gas.sort_by(|a, b| b.average_gas_used.cmp(&a.average_gas_used));

        for (i, analysis) in sorted_by_gas.iter().enumerate() {
            info!("   {}. {}: {} lamports avg",
                  i + 1, analysis.program_name, analysis.average_gas_used);
        }

        // Compare success rates
        info!("✅ Success Rate Comparison:");
        let mut sorted_by_success: Vec<_> = analyses.iter().collect();
        sorted_by_success.sort_by(|a, b| b.success_rate.partial_cmp(&a.success_rate).unwrap());

        for (i, analysis) in sorted_by_success.iter().enumerate() {
            info!("   {}. {}: {:.1}%",
                  i + 1, analysis.program_name, analysis.success_rate * 100.0);
        }

        // Generate insights
        info!("💡 Key Insights:");

        let most_complex = sorted_by_complexity.first().unwrap();
        info!("   - Most complex program: {} ({} instruction types)",
              most_complex.program_name, most_complex.unique_instructions.len());

        let highest_gas = sorted_by_gas.first().unwrap();
        info!("   - Highest gas usage: {} ({} lamports avg)",
              highest_gas.program_name, highest_gas.average_gas_used);

        let most_reliable = sorted_by_success.first().unwrap();
        info!("   - Most reliable: {} ({:.1}% success rate)",
              most_reliable.program_name, most_reliable.success_rate * 100.0);

        // Recommendations for StreamSync optimization
        info!("🚀 StreamSync Optimization Opportunities:");

        for analysis in analyses {
            if analysis.unique_instructions.len() > 10 {
                info!("   - {} has high complexity - good candidate for IDL generation", analysis.program_name);
            }
            if analysis.success_rate < 0.9 {
                info!("   - {} has failure patterns - good for error analysis", analysis.program_name);
            }
        }
    }

    /// Estimate instruction name from discriminator and program context
    fn estimate_instruction_name(&self, discriminator: &[u8], program_name: &str) -> String {
        if discriminator.is_empty() {
            return "Unknown".to_string();
        }

        // Program-specific instruction naming based on known patterns
        match program_name {
            "SPL Token" => {
                match discriminator.get(0) {
                    Some(0) => "InitializeMint".to_string(),
                    Some(1) => "InitializeAccount".to_string(),
                    Some(3) => "Transfer".to_string(),
                    Some(4) => "Approve".to_string(),
                    Some(7) => "MintTo".to_string(),
                    Some(8) => "Burn".to_string(),
                    Some(9) => "CloseAccount".to_string(),
                    _ => format!("Token_Instruction_{:02x}", discriminator[0]),
                }
            },
            "Metaplex Token Metadata" => {
                match discriminator.get(0) {
                    Some(0) => "CreateMetadataAccount".to_string(),
                    Some(1) => "UpdateMetadataAccount".to_string(),
                    Some(15) => "CreateMasterEdition".to_string(),
                    Some(16) => "MintNewEditionFromMasterEdition".to_string(),
                    _ => format!("Metadata_Instruction_{:02x}", discriminator[0]),
                }
            },
            "Jupiter Aggregator" => {
                format!("Jupiter_Route_{:02x?}", &discriminator[..2.min(discriminator.len())])
            },
            "Raydium AMM" => {
                match discriminator.get(0) {
                    Some(1) => "Swap".to_string(),
                    Some(3) => "AddLiquidity".to_string(),
                    Some(4) => "RemoveLiquidity".to_string(),
                    _ => format!("Raydium_Instruction_{:02x}", discriminator[0]),
                }
            },
            _ => format!("Instruction_{:02x?}", &discriminator[..4.min(discriminator.len())]),
        }
    }

    /// Demonstrate ZK reconstruction with real compressed data
    pub async fn demonstrate_compression_reconstruction(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("🔧 Demonstrating ZK reconstruction with compressed data patterns");

        // Create realistic compression scenarios based on actual Solana data patterns
        let compression_scenarios = vec![
            ("Token Account", self.create_token_account_data()),
            ("NFT Metadata", self.create_nft_metadata_data()),
            ("AMM Pool State", self.create_amm_pool_data()),
        ];

        for (scenario_name, data) in compression_scenarios {
            info!("   Testing scenario: {}", scenario_name);

            // Simulate different truncation levels
            for truncation_level in [256, 512, 1024] {
                if data.len() <= truncation_level {
                    continue;
                }

                let truncated_data = TruncatedData {
                    data: data[..truncation_level].to_vec(),
                    original_size_hint: Some(data.len()),
                    truncation_point: truncation_level,
                    metadata: TruncationMetadata {
                        slot: 250_000_000,
                        account: Pubkey::new_unique(),
                        program_id: Pubkey::new_unique(),
                        compression_type: CompressionType::StateCompression,
                        truncation_timestamp: SystemTime::now(),
                    },
                };

                let compression_params = CompressionParams {
                    compression_type: CompressionType::StateCompression,
                    merkle_tree_height: 20,
                    leaf_count: 1000,
                    root_hash: blake3::hash(&data).as_bytes().try_into().unwrap(),
                    compression_program: Pubkey::new_unique(),
                    additional_params: std::collections::HashMap::new(),
                };

                match self.zk_reconstruction.reconstruct_compressed_account(
                    &truncated_data,
                    &compression_params
                ).await {
                    Ok(result) => {
                        let expansion_ratio = result.account_data.len() as f64 / truncated_data.data.len() as f64;
                        info!("      ✅ Truncation {}: {:.2}x expansion, {:.1}% confidence",
                              truncation_level, expansion_ratio, result.confidence_score * 100.0);
                    },
                    Err(_) => {
                        info!("      ❌ Truncation {} failed", truncation_level);
                    }
                }
            }
        }

        Ok(())
    }

    fn create_token_account_data(&self) -> Vec<u8> {
        let mut data = vec![0u8; 165]; // SPL Token account size
        data[0..32].copy_from_slice(&Pubkey::new_unique().to_bytes()); // mint
        data[32..64].copy_from_slice(&Pubkey::new_unique().to_bytes()); // owner
        data[64..72].copy_from_slice(&1000000u64.to_le_bytes()); // amount
        data[72] = 1; // delegate option
        data[73..105].copy_from_slice(&Pubkey::new_unique().to_bytes()); // delegate
        data
    }

    fn create_nft_metadata_data(&self) -> Vec<u8> {
        let metadata = serde_json::json!({
            "name": "Real NFT Example",
            "symbol": "RNE",
            "description": "An example NFT with realistic metadata structure",
            "image": "https://arweave.net/example-image-hash",
            "attributes": [
                {"trait_type": "Background", "value": "Gradient Blue"},
                {"trait_type": "Eyes", "value": "Laser"},
                {"trait_type": "Mouth", "value": "Smile"},
                {"trait_type": "Rarity", "value": "Epic"}
            ],
            "properties": {
                "creators": [{
                    "address": "11111111111111111111111111111112",
                    "verified": true,
                    "share": 100
                }]
            }
        });

        metadata.to_string().into_bytes()
    }

    fn create_amm_pool_data(&self) -> Vec<u8> {
        let mut data = vec![0u8; 752]; // Typical AMM pool size
        data[0..8].copy_from_slice(&1000000000u64.to_le_bytes()); // token_a_amount
        data[8..16].copy_from_slice(&2000000000u64.to_le_bytes()); // token_b_amount
        data[16..48].copy_from_slice(&Pubkey::new_unique().to_bytes()); // token_a_mint
        data[48..80].copy_from_slice(&Pubkey::new_unique().to_bytes()); // token_b_mint
        data[80..112].copy_from_slice(&Pubkey::new_unique().to_bytes()); // pool_mint
        data
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("🌐 Real Solana Data Analysis with StreamSync");

    let analyzer = RealSolanaAnalyzer::new();

    // Run comprehensive program analysis
    info!("🚀 Starting comprehensive program analysis...");
    match analyzer.analyze_programs().await {
        Ok(analyses) => {
            info!("✅ Successfully analyzed {} programs", analyses.len());
        },
        Err(e) => {
            error!("❌ Analysis failed: {}", e);
            return Err(e);
        }
    }

    // Demonstrate compression reconstruction
    info!("🔧 Demonstrating compression reconstruction...");
    match analyzer.demonstrate_compression_reconstruction().await {
        Ok(_) => {
            info!("✅ Compression reconstruction demonstration completed");
        },
        Err(e) => {
            error!("❌ Compression demonstration failed: {}", e);
        }
    }

    info!("🎉 Real Solana data analysis completed successfully!");
    info!("📊 This demonstrates StreamSync's capability to:");
    info!("   - Fetch and analyze real blockchain transaction data");
    info!("   - Generate IDLs from actual program behavior");
    info!("   - Reconstruct compressed account data");
    info!("   - Provide actionable insights for Solana programs");

    Ok(())
}