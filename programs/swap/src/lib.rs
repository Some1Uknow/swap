use anchor_lang::prelude::*;
pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("CmHeBm28AQ2jq5GY6P6heu8ByMaoqyDZGJx7HEsJiFJh");

#[program]
pub mod swap {
    use super::*;

    pub fn make_offer(
        ctx: Context<MakeOffer>,
        offer_id: u64,
        amount_maker_gives: u64,
        amount_maker_wants: u64,
    ) -> Result<()> {
        instructions::make_offer::handle_make_offer(
            ctx,
            offer_id,
            amount_maker_gives,
            amount_maker_wants,
        )
    }

    pub fn take_offer(ctx: Context<TakeOffer>, offer_id: u64) -> Result<()> {
        instructions::take_offer::handle_take_offer(ctx, offer_id)
    }

    pub fn cancel_offer(ctx: Context<CancelOffer>, offer_id: u64) -> Result<()> {
        instructions::cancel_offer::handle_cancel_offer(ctx, offer_id)
    }
}
