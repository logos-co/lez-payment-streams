use nssa::program::Program;
use nssa_core::{
    account::{Balance, Data, Nonce},
    program::BlockId,
};

use crate::Instruction;
use crate::{
    test_helpers::{
        assert_vault_state_unchanged, build_signed_public_tx, create_keypair,
        create_state_with_guest_program, derive_vault_pdas, state_with_initialized_vault,
    },
    VaultConfig, VaultHolding, VaultId, ERR_VERSION_MISMATCH,
};

use super::common::{assert_execution_failed_with_code, DEFAULT_OWNER_GENESIS_BALANCE};

#[test]
fn test_deposit() {
    let owner_balance_before = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = 300 as Balance;
    let block_deposit = 2 as BlockId;
    let nonce_deposit = Nonce(1);

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_with_initialized_vault(owner_balance_before);
    let account_ids = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
    ];

    let vault_config_before = state.get_account_by_id(vault_config_account_id);
    let vault_config_state_before =
        VaultConfig::from_bytes(&vault_config_before.data).expect("valid vault config bytes");
    let instruction_deposit = Instruction::Deposit {
        vault_id,
        amount: deposit_amount,
        authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
    };
    let tx_deposit = build_signed_public_tx(
        program_id,
        instruction_deposit,
        &account_ids,
        &[nonce_deposit],
        &[&owner_private_key],
    );

    let vault_holding_balance_before = state.get_account_by_id(vault_holding_account_id).balance;

    let result_deposit = state.transition_from_public_transaction(&tx_deposit, block_deposit);
    assert!(
        result_deposit.is_ok(),
        "deposit tx failed: {:?}",
        result_deposit
    );

    let owner_balance_after = state.get_account_by_id(owner_account_id).balance;
    let vault_holding_balance_after = state.get_account_by_id(vault_holding_account_id).balance;
    let vault_config_after = state.get_account_by_id(vault_config_account_id);
    let vault_config_state_after =
        VaultConfig::from_bytes(&vault_config_after.data).expect("valid vault config bytes");

    assert_eq!(owner_balance_after, owner_balance_before - deposit_amount);
    assert_eq!(
        vault_holding_balance_after,
        vault_holding_balance_before + deposit_amount
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
}

#[test]
fn test_deposit_zero_amount_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let block_deposit = 2 as BlockId;
    let nonce_deposit = Nonce(1);
    let deposit_amount = 0 as Balance;

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_with_initialized_vault(owner_balance_start);
    let account_ids = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
    ];

    let owner_balance_before = state.get_account_by_id(owner_account_id).balance;
    let vault_holding_balance_before = state.get_account_by_id(vault_holding_account_id).balance;
    let vault_config_data_before = state
        .get_account_by_id(vault_config_account_id)
        .data
        .clone();

    let tx_deposit = build_signed_public_tx(
        program_id,
        Instruction::Deposit {
            vault_id,
            amount: deposit_amount,
            authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
        },
        &account_ids,
        &[nonce_deposit],
        &[&owner_private_key],
    );

    let result = state.transition_from_public_transaction(&tx_deposit, block_deposit);
    assert!(
        result.is_err(),
        "deposit with zero amount succeeded: {:?}",
        result
    );

    assert_vault_state_unchanged(
        &state,
        owner_account_id,
        vault_holding_account_id,
        vault_config_account_id,
        owner_balance_before,
        vault_holding_balance_before,
        vault_config_data_before,
    );
}

#[test]
fn test_deposit_wrong_vault_id_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let block_deposit = 2 as BlockId;
    let nonce_deposit = Nonce(1);
    let wrong_vault_id = VaultId::from(999u64);
    let deposit_amount = 100 as Balance;

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        _vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_with_initialized_vault(owner_balance_start);
    let account_ids = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
    ];

    let owner_balance_before = state.get_account_by_id(owner_account_id).balance;
    let vault_holding_balance_before = state.get_account_by_id(vault_holding_account_id).balance;
    let vault_config_data_before = state
        .get_account_by_id(vault_config_account_id)
        .data
        .clone();

    let tx_deposit = build_signed_public_tx(
        program_id,
        Instruction::Deposit {
            vault_id: wrong_vault_id,
            amount: deposit_amount,
            authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
        },
        &account_ids,
        &[nonce_deposit],
        &[&owner_private_key],
    );

    let result = state.transition_from_public_transaction(&tx_deposit, block_deposit);
    assert!(
        result.is_err(),
        "deposit with mismatched vault_id succeeded: {:?}",
        result
    );

    assert_vault_state_unchanged(
        &state,
        owner_account_id,
        vault_holding_account_id,
        vault_config_account_id,
        owner_balance_before,
        vault_holding_balance_before,
        vault_config_data_before,
    );
}

#[test]
fn test_deposit_wrong_authenticated_transfer_program_id_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let block_deposit = 2 as BlockId;
    let nonce_deposit = Nonce(1);
    let deposit_amount = 100 as Balance;

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_with_initialized_vault(owner_balance_start);
    let account_ids = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
    ];

    let owner_balance_before = state.get_account_by_id(owner_account_id).balance;
    let vault_holding_balance_before = state.get_account_by_id(vault_holding_account_id).balance;
    let vault_config_data_before = state
        .get_account_by_id(vault_config_account_id)
        .data
        .clone();

    // Chained transfer must target `Program::authenticated_transfer_program().id()`; using the
    // payment-streams guest `program_id` here is deliberately wrong and fails chained execution.
    let tx_deposit = build_signed_public_tx(
        program_id,
        Instruction::Deposit {
            vault_id,
            amount: deposit_amount,
            authenticated_transfer_program_id: program_id,
        },
        &account_ids,
        &[nonce_deposit],
        &[&owner_private_key],
    );

    let result = state.transition_from_public_transaction(&tx_deposit, block_deposit);
    assert!(
        result.is_err(),
        "deposit with wrong authenticated_transfer_program_id succeeded: {:?}",
        result
    );

    assert_vault_state_unchanged(
        &state,
        owner_account_id,
        vault_holding_account_id,
        vault_config_account_id,
        owner_balance_before,
        vault_holding_balance_before,
        vault_config_data_before,
    );
}

#[test]
fn test_deposit_insufficient_funds_fails() {
    let block_deposit = 2 as BlockId;
    let nonce_deposit = Nonce(1);
    let genesis_owner_balance = 100 as Balance;
    let deposit_amount = 200 as Balance;

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_with_initialized_vault(genesis_owner_balance);
    let account_ids = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
    ];

    let vault_holding_balance_before = state.get_account_by_id(vault_holding_account_id).balance;
    let vault_config_data_before = state
        .get_account_by_id(vault_config_account_id)
        .data
        .clone();

    let tx_deposit = build_signed_public_tx(
        program_id,
        Instruction::Deposit {
            vault_id,
            amount: deposit_amount,
            authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
        },
        &account_ids,
        &[nonce_deposit],
        &[&owner_private_key],
    );

    let result = state.transition_from_public_transaction(&tx_deposit, block_deposit);
    assert!(
        result.is_err(),
        "deposit larger than owner balance succeeded: {:?}",
        result
    );

    assert_vault_state_unchanged(
        &state,
        owner_account_id,
        vault_holding_account_id,
        vault_config_account_id,
        genesis_owner_balance,
        vault_holding_balance_before,
        vault_config_data_before,
    );
}

#[test]
fn test_deposit_owner_mismatch_fails() {
    let block_init = 1 as BlockId;
    let block_deposit = 2 as BlockId;
    let nonce_init = Nonce(0);
    let nonce_deposit = Nonce(1);
    let deposit_amount = 100 as Balance;
    let signer_account_balance = DEFAULT_OWNER_GENESIS_BALANCE;

    let (owner_private_key, owner_account_id) = create_keypair(1);
    let (other_private_key, other_account_id) = create_keypair(2);
    let initial_accounts_data = vec![
        (owner_account_id, signer_account_balance),
        (other_account_id, signer_account_balance),
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
    let result_init = state.transition_from_public_transaction(&tx_init, block_init);
    assert!(
        result_init.is_ok(),
        "initialize_vault tx failed: {:?}",
        result_init
    );

    let owner_balance_before = state.get_account_by_id(owner_account_id).balance;
    let other_balance_before = state.get_account_by_id(other_account_id).balance;
    let vault_holding_balance_before = state.get_account_by_id(vault_holding_account_id).balance;
    let vault_config_data_before = state
        .get_account_by_id(vault_config_account_id)
        .data
        .clone();

    let account_ids_deposit = [
        vault_config_account_id,
        vault_holding_account_id,
        other_account_id,
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
        &[&other_private_key],
    );

    let result = state.transition_from_public_transaction(&tx_deposit, block_deposit);
    assert!(
        result.is_err(),
        "deposit with non-owner funding account succeeded: {:?}",
        result
    );

    assert_vault_state_unchanged(
        &state,
        owner_account_id,
        vault_holding_account_id,
        vault_config_account_id,
        owner_balance_before,
        vault_holding_balance_before,
        vault_config_data_before,
    );
    assert_eq!(
        state.get_account_by_id(other_account_id).balance,
        other_balance_before
    );
}

#[test]
fn test_deposit_vault_holding_version_mismatch_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = 10 as Balance;
    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_with_initialized_vault(owner_balance_start);

    let mut holding = state.get_account_by_id(vault_holding_account_id).clone();
    holding.data = Data::try_from(VaultHolding::new_with_version(2).to_bytes())
        .expect("vault holding payload fits Data limits");
    state.force_insert_account(vault_holding_account_id, holding);

    let result = state.transition_from_public_transaction(
        &build_signed_public_tx(
            program_id,
            Instruction::Deposit {
                vault_id,
                amount: deposit_amount,
                authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
            },
            &[
                vault_config_account_id,
                vault_holding_account_id,
                owner_account_id,
            ],
            &[Nonce(1)],
            &[&owner_private_key],
        ),
        2 as BlockId,
    );
    assert_execution_failed_with_code(result, ERR_VERSION_MISMATCH);
}
