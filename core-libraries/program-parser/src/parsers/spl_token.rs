//! SPL Token program parser

use crate::{
    error::{ParseError, ParseResult},
    types::*,
};
use solana_sdk::pubkey::Pubkey;
use spl_token::instruction::TokenInstruction;
use std::str::FromStr;

pub struct SplTokenParser;

impl SplTokenParser {
    pub fn new() -> Self {
        Self
    }

    pub async fn parse(
        &self,
        program_id: &Pubkey,
        instruction_data: &[u8],
        account_keys: &[Pubkey],
    ) -> Result<crate::types::ParseResult, ParseError> {
        if instruction_data.is_empty() {
            return Err(ParseError::insufficient_data(1, 0));
        }

        // Parse SPL Token instruction
        let instruction = TokenInstruction::unpack(instruction_data)
            .map_err(|e| ParseError::invalid_instruction_data(format!("SPL Token unpack error: {}", e)))?;

        let spl_data = match instruction {
            TokenInstruction::Transfer { amount } => {
                self.parse_transfer(amount, account_keys).await?
            }
            TokenInstruction::TransferChecked { amount, decimals } => {
                self.parse_transfer_checked(amount, decimals, account_keys).await?
            }
            TokenInstruction::MintTo { amount } => {
                self.parse_mint_to(amount, account_keys).await?
            }
            TokenInstruction::Burn { amount } => {
                self.parse_burn(amount, account_keys).await?
            }
            TokenInstruction::BurnChecked { amount, decimals } => {
                self.parse_burn_checked(amount, decimals, account_keys).await?
            }
            TokenInstruction::Approve { amount } => {
                self.parse_approve(amount, account_keys).await?
            }
            TokenInstruction::Revoke => {
                self.parse_revoke(account_keys).await?
            }
            TokenInstruction::InitializeMint { decimals, mint_authority, freeze_authority } => {
                self.parse_initialize_mint(decimals, mint_authority.into(), freeze_authority.into(), account_keys).await?
            }
            TokenInstruction::InitializeAccount => {
                self.parse_initialize_account(account_keys).await?
            }
            TokenInstruction::CloseAccount => {
                self.parse_close_account(account_keys).await?
            }
            TokenInstruction::SetAuthority { authority_type, new_authority } => {
                self.parse_set_authority(authority_type, new_authority.into(), account_keys).await?
            }
            _ => {
                return Ok(crate::types::ParseResult::Unknown(UnknownData {
                    program_id: *program_id,
                    instruction_data: instruction_data.to_vec(),
                    accounts: account_keys.to_vec(),
                    suspected_program_type: Some("SPL Token".to_string()),
                    confidence: 0.8,
                    raw_bytes: instruction_data.to_vec(),
                    parsing_hints: vec!["Unsupported SPL Token instruction".to_string()],
                }));
            }
        };

        Ok(crate::types::ParseResult::SplToken(spl_data))
    }

    pub async fn parse_account(&self, account_data: &[u8]) -> Result<crate::types::ParseResult, ParseError> {
        if account_data.len() == 82 {
            // SPL Token Mint account
            self.parse_mint_account(account_data).await
        } else if account_data.len() == 165 {
            // SPL Token Account
            self.parse_token_account(account_data).await
        } else {
            Err(ParseError::invalid_instruction_data("Invalid SPL Token account size"))
        }
    }

    async fn parse_transfer(&self, amount: u64, account_keys: &[Pubkey]) -> ParseResult<SplTokenData> {
        if account_keys.len() < 3 {
            return Err(ParseError::insufficient_data(3, account_keys.len()));
        }

        Ok(SplTokenData {
            operation_type: SplTokenOperation::Transfer,
            mint: Pubkey::new_unique(), // Would need to lookup from token account
            amount,
            decimals: 6, // Would need to lookup from mint account
            from: Some(account_keys[0]),
            to: Some(account_keys[1]),
            authority: Some(account_keys[2]),
            multisig_signers: account_keys.get(3..).unwrap_or(&[]).to_vec(),
            parsed_amount: rust_decimal::Decimal::new(amount as i64, 6),
            symbol: None, // Would need metadata lookup
            metadata: self.create_default_metadata(),
        })
    }

    async fn parse_transfer_checked(&self, amount: u64, decimals: u8, account_keys: &[Pubkey]) -> ParseResult<SplTokenData> {
        if account_keys.len() < 4 {
            return Err(ParseError::insufficient_data(4, account_keys.len()));
        }

        Ok(SplTokenData {
            operation_type: SplTokenOperation::TransferChecked,
            mint: account_keys[3],
            amount,
            decimals,
            from: Some(account_keys[0]),
            to: Some(account_keys[1]),
            authority: Some(account_keys[2]),
            multisig_signers: account_keys.get(4..).unwrap_or(&[]).to_vec(),
            parsed_amount: rust_decimal::Decimal::new(amount as i64, decimals as u32),
            symbol: None,
            metadata: self.create_default_metadata(),
        })
    }

    async fn parse_mint_to(&self, amount: u64, account_keys: &[Pubkey]) -> ParseResult<SplTokenData> {
        if account_keys.len() < 3 {
            return Err(ParseError::insufficient_data(3, account_keys.len()));
        }

        Ok(SplTokenData {
            operation_type: SplTokenOperation::MintTo,
            mint: account_keys[0],
            amount,
            decimals: 6, // Would need to lookup
            from: None,
            to: Some(account_keys[1]),
            authority: Some(account_keys[2]),
            multisig_signers: account_keys.get(3..).unwrap_or(&[]).to_vec(),
            parsed_amount: rust_decimal::Decimal::new(amount as i64, 6),
            symbol: None,
            metadata: self.create_default_metadata(),
        })
    }

    async fn parse_burn(&self, amount: u64, account_keys: &[Pubkey]) -> ParseResult<SplTokenData> {
        if account_keys.len() < 3 {
            return Err(ParseError::insufficient_data(3, account_keys.len()));
        }

        Ok(SplTokenData {
            operation_type: SplTokenOperation::Burn,
            mint: Pubkey::new_unique(), // Would need to lookup
            amount,
            decimals: 6,
            from: Some(account_keys[0]),
            to: None,
            authority: Some(account_keys[2]),
            multisig_signers: account_keys.get(3..).unwrap_or(&[]).to_vec(),
            parsed_amount: rust_decimal::Decimal::new(amount as i64, 6),
            symbol: None,
            metadata: self.create_default_metadata(),
        })
    }

    async fn parse_burn_checked(&self, amount: u64, decimals: u8, account_keys: &[Pubkey]) -> ParseResult<SplTokenData> {
        if account_keys.len() < 4 {
            return Err(ParseError::insufficient_data(4, account_keys.len()));
        }

        Ok(SplTokenData {
            operation_type: SplTokenOperation::BurnChecked,
            mint: account_keys[1],
            amount,
            decimals,
            from: Some(account_keys[0]),
            to: None,
            authority: Some(account_keys[2]),
            multisig_signers: account_keys.get(4..).unwrap_or(&[]).to_vec(),
            parsed_amount: rust_decimal::Decimal::new(amount as i64, decimals as u32),
            symbol: None,
            metadata: self.create_default_metadata(),
        })
    }

    async fn parse_approve(&self, amount: u64, account_keys: &[Pubkey]) -> ParseResult<SplTokenData> {
        if account_keys.len() < 3 {
            return Err(ParseError::insufficient_data(3, account_keys.len()));
        }

        Ok(SplTokenData {
            operation_type: SplTokenOperation::Approve,
            mint: Pubkey::new_unique(),
            amount,
            decimals: 6,
            from: Some(account_keys[0]),
            to: Some(account_keys[1]), // Delegate
            authority: Some(account_keys[2]),
            multisig_signers: account_keys.get(3..).unwrap_or(&[]).to_vec(),
            parsed_amount: rust_decimal::Decimal::new(amount as i64, 6),
            symbol: None,
            metadata: self.create_default_metadata(),
        })
    }

    async fn parse_revoke(&self, account_keys: &[Pubkey]) -> ParseResult<SplTokenData> {
        if account_keys.len() < 2 {
            return Err(ParseError::insufficient_data(2, account_keys.len()));
        }

        Ok(SplTokenData {
            operation_type: SplTokenOperation::Revoke,
            mint: Pubkey::new_unique(),
            amount: 0,
            decimals: 6,
            from: Some(account_keys[0]),
            to: None,
            authority: Some(account_keys[1]),
            multisig_signers: account_keys.get(2..).unwrap_or(&[]).to_vec(),
            parsed_amount: rust_decimal::Decimal::ZERO,
            symbol: None,
            metadata: self.create_default_metadata(),
        })
    }

    async fn parse_initialize_mint(
        &self,
        decimals: u8,
        mint_authority: Option<Pubkey>,
        freeze_authority: Option<Pubkey>,
        account_keys: &[Pubkey],
    ) -> ParseResult<SplTokenData> {
        if account_keys.is_empty() {
            return Err(ParseError::insufficient_data(1, 0));
        }

        Ok(SplTokenData {
            operation_type: SplTokenOperation::InitializeMint,
            mint: account_keys[0],
            amount: 0,
            decimals,
            from: None,
            to: None,
            authority: mint_authority,
            multisig_signers: vec![],
            parsed_amount: rust_decimal::Decimal::ZERO,
            symbol: None,
            metadata: SplTokenMetadata {
                name: None,
                symbol: None,
                logo_uri: None,
                total_supply: Some(0),
                is_initialized: true,
                freeze_authority,
                mint_authority,
            },
        })
    }

    async fn parse_initialize_account(&self, account_keys: &[Pubkey]) -> ParseResult<SplTokenData> {
        if account_keys.len() < 3 {
            return Err(ParseError::insufficient_data(3, account_keys.len()));
        }

        Ok(SplTokenData {
            operation_type: SplTokenOperation::CreateAccount,
            mint: account_keys[1],
            amount: 0,
            decimals: 6,
            from: None,
            to: Some(account_keys[0]),
            authority: Some(account_keys[2]),
            multisig_signers: vec![],
            parsed_amount: rust_decimal::Decimal::ZERO,
            symbol: None,
            metadata: self.create_default_metadata(),
        })
    }

    async fn parse_close_account(&self, account_keys: &[Pubkey]) -> ParseResult<SplTokenData> {
        if account_keys.len() < 3 {
            return Err(ParseError::insufficient_data(3, account_keys.len()));
        }

        Ok(SplTokenData {
            operation_type: SplTokenOperation::CloseAccount,
            mint: Pubkey::new_unique(),
            amount: 0,
            decimals: 6,
            from: Some(account_keys[0]),
            to: Some(account_keys[1]), // Destination for lamports
            authority: Some(account_keys[2]),
            multisig_signers: account_keys.get(3..).unwrap_or(&[]).to_vec(),
            parsed_amount: rust_decimal::Decimal::ZERO,
            symbol: None,
            metadata: self.create_default_metadata(),
        })
    }

    async fn parse_set_authority(
        &self,
        authority_type: spl_token::instruction::AuthorityType,
        new_authority: Option<Pubkey>,
        account_keys: &[Pubkey],
    ) -> ParseResult<SplTokenData> {
        if account_keys.len() < 2 {
            return Err(ParseError::insufficient_data(2, account_keys.len()));
        }

        Ok(SplTokenData {
            operation_type: SplTokenOperation::SetAuthority,
            mint: account_keys[0],
            amount: 0,
            decimals: 6,
            from: None,
            to: new_authority.into(),
            authority: Some(account_keys[1]),
            multisig_signers: account_keys.get(2..).unwrap_or(&[]).to_vec(),
            parsed_amount: rust_decimal::Decimal::ZERO,
            symbol: None,
            metadata: self.create_default_metadata(),
        })
    }

    async fn parse_mint_account(&self, account_data: &[u8]) -> Result<crate::types::ParseResult, ParseError> {
        // Parse SPL Token Mint account structure
        if account_data.len() != 82 {
            return Err(ParseError::insufficient_data(82, account_data.len()));
        }

        // This would parse the actual mint account data structure
        // For now, return a simplified version
        Ok(crate::types::ParseResult::SplToken(SplTokenData {
            operation_type: SplTokenOperation::InitializeMint,
            mint: Pubkey::new_unique(),
            amount: 0,
            decimals: 6,
            from: None,
            to: None,
            authority: None,
            multisig_signers: vec![],
            parsed_amount: rust_decimal::Decimal::ZERO,
            symbol: None,
            metadata: self.create_default_metadata(),
        }))
    }

    async fn parse_token_account(&self, account_data: &[u8]) -> Result<crate::types::ParseResult, ParseError> {
        // Parse SPL Token Account structure
        if account_data.len() != 165 {
            return Err(ParseError::insufficient_data(165, account_data.len()));
        }

        // This would parse the actual token account data structure
        Ok(crate::types::ParseResult::SplToken(SplTokenData {
            operation_type: SplTokenOperation::CreateAccount,
            mint: Pubkey::new_unique(),
            amount: 0,
            decimals: 6,
            from: None,
            to: None,
            authority: None,
            multisig_signers: vec![],
            parsed_amount: rust_decimal::Decimal::ZERO,
            symbol: None,
            metadata: self.create_default_metadata(),
        }))
    }

    fn create_default_metadata(&self) -> SplTokenMetadata {
        SplTokenMetadata {
            name: None,
            symbol: None,
            logo_uri: None,
            total_supply: None,
            is_initialized: true,
            freeze_authority: None,
            mint_authority: None,
        }
    }
}

impl Default for SplTokenParser {
    fn default() -> Self {
        Self::new()
    }
}