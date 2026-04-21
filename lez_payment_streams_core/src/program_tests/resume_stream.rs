//! `resume_stream` success and failure cases.

use nssa_core::{
    account::{Balance, Nonce},
    program::BlockId,
};

use crate::{
    test_helpers::{force_mock_timestamp_account, harness_mock_clock_and_provider_account_ids},
    MockTimestamp, StreamConfig, StreamState, Timestamp, TokensPerSecond,
    ERR_RESUME_ZERO_UNACCRUED, ERR_STREAM_NOT_PAUSED,
};

use super::common::{
    assert_execution_failed_with_code, first_stream_accounts, force_stream_state_closed,
    signed_create_stream, signed_pause_stream, signed_resume_stream, signed_sync_stream,
    state_deposited_with_mock_clock, transition_ok, DEFAULT_OWNER_GENESIS_BALANCE,
    DEFAULT_STREAM_TEST_DEPOSIT,
};

#[test]
fn test_resume() {
    let t0: Timestamp = 100;
    let t1: Timestamp = 200;
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
            5 as TokensPerSecond,
            400 as Balance,
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

    force_mock_timestamp_account(&mut state, mock_clock_account_id, MockTimestamp::new(t1));

    transition_ok(
        &mut state,
        &signed_resume_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(4),
            &owner_private_key,
        ),
        5 as BlockId,
        "resume_stream failed",
    );

    let s_resumed =
        StreamConfig::from_bytes(&state.get_account_by_id(stream_pda).data).expect("stream");
    assert_eq!(s_resumed.state, StreamState::Active);
    assert_eq!(s_resumed.accrued, 0 as Balance);
    assert_eq!(s_resumed.accrued_as_of, t1);
}

#[test]
fn test_resume_active_fails() {
    let t0: Timestamp = 50;
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

    let r = state.transition_from_public_transaction(
        &signed_resume_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_STREAM_NOT_PAUSED);
}

#[test]
fn test_resume_zero_remaining_fails() {
    let t0: Timestamp = 0;
    let t1: Timestamp = 100;
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

    force_mock_timestamp_account(&mut state, mock_clock_account_id, MockTimestamp::new(t1));

    transition_ok(
        &mut state,
        &signed_sync_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
        "sync_stream failed",
    );

    let r = state.transition_from_public_transaction(
        &signed_resume_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(4),
            &owner_private_key,
        ),
        5 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_RESUME_ZERO_UNACCRUED);
}

#[test]
fn test_resume_twice_fails() {
    let t0: Timestamp = 10;
    let t1: Timestamp = 20;
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
            500 as Balance,
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

    force_mock_timestamp_account(&mut state, mock_clock_account_id, MockTimestamp::new(t1));

    transition_ok(
        &mut state,
        &signed_resume_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(4),
            &owner_private_key,
        ),
        5 as BlockId,
        "resume_stream failed",
    );

    let r = state.transition_from_public_transaction(
        &signed_resume_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(5),
            &owner_private_key,
        ),
        6 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_STREAM_NOT_PAUSED);
}

#[test]
fn test_resume_closed_fails() {
    let t0: Timestamp = 8;
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
        &signed_resume_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_STREAM_NOT_PAUSED);
}

#[test]
fn test_resume_then_accrual_ignores_paused_gap() {
    let t0: Timestamp = 100;
    let t1: Timestamp = 105;
    let t_gap: Timestamp = 200;
    let t2: Timestamp = 210;
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

    let rate = 10 as TokensPerSecond;
    let allocation = 500 as Balance;
    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            stream_id,
            provider_account_id,
            rate,
            allocation,
            &account_ids,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    force_mock_timestamp_account(&mut state, mock_clock_account_id, MockTimestamp::new(t1));
    transition_ok(
        &mut state,
        &signed_sync_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
        "sync_stream failed",
    );

    transition_ok(
        &mut state,
        &signed_pause_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(4),
            &owner_private_key,
        ),
        5 as BlockId,
        "pause_stream failed",
    );

    force_mock_timestamp_account(&mut state, mock_clock_account_id, MockTimestamp::new(t_gap));
    transition_ok(
        &mut state,
        &signed_resume_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(5),
            &owner_private_key,
        ),
        6 as BlockId,
        "resume_stream failed",
    );

    force_mock_timestamp_account(&mut state, mock_clock_account_id, MockTimestamp::new(t2));
    transition_ok(
        &mut state,
        &signed_sync_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(6),
            &owner_private_key,
        ),
        7 as BlockId,
        "sync_stream after resume failed",
    );

    let s_after_resume_and_accrual =
        StreamConfig::from_bytes(&state.get_account_by_id(stream_pda).data).expect("stream");
    let expected_accrued = 50 + (u128::from(rate) * u128::from(t2 - t_gap));
    assert_eq!(s_after_resume_and_accrual.accrued, expected_accrued);
    assert_eq!(s_after_resume_and_accrual.accrued_as_of, t2);
    assert_eq!(s_after_resume_and_accrual.state, StreamState::Active);
}
