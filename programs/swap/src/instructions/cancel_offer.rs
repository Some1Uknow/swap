use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    self, CloseAccount, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::error::ErrorCode;
use crate::state::{Offer, OfferStatus};

#[derive(Accounts)]
#[instruction(offer_id: u64)]
pub struct CancelOffer<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(mint::token_program = token_program)]
    pub mint_maker_gives: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint_maker_gives,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    )]
    pub maker_ata_gives: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_maker_gives,
        associated_token::authority = offer,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        close = maker,
        seeds = [b"offer", maker.key().as_ref(), offer_id.to_le_bytes().as_ref()],
        bump
    )]
    pub offer: Account<'info, Offer>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handle_cancel_offer(ctx: Context<CancelOffer>, offer_id: u64) -> Result<()> {
    let offer = &ctx.accounts.offer;

    require!(offer.id == offer_id, ErrorCode::OfferIdMismatch);
    require!(
        offer.status == OfferStatus::Open as u8,
        ErrorCode::OfferNotOpen
    );
    require_keys_eq!(offer.maker, ctx.accounts.maker.key(), ErrorCode::MakerMismatch);
    require_keys_eq!(
        offer.mint_maker_gives,
        ctx.accounts.mint_maker_gives.key(),
        ErrorCode::MintMismatch
    );

    let refund_amount = offer.amount_maker_gives;
    let offer_bump = offer.bump;

    let refund_maker = TransferChecked {
        from: ctx.accounts.vault.to_account_info(),
        to: ctx.accounts.maker_ata_gives.to_account_info(),
        authority: ctx.accounts.offer.to_account_info(),
        mint: ctx.accounts.mint_maker_gives.to_account_info(),
    };

    let maker_key = ctx.accounts.maker.key();
    let offer_id_bytes = offer_id.to_le_bytes();
    let signer_seeds: &[&[u8]] = &[
        b"offer",
        maker_key.as_ref(),
        offer_id_bytes.as_ref(),
        &[offer_bump],
    ];
    let signer_binding = [signer_seeds];

    let refund_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        refund_maker,
        &signer_binding,
    );

    token_interface::transfer_checked(
        refund_ctx,
        refund_amount,
        ctx.accounts.mint_maker_gives.decimals,
    )?;

    let close_vault = CloseAccount {
        account: ctx.accounts.vault.to_account_info(),
        destination: ctx.accounts.maker.to_account_info(),
        authority: ctx.accounts.offer.to_account_info(),
    };

    let close_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        close_vault,
        &signer_binding,
    );
    token_interface::close_account(close_ctx)?;

    Ok(())
}
