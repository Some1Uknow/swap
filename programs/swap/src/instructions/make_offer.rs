use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::constants::ANCHOR_DISCRIMINATOR_SIZE;
use crate::error::ErrorCode;
use crate::events::OfferCreated;
use crate::state::{Offer, OfferStatus};

#[derive(Accounts)]
#[instruction(offer_id: u64)]
pub struct MakeOffer<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(mint::token_program = token_program)]
    pub mint_maker_gives: InterfaceAccount<'info, Mint>,

    #[account(mint::token_program = token_program)]
    pub mint_maker_wants: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint_maker_gives,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    )]
    pub maker_ata_gives: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        payer = maker,
        associated_token::mint = mint_maker_gives,
        associated_token::authority = offer,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    #[account(
        init,
        payer = maker,
        space = ANCHOR_DISCRIMINATOR_SIZE + Offer::INIT_SPACE,
        seeds = [b"offer", maker.key().as_ref(), offer_id.to_le_bytes().as_ref()],
        bump
    )]
    pub offer: Account<'info, Offer>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn handle_make_offer(
    ctx: Context<MakeOffer>,
    offer_id: u64,
    amount_maker_gives: u64,
    amount_maker_wants: u64,
) -> Result<()> {
    require!(
        amount_maker_gives > 0 && amount_maker_wants > 0,
        ErrorCode::InvalidAmount
    );
    require_keys_neq!(
        ctx.accounts.mint_maker_gives.key(),
        ctx.accounts.mint_maker_wants.key(),
        ErrorCode::SameMintNotAllowed
    );

    let transfer_accounts = TransferChecked {
        from: ctx.accounts.maker_ata_gives.to_account_info(),
        mint: ctx.accounts.mint_maker_gives.to_account_info(),
        to: ctx.accounts.vault.to_account_info(),
        authority: ctx.accounts.maker.to_account_info(),
    };

    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        transfer_accounts,
    );

    token_interface::transfer_checked(
        cpi_ctx,
        amount_maker_gives,
        ctx.accounts.mint_maker_gives.decimals,
    )?;

    ctx.accounts.offer.set_inner(Offer {
        id: offer_id,
        maker: ctx.accounts.maker.key(),
        taker: None,
        mint_maker_gives: ctx.accounts.mint_maker_gives.key(),
        mint_maker_wants: ctx.accounts.mint_maker_wants.key(),
        amount_maker_gives,
        amount_maker_wants,
        status: OfferStatus::Open as u8,
        bump: ctx.bumps.offer,
    });

    emit!(OfferCreated {
        offer: ctx.accounts.offer.key(),
        offer_id,
        maker: ctx.accounts.maker.key(),
        mint_maker_gives: ctx.accounts.mint_maker_gives.key(),
        mint_maker_wants: ctx.accounts.mint_maker_wants.key(),
        amount_maker_gives,
        amount_maker_wants,
        vault: ctx.accounts.vault.key(),
    });

    Ok(())
}
