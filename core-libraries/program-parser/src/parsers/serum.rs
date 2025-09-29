//! Serum DEX parser
use crate::{error::ParseError, types::*};
use solana_sdk::pubkey::Pubkey;

pub struct SerumParser;
impl SerumParser {
    pub fn new() -> Self { Self }
    pub async fn parse(&self, _: &Pubkey, _: &[u8], _: &[Pubkey]) -> Result<crate::types::ParseResult, ParseError> {
        Ok(crate::types::ParseResult::Serum(SerumData {
            operation_type: SerumOperation::PlaceOrder,
            market: Pubkey::new_unique(),
            base_mint: Pubkey::new_unique(),
            quote_mint: Pubkey::new_unique(),
            side: OrderSide::Bid,
            price: 10000,
            quantity: 1000,
            order_id: Some(12345),
            client_order_id: Some(67890),
            fees: SerumFees { base_fee: 100, quote_fee: 50, referrer_rebate: None },
        }))
    }
}