//! Anchor framework parser
use crate::{error::ParseError, types::*};
use solana_sdk::pubkey::Pubkey;

pub struct AnchorParser;
impl AnchorParser {
    pub fn new() -> Self { Self }
    pub async fn parse(&self, program_id: &Pubkey, instruction_data: &[u8], account_keys: &[Pubkey]) -> Result<crate::types::ParseResult, ParseError> {
        Ok(crate::types::ParseResult::Anchor(AnchorData {
            program_id: *program_id,
            instruction_name: "unknown".to_string(),
            instruction_discriminator: instruction_data.get(..8).unwrap_or(&[]).to_vec(),
            accounts: account_keys.iter().enumerate().map(|(i, &pubkey)| AnchorAccount {
                name: format!("account_{}", i),
                pubkey,
                is_signer: false,
                is_writable: false,
                is_optional: false,
            }).collect(),
            instruction_data: std::collections::HashMap::new(),
            idl_version: None,
            program_name: None,
        }))
    }
}