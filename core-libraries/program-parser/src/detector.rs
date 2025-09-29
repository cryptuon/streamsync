//! Program type detection logic

use crate::{
    error::{ParseError, ParseResult},
    types::{DetectionResult, ProgramType, DetectionMethod},
};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;

/// Automatic program type detector
pub struct ProgramDetector {
    pub known_programs: HashMap<Pubkey, ProgramType>,
    signature_patterns: HashMap<Vec<u8>, ProgramType>,
}

impl ProgramDetector {
    pub fn new() -> Self {
        let mut detector = Self {
            known_programs: HashMap::new(),
            signature_patterns: HashMap::new(),
        };

        detector.initialize_known_programs();
        detector.initialize_signature_patterns();
        detector
    }

    /// Detect program type from program ID and instruction data
    pub fn detect_program(
        &self,
        program_id: &Pubkey,
        instruction_data: &[u8],
        account_keys: &[Pubkey],
    ) -> ParseResult<DetectionResult> {
        // Method 1: Check known program IDs first
        if let Some(program_type) = self.known_programs.get(program_id) {
            return Ok(DetectionResult {
                program_type: program_type.clone(),
                confidence: 1.0,
                detection_method: DetectionMethod::ProgramId,
                metadata: HashMap::new(),
            });
        }

        // Method 2: Check instruction signatures
        if let Some(program_type) = self.detect_by_signature(instruction_data) {
            return Ok(DetectionResult {
                program_type,
                confidence: 0.9,
                detection_method: DetectionMethod::DataSignature,
                metadata: HashMap::new(),
            });
        }

        // Method 3: Heuristic detection based on patterns
        if let Some(result) = self.detect_by_heuristics(program_id, instruction_data, account_keys) {
            return Ok(result);
        }

        // Method 4: Check if it's an Anchor program
        if self.is_anchor_program(instruction_data) {
            return Ok(DetectionResult {
                program_type: ProgramType::Anchor,
                confidence: 0.8,
                detection_method: DetectionMethod::InstructionPattern,
                metadata: HashMap::new(),
            });
        }

        // Default to unknown
        Ok(DetectionResult {
            program_type: ProgramType::Unknown,
            confidence: 0.0,
            detection_method: DetectionMethod::Heuristic,
            metadata: HashMap::new(),
        })
    }

    fn initialize_known_programs(&mut self) {
        // SPL Token Program
        self.known_programs.insert(
            Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap(),
            ProgramType::SplToken,
        );

        // SPL Token 2022
        self.known_programs.insert(
            Pubkey::from_str("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb").unwrap(),
            ProgramType::SplToken,
        );

        // Metaplex Token Metadata
        self.known_programs.insert(
            Pubkey::from_str("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s").unwrap(),
            ProgramType::Metaplex,
        );

        // Metaplex Bubblegum (Compressed NFTs)
        self.known_programs.insert(
            Pubkey::from_str("BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY").unwrap(),
            ProgramType::Metaplex,
        );

        // Jupiter Aggregator V6
        self.known_programs.insert(
            Pubkey::from_str("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4").unwrap(),
            ProgramType::Jupiter,
        );

        // Jupiter Aggregator V4
        self.known_programs.insert(
            Pubkey::from_str("JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB").unwrap(),
            ProgramType::Jupiter,
        );

        // Raydium AMM V4
        self.known_programs.insert(
            Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8").unwrap(),
            ProgramType::Raydium,
        );

        // Orca Whirlpool
        self.known_programs.insert(
            Pubkey::from_str("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc").unwrap(),
            ProgramType::Orca,
        );

        // Serum DEX V3
        self.known_programs.insert(
            Pubkey::from_str("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin").unwrap(),
            ProgramType::Serum,
        );

        // Solend Main Pool
        self.known_programs.insert(
            Pubkey::from_str("So1endDq2YkqhipRh3WViPa8hdiSpxWy6z3Z6tMCpAo").unwrap(),
            ProgramType::Solend,
        );

        // Marinade Finance
        self.known_programs.insert(
            Pubkey::from_str("MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD").unwrap(),
            ProgramType::Marinade,
        );

        // System Program
        self.known_programs.insert(
            Pubkey::from_str("11111111111111111111111111111111").unwrap(),
            ProgramType::System,
        );
    }

    fn initialize_signature_patterns(&mut self) {
        // SPL Token instruction discriminators
        self.signature_patterns.insert(vec![3], ProgramType::SplToken); // Transfer
        self.signature_patterns.insert(vec![7], ProgramType::SplToken); // MintTo
        self.signature_patterns.insert(vec![8], ProgramType::SplToken); // Burn

        // Metaplex instruction discriminators (first 8 bytes)
        self.signature_patterns.insert(
            vec![0x6a, 0x18, 0x53, 0x00, 0x7c, 0x05, 0x26, 0xd3],
            ProgramType::Metaplex,
        ); // CreateMetadataAccountV3

        // Jupiter swap discriminator
        self.signature_patterns.insert(
            vec![0x8a, 0x49, 0x25, 0xf9, 0xe2, 0x50, 0x69, 0x8f],
            ProgramType::Jupiter,
        ); // Route

        // Raydium swap discriminator
        self.signature_patterns.insert(
            vec![0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8],
            ProgramType::Raydium,
        ); // Swap
    }

    fn detect_by_signature(&self, instruction_data: &[u8]) -> Option<ProgramType> {
        if instruction_data.is_empty() {
            return None;
        }

        // Check single byte discriminators (SPL Token)
        if instruction_data.len() >= 1 {
            if let Some(program_type) = self.signature_patterns.get(&instruction_data[..1]) {
                return Some(program_type.clone());
            }
        }

        // Check 8-byte discriminators (Anchor programs)
        if instruction_data.len() >= 8 {
            if let Some(program_type) = self.signature_patterns.get(&instruction_data[..8]) {
                return Some(program_type.clone());
            }
        }

        None
    }

    fn detect_by_heuristics(
        &self,
        program_id: &Pubkey,
        instruction_data: &[u8],
        account_keys: &[Pubkey],
    ) -> Option<DetectionResult> {
        let mut confidence = 0.0;
        let mut detected_type = ProgramType::Unknown;
        let mut metadata = HashMap::new();

        // Heuristic 1: Check for SPL Token patterns
        if self.looks_like_spl_token(instruction_data, account_keys) {
            confidence = 0.7;
            detected_type = ProgramType::SplToken;
            metadata.insert("heuristic".to_string(), "spl_token_pattern".to_string());
        }

        // Heuristic 2: Check for NFT/Metaplex patterns
        if self.looks_like_metaplex(instruction_data, account_keys) {
            confidence = 0.6;
            detected_type = ProgramType::Metaplex;
            metadata.insert("heuristic".to_string(), "metaplex_pattern".to_string());
        }

        // Heuristic 3: Check for DEX patterns
        if self.looks_like_dex(instruction_data, account_keys) {
            confidence = 0.5;
            detected_type = ProgramType::Jupiter; // Generic DEX detection
            metadata.insert("heuristic".to_string(), "dex_pattern".to_string());
        }

        if confidence > 0.0 {
            Some(DetectionResult {
                program_type: detected_type,
                confidence,
                detection_method: DetectionMethod::Heuristic,
                metadata,
            })
        } else {
            None
        }
    }

    fn is_anchor_program(&self, instruction_data: &[u8]) -> bool {
        // Anchor programs typically have 8-byte discriminators
        instruction_data.len() >= 8 &&
        // Check for common Anchor patterns
        (instruction_data[0] != 0 || instruction_data.iter().take(8).any(|&b| b != 0))
    }

    fn looks_like_spl_token(&self, instruction_data: &[u8], account_keys: &[Pubkey]) -> bool {
        if instruction_data.is_empty() {
            return false;
        }

        // SPL Token instructions are typically single-byte discriminators (0-12)
        let discriminator = instruction_data[0];
        if discriminator > 12 {
            return false;
        }

        // SPL Token operations typically involve 2-4 accounts
        account_keys.len() >= 2 && account_keys.len() <= 8
    }

    fn looks_like_metaplex(&self, instruction_data: &[u8], account_keys: &[Pubkey]) -> bool {
        // Metaplex operations typically involve many accounts (5-15)
        if account_keys.len() < 5 {
            return false;
        }

        // Check for metadata account patterns
        account_keys.iter().any(|key| {
            let key_str = key.to_string();
            // Metadata accounts often start with specific prefixes
            key_str.starts_with("metadata") ||
            key_str.contains("11111111111111111111111111111111") // System program often involved
        })
    }

    fn looks_like_dex(&self, instruction_data: &[u8], account_keys: &[Pubkey]) -> bool {
        // DEX operations typically involve 6-20 accounts
        if account_keys.len() < 6 || account_keys.len() > 20 {
            return false;
        }

        // DEX instructions are usually complex (larger instruction data)
        instruction_data.len() > 8
    }
}

impl Default for ProgramDetector {
    fn default() -> Self {
        Self::new()
    }
}