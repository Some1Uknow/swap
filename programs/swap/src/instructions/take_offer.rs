use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    self, CloseAccount, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::error::ErrorCode;
use crate::state::{Offer, OfferStatus};

#[derive(Accounts)]
#[instruction(offer_id: u64)]
pub struct TakeOffer<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,

    #[account(mut)]
    pub maker: SystemAccount<'info>,

    #[account(mint::token_program = token_program)]
    pub mint_maker_gives: InterfaceAccount<'info, Mint>,

    #[account(mint::token_program = token_program)]
    pub mint_maker_wants: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint_maker_wants,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    )]
    pub maker_ata_wants: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_maker_wants,
        associated_token::authority = taker,
        associated_token::token_program = token_program,
    )]
    pub taker_ata_wants: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_maker_gives,
        associated_token::authority = taker,
        associated_token::token_program = token_program,
    )]
    pub taker_ata_gives: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_maker_gives,
        associated_token::authority = offer,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"offer", maker.key().as_ref(), offer_id.to_le_bytes().as_ref()],
        bump
    )]
    pub offer: Account<'info, Offer>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handle_take_offer(ctx: Context<TakeOffer>, offer_id: u64) -> Result<()> {
    let offer = &ctx.accounts.offer;

    require!(offer.id == offer_id, ErrorCode::OfferIdMismatch);
    require!(
        offer.status == OfferStatus::Open as u8,
        ErrorCode::OfferNotOpen
    );
    require_keys_eq!(
        offer.mint_maker_gives,
        ctx.accounts.mint_maker_gives.key(),
        ErrorCode::MintMismatch
    );
    require_keys_eq!(
        offer.mint_maker_wants,
        ctx.accounts.mint_maker_wants.key(),
        ErrorCode::MintMismatch
    );
    require_keys_neq!(
        ctx.accounts.maker.key(),
        ctx.accounts.taker.key(),
        ErrorCode::MakerCannotBeTaker
    );

    let pay_maker = TransferChecked {
        from: ctx.accounts.taker_ata_wants.to_account_info(),
        to: ctx.accounts.maker_ata_wants.to_account_info(),
        authority: ctx.accounts.taker.to_account_info(),
        mint: ctx.accounts.mint_maker_wants.to_account_info(),
    };

    let cpi_ctx_maker = CpiContext::new(ctx.accounts.token_program.to_account_info(), pay_maker);

    token_interface::transfer_checked(
        cpi_ctx_maker,
        offer.amount_maker_wants,
        ctx.accounts.mint_maker_wants.decimals,
    )?;

    let pay_taker = TransferChecked {
        from: ctx.accounts.vault.to_account_info(),
        to: ctx.accounts.taker_ata_gives.to_account_info(),
        authority: ctx.accounts.offer.to_account_info(),
        mint: ctx.accounts.mint_maker_gives.to_account_info(),
    };

    let maker_key = ctx.accounts.maker.key();
    let offer_id_bytes = offer_id.to_le_bytes();

    let signer_seeds: &[&[u8]] = &[
        b"offer",
        maker_key.as_ref(),
        offer_id_bytes.as_ref(),
        &[ctx.accounts.offer.bump],
    ];

    let binding = [signer_seeds];
    let cpi_ctx_taker = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        pay_taker,
        &binding,
    );

    token_interface::transfer_checked(
        cpi_ctx_taker,
        offer.amount_maker_gives,
        ctx.accounts.mint_maker_gives.decimals,
    )?;

    ctx.accounts.offer.taker = Some(ctx.accounts.taker.key());
    ctx.accounts.offer.status = OfferStatus::Filled as u8;

    let close_vault = CloseAccount {
        account: ctx.accounts.vault.to_account_info(),
        destination: ctx.accounts.maker.to_account_info(),
        authority: ctx.accounts.offer.to_account_info(),
    };
    let close_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        close_vault,
        &binding,
    );
    token_interface::close_account(close_ctx)?;

    Ok(())
}
