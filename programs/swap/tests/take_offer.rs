mod common;

use anchor_lang::InstructionData;
use mollusk_svm::{result::Check, Mollusk};
use mollusk_svm_programs_token::{associated_token, token};
use solana_account::{Account, ReadableAccount};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;
use spl_associated_token_account_interface::address::get_associated_token_address_with_program_id;
use std::collections::HashMap;
use swap::instruction as swap_instruction;

use common::{
    configure_sbf_out_dir, mint_account, system_account, token_account, DECIMALS,
    INITIAL_MAKER_MINT_AMOUNT, OFFER_AMOUNT_GIVES, OFFER_AMOUNT_WANTS,
};

const INITIAL_TAKER_WANTS_AMOUNT: u64 = 8_000_000;

fn swap_program_id() -> Pubkey {
    Pubkey::new_from_array(swap::ID.to_bytes())
}

fn system_program_id() -> Pubkey {
    Pubkey::new_from_array(solana_sdk_ids::system_program::id().to_bytes())
}

fn find_offer_pda(maker: &Pubkey, offer_id: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"offer", maker.as_ref(), &offer_id.to_le_bytes()],
        &swap_program_id(),
    )
}

fn make_offer_instruction(
    maker: Pubkey,
    mint_maker_gives: Pubkey,
    mint_maker_wants: Pubkey,
    maker_ata_gives: Pubkey,
    vault: Pubkey,
    offer: Pubkey,
    offer_id: u64,
) -> Instruction {
    Instruction {
        program_id: swap_program_id(),
        accounts: vec![
            AccountMeta::new(maker, true),
            AccountMeta::new_readonly(mint_maker_gives, false),
            AccountMeta::new_readonly(mint_maker_wants, false),
            AccountMeta::new(maker_ata_gives, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(associated_token::ID, false),
            AccountMeta::new(offer, false),
            AccountMeta::new_readonly(token::ID, false),
            AccountMeta::new_readonly(system_program_id(), false),
        ],
        data: swap_instruction::MakeOffer {
            offer_id,
            amount_maker_gives: OFFER_AMOUNT_GIVES,
            amount_maker_wants: OFFER_AMOUNT_WANTS,
        }
        .data(),
    }
}

fn take_offer_instruction(
    taker: Pubkey,
    maker: Pubkey,
    mint_maker_gives: Pubkey,
    mint_maker_wants: Pubkey,
    maker_ata_wants: Pubkey,
    taker_ata_wants: Pubkey,
    taker_ata_gives: Pubkey,
    vault: Pubkey,
    offer: Pubkey,
    offer_id: u64,
) -> Instruction {
    Instruction {
        program_id: swap_program_id(),
        accounts: vec![
            AccountMeta::new(taker, true),
            AccountMeta::new(maker, false),
            AccountMeta::new_readonly(mint_maker_gives, false),
            AccountMeta::new_readonly(mint_maker_wants, false),
            AccountMeta::new(maker_ata_wants, false),
            AccountMeta::new(taker_ata_wants, false),
            AccountMeta::new(taker_ata_gives, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(offer, false),
            AccountMeta::new_readonly(token::ID, false),
        ],
        data: swap_instruction::TakeOffer { offer_id }.data(),
    }
}

#[test]
fn take_offer_success_settles_and_closes_escrow_accounts() {
    configure_sbf_out_dir();
    let program_id = swap_program_id();
    let mut mollusk = Mollusk::new(&program_id, "swap");
    token::add_program(&mut mollusk);
    associated_token::add_program(&mut mollusk);

    let maker = Pubkey::new_unique();
    let taker = Pubkey::new_unique();
    let mint_maker_gives = Pubkey::new_unique();
    let mint_maker_wants = Pubkey::new_unique();
    let maker_ata_gives =
        get_associated_token_address_with_program_id(&maker, &mint_maker_gives, &token::ID);
    let maker_ata_wants =
        get_associated_token_address_with_program_id(&maker, &mint_maker_wants, &token::ID);
    let taker_ata_wants =
        get_associated_token_address_with_program_id(&taker, &mint_maker_wants, &token::ID);
    let taker_ata_gives =
        get_associated_token_address_with_program_id(&taker, &mint_maker_gives, &token::ID);
    let offer_id = 3u64;
    let (offer, _) = find_offer_pda(&maker, offer_id);
    let vault =
        get_associated_token_address_with_program_id(&offer, &mint_maker_gives, &token::ID);

    let mut account_store = HashMap::new();
    account_store.insert(maker, system_account(10_000_000_000));
    account_store.insert(taker, system_account(10_000_000_000));
    account_store.insert(
        mint_maker_gives,
        mint_account(&maker, INITIAL_MAKER_MINT_AMOUNT, DECIMALS),
    );
    account_store.insert(
        mint_maker_wants,
        mint_account(&taker, INITIAL_TAKER_WANTS_AMOUNT, DECIMALS),
    );
    account_store.insert(
        maker_ata_gives,
        token_account(&mint_maker_gives, &maker, INITIAL_MAKER_MINT_AMOUNT),
    );
    account_store.insert(
        maker_ata_wants,
        token_account(&mint_maker_wants, &maker, 0),
    );
    account_store.insert(
        taker_ata_wants,
        token_account(&mint_maker_wants, &taker, INITIAL_TAKER_WANTS_AMOUNT),
    );
    account_store.insert(
        taker_ata_gives,
        token_account(&mint_maker_gives, &taker, 0),
    );

    let context = mollusk.with_context(account_store);

    context.process_and_validate_instruction(
        &make_offer_instruction(
            maker,
            mint_maker_gives,
            mint_maker_wants,
            maker_ata_gives,
            vault,
            offer,
            offer_id,
        ),
        &[Check::success()],
    );

    context.process_and_validate_instruction(
        &take_offer_instruction(
            taker,
            maker,
            mint_maker_gives,
            mint_maker_wants,
            maker_ata_wants,
            taker_ata_wants,
            taker_ata_gives,
            vault,
            offer,
            offer_id,
        ),
        &[
            Check::success(),
            Check::account(&vault).closed().build(),
            Check::account(&offer).closed().build(),
        ],
    );

    let store = context.account_store.borrow();

    let maker_ata_wants_account = store.get(&maker_ata_wants).unwrap();
    assert_eq!(
        maker_ata_wants_account.data(),
        token_account(&mint_maker_wants, &maker, OFFER_AMOUNT_WANTS).data()
    );

    let taker_ata_wants_account = store.get(&taker_ata_wants).unwrap();
    assert_eq!(
        taker_ata_wants_account.data(),
        token_account(
            &mint_maker_wants,
            &taker,
            INITIAL_TAKER_WANTS_AMOUNT - OFFER_AMOUNT_WANTS,
        )
        .data()
    );

    let taker_ata_gives_account = store.get(&taker_ata_gives).unwrap();
    assert_eq!(
        taker_ata_gives_account.data(),
        token_account(&mint_maker_gives, &taker, OFFER_AMOUNT_GIVES).data()
    );

    let vault_account = store.get(&vault).unwrap();
    assert_eq!(vault_account, &Account::default());

    let offer_account = store.get(&offer).unwrap();
    assert_eq!(offer_account, &Account::default());
}

#[test]
fn take_offer_rejects_self_take_without_mutating_escrow() {
    configure_sbf_out_dir();
    let program_id = swap_program_id();
    let mut mollusk = Mollusk::new(&program_id, "swap");
    token::add_program(&mut mollusk);
    associated_token::add_program(&mut mollusk);

    let maker = Pubkey::new_unique();
    let mint_maker_gives = Pubkey::new_unique();
    let mint_maker_wants = Pubkey::new_unique();
    let maker_ata_gives =
        get_associated_token_address_with_program_id(&maker, &mint_maker_gives, &token::ID);
    let maker_ata_wants =
        get_associated_token_address_with_program_id(&maker, &mint_maker_wants, &token::ID);
    let offer_id = 11u64;
    let (offer, _) = find_offer_pda(&maker, offer_id);
    let vault =
        get_associated_token_address_with_program_id(&offer, &mint_maker_gives, &token::ID);

    let mut account_store = HashMap::new();
    account_store.insert(maker, system_account(10_000_000_000));
    account_store.insert(
        mint_maker_gives,
        mint_account(&maker, INITIAL_MAKER_MINT_AMOUNT, DECIMALS),
    );
    account_store.insert(
        mint_maker_wants,
        mint_account(&maker, INITIAL_TAKER_WANTS_AMOUNT, DECIMALS),
    );
    account_store.insert(
        maker_ata_gives,
        token_account(&mint_maker_gives, &maker, INITIAL_MAKER_MINT_AMOUNT),
    );
    account_store.insert(
        maker_ata_wants,
        token_account(&mint_maker_wants, &maker, INITIAL_TAKER_WANTS_AMOUNT),
    );

    let context = mollusk.with_context(account_store);

    context.process_and_validate_instruction(
        &make_offer_instruction(
            maker,
            mint_maker_gives,
            mint_maker_wants,
            maker_ata_gives,
            vault,
            offer,
            offer_id,
        ),
        &[Check::success()],
    );

    context.process_and_validate_instruction(
        &take_offer_instruction(
            maker,
            maker,
            mint_maker_gives,
            mint_maker_wants,
            maker_ata_wants,
            maker_ata_wants,
            maker_ata_gives,
            vault,
            offer,
            offer_id,
        ),
        &[
            Check::err(ProgramError::Custom(6007)),
            Check::account(&maker_ata_wants)
                .data(token_account(
                    &mint_maker_wants,
                    &maker,
                    INITIAL_TAKER_WANTS_AMOUNT,
                )
                .data())
                .owner(&token::ID)
                .build(),
            Check::account(&vault)
                .data(token_account(&mint_maker_gives, &offer, OFFER_AMOUNT_GIVES).data())
                .owner(&token::ID)
                .build(),
        ],
    );

    let store = context.account_store.borrow();
    let offer_account = store.get(&offer).unwrap();
    assert_ne!(offer_account, &Account::default());
}
