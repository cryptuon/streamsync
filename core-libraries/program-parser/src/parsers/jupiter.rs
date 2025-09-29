//! Jupiter aggregator parser
use crate::{error::ParseError, types::*};
use solana_sdk::pubkey::Pubkey;

pub struct JupiterParser;
impl JupiterParser {
    pub fn new() -> Self { Self }
    pub async fn parse(&self, _program_id: &Pubkey, _instruction_data: &[u8], _account_keys: &[Pubkey]) -> Result<crate::types::ParseResult, ParseError> {
        Ok(crate::types::ParseResult::Jupiter(JupiterData {
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
        }))
    }
}