use nssa::program::Program;
use nssa_core::{
    account::{Balance, Nonce},
    program::BlockId,
};

use crate::Instruction;
use crate::{
    test_helpers::{
        assert_vault_state_unchanged_with_recipient, build_signed_public_tx, create_keypair,
        create_state_with_guest_program, derive_vault_pdas,
        state_with_initialized_vault_with_recipient,
    },
    VaultConfig, VaultId, ERR_ARITHMETIC_OVERFLOW,
};

use super::common::{assert_execution_failed_with_code, DEFAULT_OWNER_GENESIS_BALANCE};

#[test]
fn test_withdraw() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = 400 as Balance;
    let withdraw_amount = 100 as Balance;
    let block_deposit = 2 as BlockId;
    let block_withdraw = 3 as BlockId;
    let nonce_deposit = Nonce(1);
    let nonce_withdraw = Nonce(2);

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        recipient_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_with_initialized_vault_with_recipient(owner_balance_start);

    let account_ids_deposit = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
    ];
    let tx_deposit = build_signed_public_tx(
        program_id,
        Instruction::Deposit {
            vault_id,
            amount: deposit_amount,
            authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
        },
        &account_ids_deposit,
        &[nonce_deposit],
        &[&owner_private_key],
    );
    let result_deposit = state.transition_from_public_transaction(&tx_deposit, block_deposit);
    assert!(
        result_deposit.is_ok(),
        "deposit tx failed: {:?}",
        result_deposit
    );

    let vault_config_before = state.get_account_by_id(vault_config_account_id);
    let vault_config_state_before =
        VaultConfig::from_bytes(&vault_config_before.data).expect("valid vault config bytes");
    let owner_after_deposit = state.get_account_by_id(owner_account_id).balance;
    let vault_holding_before_withdraw = state.get_account_by_id(vault_holding_account_id).balance;
    let recipient_before_withdraw = state.get_account_by_id(recipient_account_id).balance;

    let account_ids_withdraw = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        recipient_account_id,
    ];
    let tx_withdraw = build_signed_public_tx(
        program_id,
        Instruction::Withdraw {
            vault_id,
            amount: withdraw_amount,
        },
        &account_ids_withdraw,
        &[nonce_withdraw],
        &[&owner_private_key],
    );
    let result_withdraw = state.transition_from_public_transaction(&tx_withdraw, block_withdraw);
    assert!(
        result_withdraw.is_ok(),
        "withdraw tx failed: {:?}",
        result_withdraw
    );

    let vault_config_after = state.get_account_by_id(vault_config_account_id);
    let vault_config_state_after =
        VaultConfig::from_bytes(&vault_config_after.data).expect("valid vault config bytes");

    assert_eq!(
        state.get_account_by_id(owner_account_id).balance,
        owner_after_deposit
    );
    assert_eq!(
        state.get_account_by_id(vault_holding_account_id).balance,
        vault_holding_before_withdraw - withdraw_amount
    );
    assert_eq!(
        state.get_account_by_id(recipient_account_id).balance,
        recipient_before_withdraw + withdraw_amount
    );
    assert_eq!(
        vault_config_state_after.total_allocated,
        vault_config_state_before.total_allocated
    );
    assert_eq!(
        vault_config_state_after.next_stream_id,
        vault_config_state_before.next_stream_id
    );
    assert_eq!(
        vault_config_state_after.version,
        vault_config_state_before.version
    );
    assert_eq!(
        vault_config_state_after.owner,
        vault_config_state_before.owner
    );
    assert_eq!(
        vault_config_state_after.vault_id,
        vault_config_state_before.vault_id
    );
    assert_eq!(owner_after_deposit, owner_balance_start - deposit_amount);
}

#[test]
fn test_withdraw_zero_amount_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let withdraw_amount = 0 as Balance;
    let block_withdraw = 2 as BlockId;
    let nonce_withdraw = Nonce(1);

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        recipient_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_with_initialized_vault_with_recipient(owner_balance_start);
    let account_ids = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        recipient_account_id,
    ];

    let owner_balance_before = state.get_account_by_id(owner_account_id).balance;
    let vault_holding_balance_before = state.get_account_by_id(vault_holding_account_id).balance;
    let recipient_balance_before = state.get_account_by_id(recipient_account_id).balance;
    let vault_config_data_before = state
        .get_account_by_id(vault_config_account_id)
        .data
        .clone();

    let tx_withdraw = build_signed_public_tx(
        program_id,
        Instruction::Withdraw {
            vault_id,
            amount: withdraw_amount,
        },
        &account_ids,
        &[nonce_withdraw],
        &[&owner_private_key],
    );

    let result = state.transition_from_public_transaction(&tx_withdraw, block_withdraw);
    assert!(
        result.is_err(),
        "withdraw with zero amount succeeded: {:?}",
        result
    );

    assert_vault_state_unchanged_with_recipient(
        &state,
        owner_account_id,
        vault_holding_account_id,
        vault_config_account_id,
        recipient_account_id,
        owner_balance_before,
        vault_holding_balance_before,
        recipient_balance_before,
        vault_config_data_before,
    );
}

#[test]
fn test_withdraw_wrong_vault_id_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let withdraw_amount = 100 as Balance;
    let block_withdraw = 2 as BlockId;
    let nonce_withdraw = Nonce(1);
    let wrong_vault_id = VaultId::from(999u64);

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        recipient_account_id,
        _vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_with_initialized_vault_with_recipient(owner_balance_start);
    let account_ids = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        recipient_account_id,
    ];

    let owner_balance_before = state.get_account_by_id(owner_account_id).balance;
    let vault_holding_balance_before = state.get_account_by_id(vault_holding_account_id).balance;
    let recipient_balance_before = state.get_account_by_id(recipient_account_id).balance;
    let vault_config_data_before = state
        .get_account_by_id(vault_config_account_id)
        .data
        .clone();

    let tx_withdraw = build_signed_public_tx(
        program_id,
        Instruction::Withdraw {
            vault_id: wrong_vault_id,
            amount: withdraw_amount,
        },
        &account_ids,
        &[nonce_withdraw],
        &[&owner_private_key],
    );

    let result = state.transition_from_public_transaction(&tx_withdraw, block_withdraw);
    assert!(
        result.is_err(),
        "withdraw with mismatched vault_id succeeded: {:?}",
        result
    );

    assert_vault_state_unchanged_with_recipient(
        &state,
        owner_account_id,
        vault_holding_account_id,
        vault_config_account_id,
        recipient_account_id,
        owner_balance_before,
        vault_holding_balance_before,
        recipient_balance_before,
        vault_config_data_before,
    );
}

#[test]
fn test_withdraw_exceeds_unallocated_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let block_deposit = 2 as BlockId;
    let block_withdraw = 3 as BlockId;
    let nonce_deposit = Nonce(1);
    let nonce_withdraw = Nonce(2);
    let deposit_amount = 100 as Balance;
    let withdraw_amount = deposit_amount + 1 as Balance;

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        recipient_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_with_initialized_vault_with_recipient(owner_balance_start);

    let account_ids_deposit = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
    ];
    let tx_deposit = build_signed_public_tx(
        program_id,
        Instruction::Deposit {
            vault_id,
            amount: deposit_amount,
            authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
        },
        &account_ids_deposit,
        &[nonce_deposit],
        &[&owner_private_key],
    );
    assert!(
        state
            .transition_from_public_transaction(&tx_deposit, block_deposit)
            .is_ok(),
        "deposit failed"
    );

    let owner_balance_before = state.get_account_by_id(owner_account_id).balance;
    let vault_holding_balance_before = state.get_account_by_id(vault_holding_account_id).balance;
    let recipient_balance_before = state.get_account_by_id(recipient_account_id).balance;
    let vault_config_data_before = state
        .get_account_by_id(vault_config_account_id)
        .data
        .clone();

    let account_ids_withdraw = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        recipient_account_id,
    ];
    let tx_withdraw = build_signed_public_tx(
        program_id,
        Instruction::Withdraw {
            vault_id,
            amount: withdraw_amount,
        },
        &account_ids_withdraw,
        &[nonce_withdraw],
        &[&owner_private_key],
    );

    let result = state.transition_from_public_transaction(&tx_withdraw, block_withdraw);
    assert!(
        result.is_err(),
        "withdraw exceeding unallocated balance succeeded: {:?}",
        result
    );

    assert_vault_state_unchanged_with_recipient(
        &state,
        owner_account_id,
        vault_holding_account_id,
        vault_config_account_id,
        recipient_account_id,
        owner_balance_before,
        vault_holding_balance_before,
        recipient_balance_before,
        vault_config_data_before,
    );
}

#[test]
fn test_withdraw_owner_mismatch_fails() {
    let signer_account_balance = DEFAULT_OWNER_GENESIS_BALANCE;
    let recipient_genesis_balance = 0 as Balance;
    let withdraw_amount = 50 as Balance;
    let block_init = 1 as BlockId;
    let block_withdraw = 2 as BlockId;
    let nonce_init = Nonce(0);
    let nonce_withdraw = Nonce(1);

    let (owner_private_key, owner_account_id) = create_keypair(1);
    let (other_private_key, other_account_id) = create_keypair(2);
    let (_, recipient_account_id) = create_keypair(88);
    let initial_accounts_data = vec![
        (owner_account_id, signer_account_balance),
        (other_account_id, signer_account_balance),
        (recipient_account_id, recipient_genesis_balance),
    ];
    let (mut state, guest_program) = create_state_with_guest_program(&initial_accounts_data)
        .expect(
            "guest image present (cargo build -p lez_payment_streams-methods) and state genesis ok",
        );
    let program_id = guest_program.id();

    let vault_id = VaultId::from(1u64);
    let (vault_config_account_id, vault_holding_account_id) =
        derive_vault_pdas(program_id, owner_account_id, vault_id);
    let account_ids_init = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
    ];
    let tx_init = build_signed_public_tx(
        program_id,
        Instruction::InitializeVault { vault_id },
        &account_ids_init,
        &[nonce_init],
        &[&owner_private_key],
    );
    assert!(
        state
            .transition_from_public_transaction(&tx_init, block_init)
            .is_ok(),
        "initialize_vault tx failed"
    );

    let owner_balance_before = state.get_account_by_id(owner_account_id).balance;
    let other_balance_before = state.get_account_by_id(other_account_id).balance;
    let vault_holding_balance_before = state.get_account_by_id(vault_holding_account_id).balance;
    let recipient_balance_before = state.get_account_by_id(recipient_account_id).balance;
    let vault_config_data_before = state
        .get_account_by_id(vault_config_account_id)
        .data
        .clone();

    let account_ids_withdraw = [
        vault_config_account_id,
        vault_holding_account_id,
        other_account_id,
        recipient_account_id,
    ];
    let tx_withdraw = build_signed_public_tx(
        program_id,
        Instruction::Withdraw {
            vault_id,
            amount: withdraw_amount,
        },
        &account_ids_withdraw,
        &[nonce_withdraw],
        &[&other_private_key],
    );

    let result = state.transition_from_public_transaction(&tx_withdraw, block_withdraw);
    assert!(
        result.is_err(),
        "withdraw with non-owner signer slot succeeded: {:?}",
        result
    );

    assert_vault_state_unchanged_with_recipient(
        &state,
        owner_account_id,
        vault_holding_account_id,
        vault_config_account_id,
        recipient_account_id,
        owner_balance_before,
        vault_holding_balance_before,
        recipient_balance_before,
        vault_config_data_before,
    );
    assert_eq!(
        state.get_account_by_id(other_account_id).balance,
        other_balance_before
    );
}

#[test]
fn test_withdraw_recipient_balance_overflow_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = 100 as Balance;
    let withdraw_amount = 10 as Balance;
    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        recipient_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_with_initialized_vault_with_recipient(owner_balance_start);

    assert!(
        state
            .transition_from_public_transaction(
                &build_signed_public_tx(
                    program_id,
                    Instruction::Deposit {
                        vault_id,
                        amount: deposit_amount,
                        authenticated_transfer_program_id: Program::authenticated_transfer_program(
                        )
                        .id(),
                    },
                    &[
                        vault_config_account_id,
                        vault_holding_account_id,
                        owner_account_id
                    ],
                    &[Nonce(1)],
                    &[&owner_private_key],
                ),
                2 as BlockId,
            )
            .is_ok(),
        "deposit failed"
    );

    let mut recipient = state.get_account_by_id(recipient_account_id).clone();
    recipient.balance = Balance::MAX - 5;
    state.force_insert_account(recipient_account_id, recipient);

    let result = state.transition_from_public_transaction(
        &build_signed_public_tx(
            program_id,
            Instruction::Withdraw {
                vault_id,
                amount: withdraw_amount,
            },
            &[
                vault_config_account_id,
                vault_holding_account_id,
                owner_account_id,
                recipient_account_id,
            ],
            &[Nonce(2)],
            &[&owner_private_key],
        ),
        3 as BlockId,
    );
    assert_execution_failed_with_code(result, ERR_ARITHMETIC_OVERFLOW);
}
