use nssa::program::Program;
use nssa_core::{
    account::{Balance, Data, Nonce},
    program::BlockId,
};

use crate::Instruction;
use crate::{
    test_helpers::{
        assert_vault_state_unchanged, build_signed_public_tx, create_keypair,
        create_state_with_guest_program, derive_stream_pda, derive_vault_pdas,
        state_with_initialized_vault,
    },
    TokensPerSecond, VaultConfig, VaultHolding, VaultId, ERR_VAULT_ID_MISMATCH,
    ERR_VAULT_OWNER_MISMATCH, ERR_VERSION_MISMATCH, ERR_ZERO_DEPOSIT_AMOUNT,
};

use super::common::{
    assert_execution_failed_with_code, state_deposited_with_mock_clock,
    DEFAULT_MOCK_CLOCK_INITIAL_TS, DEFAULT_OWNER_GENESIS_BALANCE, DEFAULT_STREAM_TEST_DEPOSIT,
};

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
fn test_deposit_after_create_stream_succeeds() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let initial_deposit = DEFAULT_STREAM_TEST_DEPOSIT;
    let second_deposit = 50 as Balance;
    let allocation = 200 as Balance;
    let rate = 10 as TokensPerSecond;
    let (_, mock_clock_account_id) = create_keypair(73);
    let (_, provider_account_id) = create_keypair(40);

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_deposited_with_mock_clock(
        owner_balance_start,
        initial_deposit,
        mock_clock_account_id,
        DEFAULT_MOCK_CLOCK_INITIAL_TS,
    );

    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, 0);
    let account_ids_create = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        mock_clock_account_id,
    ];
    assert!(
        state
            .transition_from_public_transaction(
                &build_signed_public_tx(
                    program_id,
                    Instruction::CreateStream {
                        vault_id,
                        stream_id: 0,
                        provider: provider_account_id,
                        rate,
                        allocation,
                    },
                    &account_ids_create,
                    &[Nonce(2)],
                    &[&owner_private_key],
                ),
                3 as BlockId,
            )
            .is_ok(),
        "create_stream failed"
    );

    let vc_after_stream =
        VaultConfig::from_bytes(&state.get_account_by_id(vault_config_account_id).data)
            .expect("vault config");
    let stream_data_after_create = state.get_account_by_id(stream_pda).data.clone();
    let owner_before_second = state.get_account_by_id(owner_account_id).balance;
    let holding_before_second = state.get_account_by_id(vault_holding_account_id).balance;

    let account_ids_deposit = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
    ];
    let result = state.transition_from_public_transaction(
        &build_signed_public_tx(
            program_id,
            Instruction::Deposit {
                vault_id,
                amount: second_deposit,
                authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
            },
            &account_ids_deposit,
            &[Nonce(3)],
            &[&owner_private_key],
        ),
        4 as BlockId,
    );
    assert!(
        result.is_ok(),
        "second deposit failed: {:?}",
        result
    );

    let vc_after =
        VaultConfig::from_bytes(&state.get_account_by_id(vault_config_account_id).data)
            .expect("vault config");
    assert_eq!(vc_after.total_allocated, vc_after_stream.total_allocated);
    assert_eq!(vc_after.next_stream_id, vc_after_stream.next_stream_id);
    assert_eq!(
        state.get_account_by_id(owner_account_id).balance,
        owner_before_second - second_deposit
    );
    assert_eq!(
        state.get_account_by_id(vault_holding_account_id).balance,
        holding_before_second + second_deposit
    );
    assert_eq!(
        state.get_account_by_id(stream_pda).data,
        stream_data_after_create
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
    assert_execution_failed_with_code(result, ERR_ZERO_DEPOSIT_AMOUNT);

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
    assert_execution_failed_with_code(result, ERR_VAULT_ID_MISMATCH);

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
    // Failure is from the chained authenticated-transfer program, not a payment-streams `ERR_*`.
    assert!(result.is_err());

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
    // Insufficient balance is enforced inside authenticated-transfer, not a lez custom code.
    assert!(result.is_err());

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
    // Signer is `other`; they have not transacted yet (only `owner` ran init).
    let nonce_deposit = Nonce(0);
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
    assert_execution_failed_with_code(result, ERR_VAULT_OWNER_MISMATCH);

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
