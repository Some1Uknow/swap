use anchor_lang::{prelude::Pubkey as AnchorPubkey, AccountSerialize, Space};
use solana_account::Account as SolanaAccount;
use solana_program_pack::Pack;
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use spl_token_interface::state::{Account as TokenAccount, AccountState, Mint};
use std::path::PathBuf;
use swap::{constants::ANCHOR_DISCRIMINATOR_SIZE, state::{Offer, OfferStatus}};

pub const INITIAL_MAKER_MINT_AMOUNT: u64 = 5_000_000;
pub const OFFER_AMOUNT_GIVES: u64 = 1_000_000;
pub const OFFER_AMOUNT_WANTS: u64 = 2_000_000;
pub const DECIMALS: u8 = 6;

pub fn anchor_pubkey(pubkey: &Pubkey) -> AnchorPubkey {
    AnchorPubkey::new_from_array(pubkey.to_bytes())
}

pub fn configure_sbf_out_dir() {
    let deploy_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/deploy")
        .canonicalize()
        .expect("workspace target/deploy should exist after anchor build");
    std::env::set_var("SBF_OUT_DIR", deploy_dir);
}

pub fn system_account(lamports: u64) -> SolanaAccount {
    SolanaAccount::new(
        lamports,
        0,
        &Pubkey::new_from_array(solana_sdk_ids::system_program::id().to_bytes()),
    )
}

pub fn mint_account(authority: &Pubkey, supply: u64, decimals: u8) -> SolanaAccount {
    let data = {
        let mut data = vec![0; Mint::LEN];
        let state = Mint {
            mint_authority: Some(*authority).into(),
            supply,
            decimals,
            is_initialized: true,
            freeze_authority: None.into(),
        };
        state.pack_into_slice(&mut data);
        data
    };

    SolanaAccount {
        lamports: Rent::default().minimum_balance(data.len()),
        data,
        owner: mollusk_svm_programs_token::token::ID,
        ..Default::default()
    }
}

pub fn token_account(mint: &Pubkey, owner: &Pubkey, amount: u64) -> SolanaAccount {
    let data = {
        let mut data = vec![0; TokenAccount::LEN];
        let state = TokenAccount {
            mint: *mint,
            owner: *owner,
            amount,
            delegate: None.into(),
            state: AccountState::Initialized,
            is_native: None.into(),
            delegated_amount: 0,
            close_authority: None.into(),
        };
        state.pack_into_slice(&mut data);
        data
    };

    SolanaAccount {
        lamports: Rent::default().minimum_balance(data.len()),
        data,
        owner: mollusk_svm_programs_token::token::ID,
        ..Default::default()
    }
}

pub fn empty_account() -> SolanaAccount {
    SolanaAccount::default()
}

pub fn offer_account(
    id: u64,
    maker: Pubkey,
    mint_maker_gives: Pubkey,
    mint_maker_wants: Pubkey,
    bump: u8,
) -> Vec<u8> {
    let mut data = Vec::new();
    Offer {
        id,
        maker: anchor_pubkey(&maker),
        taker: None,
        mint_maker_gives: anchor_pubkey(&mint_maker_gives),
        mint_maker_wants: anchor_pubkey(&mint_maker_wants),
        amount_maker_gives: OFFER_AMOUNT_GIVES,
        amount_maker_wants: OFFER_AMOUNT_WANTS,
        status: OfferStatus::Open as u8,
        bump,
    }
    .try_serialize(&mut data)
    .unwrap();
    data.resize(ANCHOR_DISCRIMINATOR_SIZE + Offer::INIT_SPACE, 0);
    data
}
