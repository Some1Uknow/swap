mod common;

use anchor_lang::InstructionData;
use mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk};
use mollusk_svm_programs_token::{associated_token, token};
use solana_account::ReadableAccount;
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;
use spl_associated_token_account_interface::address::get_associated_token_address_with_program_id;
use swap::instruction as swap_instruction;

use common::{
    configure_sbf_out_dir, empty_account, mint_account, offer_account, system_account,
    token_account, DECIMALS, INITIAL_MAKER_MINT_AMOUNT, OFFER_AMOUNT_GIVES, OFFER_AMOUNT_WANTS,
};

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
    amount_maker_gives: u64,
    amount_maker_wants: u64,
) -> Instruction {
    let instruction_data = swap_instruction::MakeOffer {
        offer_id,
        amount_maker_gives,
        amount_maker_wants,
    }
    .data();

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
        data: instruction_data,
    }
}

#[test]
fn make_offer_success_moves_tokens_and_persists_offer() {
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
    let offer_id = 1u64;
    let (offer, bump) = find_offer_pda(&maker, offer_id);
    let vault =
        get_associated_token_address_with_program_id(&offer, &mint_maker_gives, &token::ID);
    let instruction = make_offer_instruction(
        maker,
        mint_maker_gives,
        mint_maker_wants,
        maker_ata_gives,
        vault,
        offer,
        offer_id,
        OFFER_AMOUNT_GIVES,
        OFFER_AMOUNT_WANTS,
    );

    let expected_vault = token_account(&mint_maker_gives, &offer, OFFER_AMOUNT_GIVES);
    let expected_maker_ata = token_account(
        &mint_maker_gives,
        &maker,
        INITIAL_MAKER_MINT_AMOUNT - OFFER_AMOUNT_GIVES,
    );
    let expected_offer =
        offer_account(offer_id, maker, mint_maker_gives, mint_maker_wants, bump);

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
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
            (vault, empty_account()),
            (offer, empty_account()),
            associated_token::keyed_account(),
            token::keyed_account(),
            keyed_account_for_system_program(),
        ],
        &[
            Check::success(),
            Check::account(&maker_ata_gives)
                .data(expected_maker_ata.data())
                .owner(&token::ID)
                .build(),
            Check::account(&vault)
                .data(expected_vault.data())
                .owner(&token::ID)
                .build(),
            Check::account(&offer)
                .data(&expected_offer)
                .owner(&program_id)
                .build(),
        ],
    );
}

#[test]
fn make_offer_zero_amount_fails_without_state_changes() {
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
    let offer_id = 7u64;
    let (offer, _) = find_offer_pda(&maker, offer_id);
    let vault =
        get_associated_token_address_with_program_id(&offer, &mint_maker_gives, &token::ID);
    let instruction = make_offer_instruction(
        maker,
        mint_maker_gives,
        mint_maker_wants,
        maker_ata_gives,
        vault,
        offer,
        offer_id,
        0,
        OFFER_AMOUNT_WANTS,
    );

    let original_maker_ata = token_account(&mint_maker_gives, &maker, INITIAL_MAKER_MINT_AMOUNT);

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (maker, system_account(10_000_000_000)),
            (
                mint_maker_gives,
                mint_account(&maker, INITIAL_MAKER_MINT_AMOUNT, DECIMALS),
            ),
            (mint_maker_wants, mint_account(&maker, 0, DECIMALS)),
            (maker_ata_gives, original_maker_ata.clone()),
            (vault, empty_account()),
            (offer, empty_account()),
            associated_token::keyed_account(),
            token::keyed_account(),
            keyed_account_for_system_program(),
        ],
        &[
            Check::err(ProgramError::Custom(6000)),
            Check::account(&maker_ata_gives)
                .data(original_maker_ata.data())
                .owner(&token::ID)
                .build(),
            Check::account(&vault).closed().build(),
            Check::account(&offer).closed().build(),
        ],
    );
}

#[test]
fn make_offer_same_mint_fails_without_state_changes() {
    configure_sbf_out_dir();
    let program_id = swap_program_id();
    let mut mollusk = Mollusk::new(&program_id, "swap");
    token::add_program(&mut mollusk);
    associated_token::add_program(&mut mollusk);

    let maker = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let maker_ata_gives = get_associated_token_address_with_program_id(&maker, &mint, &token::ID);
    let offer_id = 9u64;
    let (offer, _) = find_offer_pda(&maker, offer_id);
    let vault = get_associated_token_address_with_program_id(&offer, &mint, &token::ID);
    let instruction = make_offer_instruction(
        maker,
        mint,
        mint,
        maker_ata_gives,
        vault,
        offer,
        offer_id,
        OFFER_AMOUNT_GIVES,
        OFFER_AMOUNT_WANTS,
    );

    let original_maker_ata = token_account(&mint, &maker, INITIAL_MAKER_MINT_AMOUNT);

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (maker, system_account(10_000_000_000)),
            (mint, mint_account(&maker, INITIAL_MAKER_MINT_AMOUNT, DECIMALS)),
            (maker_ata_gives, original_maker_ata.clone()),
            (vault, empty_account()),
            (offer, empty_account()),
            associated_token::keyed_account(),
            token::keyed_account(),
            keyed_account_for_system_program(),
        ],
        &[
            Check::err(ProgramError::Custom(6001)),
            Check::account(&maker_ata_gives)
                .data(original_maker_ata.data())
                .owner(&token::ID)
                .build(),
            Check::account(&vault).closed().build(),
            Check::account(&offer).closed().build(),
        ],
    );
}
