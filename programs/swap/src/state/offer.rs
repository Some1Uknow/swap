use anchor_lang::prelude::*;

use crate::error::ErrorCode;

#[repr(u8)]
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum OfferStatus {
    Open = 0,
    Filled = 1,
}

#[account]
#[derive(InitSpace, Default)]
pub struct Offer {
    pub id: u64,
    pub maker: Pubkey,
    pub taker: Option<Pubkey>,
    pub mint_maker_gives: Pubkey,
    pub mint_maker_wants: Pubkey,
    pub amount_maker_gives: u64,
    pub amount_maker_wants: u64,
    pub status: u8,
    pub bump: u8,
}

impl Offer {
    pub fn status_enum(&self) -> Result<OfferStatus> {
        match self.status {
            0 => Ok(OfferStatus::Open),
            1 => Ok(OfferStatus::Filled),
            _ => err!(ErrorCode::InvalidStatus),
        }
    }
}
