#[allow(dead_code)]
#[path = "../tests/common/mod.rs"]
mod common;

use anchor_lang::InstructionData;
use mollusk_svm::Mollusk;
use mollusk_svm_programs_token::{associated_token, token};
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;
use spl_associated_token_account_interface::address::get_associated_token_address_with_program_id;
use std::path::PathBuf;
use std::time::Instant;
use swap::instruction as swap_instruction;

use common::{
    mint_account, offer_account, system_account, token_account, DECIMALS,
    INITIAL_MAKER_MINT_AMOUNT, OFFER_AMOUNT_GIVES, OFFER_AMOUNT_WANTS,
};

const INITIAL_TAKER_WANTS_AMOUNT: u64 = 8_000_000;

fn configure_sbf_out_dir() {
    let deploy_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/deploy")
        .canonicalize()
        .expect("workspace target/deploy should exist after anchor build");
    std::env::set_var("SBF_OUT_DIR", deploy_dir);
}

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

fn make_offer_fixture() -> (Instruction, Vec<(Pubkey, Account)>) {
    let maker = Pubkey::new_unique();
    let mint_maker_gives = Pubkey::new_unique();
    let mint_maker_wants = Pubkey::new_unique();
    let maker_ata_gives =
        get_associated_token_address_with_program_id(&maker, &mint_maker_gives, &token::ID);
    let offer_id = 1u64;
    let (offer, _) = find_offer_pda(&maker, offer_id);
    let vault =
        get_associated_token_address_with_program_id(&offer, &mint_maker_gives, &token::ID);

    let instruction = Instruction {
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
    };

    let accounts = vec![
        (maker, system_account(10_000_000_000)),
        (
            mint_maker_gives,
            mint_account(&maker, INITIAL_MAKER_MINT_AMOUNT, DECIMALS),
        ),
        (mint_maker_wants, mint_account(&maker, 0, DECIMALS)),
        (
            maker_ata_gives,
            token_account(&mint_maker_gives, &maker, INITIAL_MAKER_MINT_AMOUNT),
        ),
        (vault, Account::default()),
        (offer, Account::default()),
        associated_token::keyed_account(),
        token::keyed_account(),
        mollusk_svm::program::keyed_account_for_system_program(),
    ];

    (instruction, accounts)
}

fn take_offer_fixture() -> (Instruction, Vec<(Pubkey, Account)>) {
    let maker = Pubkey::new_unique();
    let taker = Pubkey::new_unique();
    let mint_maker_gives = Pubkey::new_unique();
    let mint_maker_wants = Pubkey::new_unique();
    let maker_ata_wants =
        get_associated_token_address_with_program_id(&maker, &mint_maker_wants, &token::ID);
    let taker_ata_wants =
        get_associated_token_address_with_program_id(&taker, &mint_maker_wants, &token::ID);
    let taker_ata_gives =
        get_associated_token_address_with_program_id(&taker, &mint_maker_gives, &token::ID);
    let offer_id = 2u64;
    let (offer, bump) = find_offer_pda(&maker, offer_id);
    let vault =
        get_associated_token_address_with_program_id(&offer, &mint_maker_gives, &token::ID);

    let instruction = Instruction {
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
    };

    let accounts = vec![
        (maker, system_account(10_000_000_000)),
        (taker, system_account(10_000_000_000)),
        (
            mint_maker_gives,
            mint_account(&maker, INITIAL_MAKER_MINT_AMOUNT, DECIMALS),
        ),
        (
            mint_maker_wants,
            mint_account(&taker, INITIAL_TAKER_WANTS_AMOUNT, DECIMALS),
        ),
        (maker_ata_wants, token_account(&mint_maker_wants, &maker, 0)),
        (
            taker_ata_wants,
            token_account(&mint_maker_wants, &taker, INITIAL_TAKER_WANTS_AMOUNT),
        ),
        (taker_ata_gives, token_account(&mint_maker_gives, &taker, 0)),
        (vault, token_account(&mint_maker_gives, &offer, OFFER_AMOUNT_GIVES)),
        (
            offer,
            Account {
                lamports: 10_000_000,
                data: offer_account(offer_id, maker, mint_maker_gives, mint_maker_wants, bump),
                owner: swap_program_id(),
                executable: false,
                rent_epoch: 0,
            },
        ),
        token::keyed_account(),
    ];

    (instruction, accounts)
}

fn cancel_offer_fixture() -> (Instruction, Vec<(Pubkey, Account)>) {
    let maker = Pubkey::new_unique();
    let mint_maker_gives = Pubkey::new_unique();
    let maker_ata_gives =
        get_associated_token_address_with_program_id(&maker, &mint_maker_gives, &token::ID);
    let offer_id = 3u64;
    let (offer, bump) = find_offer_pda(&maker, offer_id);
    let vault =
        get_associated_token_address_with_program_id(&offer, &mint_maker_gives, &token::ID);

    let instruction = Instruction {
        program_id: swap_program_id(),
        accounts: vec![
            AccountMeta::new(maker, true),
            AccountMeta::new_readonly(mint_maker_gives, false),
            AccountMeta::new(maker_ata_gives, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(offer, false),
            AccountMeta::new_readonly(token::ID, false),
        ],
        data: swap_instruction::CancelOffer { offer_id }.data(),
    };

    let accounts = vec![
        (maker, system_account(10_000_000_000)),
        (
            mint_maker_gives,
            mint_account(&maker, INITIAL_MAKER_MINT_AMOUNT, DECIMALS),
        ),
        (
            maker_ata_gives,
            token_account(
                &mint_maker_gives,
                &maker,
                INITIAL_MAKER_MINT_AMOUNT - OFFER_AMOUNT_GIVES,
            ),
        ),
        (vault, token_account(&mint_maker_gives, &offer, OFFER_AMOUNT_GIVES)),
        (
            offer,
            Account {
                lamports: 10_000_000,
                data: offer_account(
                    offer_id,
                    maker,
                    mint_maker_gives,
                    Pubkey::new_unique(),
                    bump,
                ),
                owner: swap_program_id(),
                executable: false,
                rent_epoch: 0,
            },
        ),
        token::keyed_account(),
    ];

    (instruction, accounts)
}

fn main() {
    configure_sbf_out_dir();

    let program_id = swap_program_id();
    let (make_offer_ix, make_offer_accounts) = make_offer_fixture();
    let (take_offer_ix, take_offer_accounts) = take_offer_fixture();
    let (cancel_offer_ix, cancel_offer_accounts) = cancel_offer_fixture();

    for (name, instruction, accounts) in [
        ("make_offer", &make_offer_ix, &make_offer_accounts),
        ("take_offer", &take_offer_ix, &take_offer_accounts),
        ("cancel_offer", &cancel_offer_ix, &cancel_offer_accounts),
    ] {
        let mut total_cu = 0u64;
        let mut total_ns = 0u128;
        let iterations = 25u32;

        for _ in 0..iterations {
            let mut mollusk = Mollusk::new(&program_id, "swap");
            token::add_program(&mut mollusk);
            associated_token::add_program(&mut mollusk);

            let start = Instant::now();
            let result = mollusk.process_instruction(instruction, accounts);
            let elapsed = start.elapsed();

            assert!(
                result.program_result.is_ok(),
                "{name} benchmark fixture must succeed"
            );

            total_cu += result.compute_units_consumed;
            total_ns += elapsed.as_nanos();
        }

        let avg_cu = total_cu / iterations as u64;
        let avg_us = total_ns / iterations as u128 / 1_000;

        println!(
            "{name}: avg_cu={avg_cu} avg_wall_time_us={avg_us} iterations={iterations}"
        );
    }
}
