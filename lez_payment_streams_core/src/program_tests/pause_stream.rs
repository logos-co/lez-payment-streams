//! `pause_stream` success and failure cases.

use nssa::program::Program;
use nssa_core::{
    account::{Balance, Nonce},
    program::BlockId,
};

use crate::Instruction;
use crate::{
    test_helpers::{
        build_signed_public_tx, create_keypair, create_state_with_guest_program, derive_stream_pda,
        derive_vault_pdas, force_mock_timestamp_account, harness_mock_clock_and_provider_account_ids,
    },
    MockTimestamp, StreamConfig, StreamState, Timestamp, TokensPerSecond, VaultId,
    ERR_STREAM_NOT_ACTIVE, ERR_TIME_REGRESSION, ERR_VAULT_ID_MISMATCH, ERR_VAULT_OWNER_MISMATCH,
};

use super::common::{
    assert_execution_failed_with_code, first_stream_accounts, force_stream_state_closed,
    signed_create_stream, signed_pause_stream,
    state_deposited_with_mock_clock, transition_ok,
    DEFAULT_MOCK_CLOCK_INITIAL_TS, DEFAULT_OWNER_GENESIS_BALANCE, DEFAULT_STREAM_TEST_DEPOSIT,
};
use crate::harness_seeds::{SEED_ALT_SIGNER, SEED_OWNER};

#[test]
fn test_pause() {
    let t0: Timestamp = 12_345;
    let (mock_clock_account_id, provider_account_id) =
        harness_mock_clock_and_provider_account_ids();

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_deposited_with_mock_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        mock_clock_account_id,
        t0,
    );

    let (stream_id, stream_pda, account_ids) = first_stream_accounts(
        program_id,
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        mock_clock_account_id,
    );

    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            stream_id,
            provider_account_id,
            10 as TokensPerSecond,
            200 as Balance,
            &account_ids,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    transition_ok(
        &mut state,
        &signed_pause_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
        "pause_stream failed",
    );

    let s_paused =
        StreamConfig::from_bytes(&state.get_account_by_id(stream_pda).data).expect("stream");
    assert_eq!(s_paused.state, StreamState::Paused);
    assert_eq!(s_paused.accrued, 0 as Balance);
    assert_eq!(s_paused.accrued_as_of, t0);
}

#[test]
fn test_pause_twice_fails() {
    let t0: Timestamp = 1;
    let (mock_clock_account_id, provider_account_id) =
        harness_mock_clock_and_provider_account_ids();

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_deposited_with_mock_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        mock_clock_account_id,
        t0,
    );

    let (stream_id, _, account_ids) = first_stream_accounts(
        program_id,
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        mock_clock_account_id,
    );

    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            stream_id,
            provider_account_id,
            1 as TokensPerSecond,
            100 as Balance,
            &account_ids,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    transition_ok(
        &mut state,
        &signed_pause_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
        "first pause_stream failed",
    );

    let r = state.transition_from_public_transaction(
        &signed_pause_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(4),
            &owner_private_key,
        ),
        5 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_STREAM_NOT_ACTIVE);
}
#[test]
fn test_pause_when_at_time_depletes_fails() {
    let t0: Timestamp = 0;
    let t_deplete: Timestamp = 100;
    let (mock_clock_account_id, provider_account_id) =
        harness_mock_clock_and_provider_account_ids();

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_deposited_with_mock_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        mock_clock_account_id,
        t0,
    );

    let (stream_id, _, account_ids) = first_stream_accounts(
        program_id,
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        mock_clock_account_id,
    );

    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            stream_id,
            provider_account_id,
            10 as TokensPerSecond,
            100 as Balance,
            &account_ids,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    force_mock_timestamp_account(
        &mut state,
        mock_clock_account_id,
        MockTimestamp::new(t_deplete),
    );

    let r = state.transition_from_public_transaction(
        &signed_pause_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_STREAM_NOT_ACTIVE);
}
#[test]
fn test_pause_closed_fails() {
    let t0: Timestamp = 7;
    let (mock_clock_account_id, provider_account_id) =
        harness_mock_clock_and_provider_account_ids();

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_deposited_with_mock_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        mock_clock_account_id,
        t0,
    );

    let (stream_id, stream_pda, account_ids) = first_stream_accounts(
        program_id,
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        mock_clock_account_id,
    );

    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            stream_id,
            provider_account_id,
            2 as TokensPerSecond,
            300 as Balance,
            &account_ids,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    force_stream_state_closed(&mut state, stream_pda);

    let r = state.transition_from_public_transaction(
        &signed_pause_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_STREAM_NOT_ACTIVE);
}
#[test]
fn test_pause_stream_time_regression_fails() {
    let t0: Timestamp = 100;
    let t_bad: Timestamp = 50;
    let (mock_clock_account_id, provider_account_id) =
        harness_mock_clock_and_provider_account_ids();

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_deposited_with_mock_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        mock_clock_account_id,
        t0,
    );

    let (stream_id, _, account_ids) = first_stream_accounts(
        program_id,
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        mock_clock_account_id,
    );

    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            stream_id,
            provider_account_id,
            1 as TokensPerSecond,
            400 as Balance,
            &account_ids,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    force_mock_timestamp_account(&mut state, mock_clock_account_id, MockTimestamp::new(t_bad));

    let r = state.transition_from_public_transaction(
        &signed_pause_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_TIME_REGRESSION);
}
#[test]
fn test_pause_stream_owner_mismatch_fails() {
    let signer_account_balance = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let block_init = 1 as BlockId;
    let block_deposit = 2 as BlockId;
    let block_stream = 3 as BlockId;
    let block_pause = 4 as BlockId;
    let nonce_init = Nonce(0);
    let nonce_deposit = Nonce(1);
    let nonce_stream = Nonce(2);
    let nonce_pause = Nonce(0);

    let (owner_private_key, owner_account_id) = create_keypair(SEED_OWNER);
    let (alt_signer_private_key, alt_signer_account_id) = create_keypair(SEED_ALT_SIGNER);
    let (mock_clock_account_id, provider_account_id) =
        harness_mock_clock_and_provider_account_ids();

    let initial_accounts_data = vec![
        (owner_account_id, signer_account_balance),
        (alt_signer_account_id, signer_account_balance),
    ];
    let (mut state, guest_program) = create_state_with_guest_program(&initial_accounts_data)
        .expect("guest image present and state genesis ok");
    let program_id = guest_program.id();

    let vault_id = VaultId::from(1u64);
    let (vault_config_account_id, vault_holding_account_id) =
        derive_vault_pdas(program_id, owner_account_id, vault_id);
    transition_ok(
        &mut state,
        &build_signed_public_tx(
            program_id,
            Instruction::InitializeVault { vault_id },
            &[
                vault_config_account_id,
                vault_holding_account_id,
                owner_account_id,
            ],
            &[nonce_init],
            &[&owner_private_key],
        ),
        block_init,
        "initialize_vault failed",
    );

    transition_ok(
        &mut state,
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
            &[nonce_deposit],
            &[&owner_private_key],
        ),
        block_deposit,
        "deposit failed",
    );

    force_mock_timestamp_account(
        &mut state,
        mock_clock_account_id,
        MockTimestamp::new(DEFAULT_MOCK_CLOCK_INITIAL_TS),
    );

    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, 0);
    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            0,
            provider_account_id,
            1 as TokensPerSecond,
            100 as Balance,
            &[
                vault_config_account_id,
                vault_holding_account_id,
                stream_pda,
                owner_account_id,
                mock_clock_account_id,
            ],
            nonce_stream,
            &owner_private_key,
        ),
        block_stream,
        "create_stream failed",
    );

    let account_ids_pause = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        alt_signer_account_id,
        mock_clock_account_id,
    ];
    let r = state.transition_from_public_transaction(
        &signed_pause_stream(
            program_id,
            vault_id,
            0,
            &account_ids_pause,
            nonce_pause,
            &alt_signer_private_key,
        ),
        block_pause,
    );
    assert_execution_failed_with_code(r, ERR_VAULT_OWNER_MISMATCH);
}
#[test]
fn test_pause_stream_wrong_vault_id_fails() {
    let (mock_clock_account_id, provider_account_id) =
        harness_mock_clock_and_provider_account_ids();
    let t0: Timestamp = 50;

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        _vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_deposited_with_mock_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        mock_clock_account_id,
        t0,
    );
    let (stream_id, _, account_ids) = first_stream_accounts(
        program_id,
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        mock_clock_account_id,
    );

    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            _vault_id,
            stream_id,
            provider_account_id,
            1 as TokensPerSecond,
            400 as Balance,
            &account_ids,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    let wrong_vault_id = VaultId::from(999u64);
    let r = state.transition_from_public_transaction(
        &signed_pause_stream(
            program_id,
            wrong_vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_VAULT_ID_MISMATCH);
}
