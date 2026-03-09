use anchor_lang::prelude::*;

#[event]
pub struct OfferCreated {
    pub offer: Pubkey,
    pub offer_id: u64,
    pub maker: Pubkey,
    pub mint_maker_gives: Pubkey,
    pub mint_maker_wants: Pubkey,
    pub amount_maker_gives: u64,
    pub amount_maker_wants: u64,
    pub vault: Pubkey,
}

#[event]
pub struct OfferTaken {
    pub offer: Pubkey,
    pub offer_id: u64,
    pub maker: Pubkey,
    pub taker: Pubkey,
    pub mint_maker_gives: Pubkey,
    pub mint_maker_wants: Pubkey,
    pub amount_maker_gives: u64,
    pub amount_maker_wants: u64,
}

#[event]
pub struct OfferCancelled {
    pub offer: Pubkey,
    pub offer_id: u64,
    pub maker: Pubkey,
    pub mint_maker_gives: Pubkey,
    pub amount_maker_gives: u64,
}
