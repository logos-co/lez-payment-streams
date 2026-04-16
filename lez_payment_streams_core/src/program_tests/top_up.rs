//! Guest-backed top-up stream tests.

use nssa_core::{
    account::{Balance, Nonce},
    program::BlockId,
};

use crate::{
    test_helpers::{create_keypair, force_mock_timestamp_account},
    MockTimestamp, StreamConfig, StreamState, Timestamp, TokensPerSecond,
    ERR_ALLOCATION_EXCEEDS_UNALLOCATED, ERR_ARITHMETIC_OVERFLOW, ERR_STREAM_CLOSED,
    ERR_ZERO_TOP_UP_AMOUNT,
};

use super::common::{
    assert_execution_failed_with_code, first_stream_accounts, force_stream_state_closed,
    signed_create_stream, signed_deposit, signed_pause_stream, signed_sync_stream,
    signed_top_up_stream, state_deposited_with_mock_clock, transition_ok,
    DEFAULT_MOCK_CLOCK_INITIAL_TS, DEFAULT_OWNER_GENESIS_BALANCE, DEFAULT_STREAM_TEST_DEPOSIT,
};

#[test]
fn test_topup_resumes() {
    let t0: Timestamp = 0;
    let t1: Timestamp = 100;
    let t2: Timestamp = 250;
    let (_, mock_clock_account_id) = create_keypair(201);
    let (_, provider_account_id) = create_keypair(202);

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

    let s_depleted_paused =
        StreamConfig::from_bytes(&state.get_account_by_id(stream_pda).data).expect("stream");
    assert_eq!(s_depleted_paused.state, StreamState::Paused);
    assert_eq!(s_depleted_paused.accrued, 100 as Balance);

    transition_ok(
        &mut state,
        &signed_deposit(
            program_id,
            vault_id,
            200 as Balance,
            vault_config_account_id,
            vault_holding_account_id,
            owner_account_id,
            Nonce(4),
            &owner_private_key,
        ),
        5 as BlockId,
        "deposit failed",
    );

    force_mock_timestamp_account(&mut state, mock_clock_account_id, MockTimestamp::new(t2));

    transition_ok(
        &mut state,
        &signed_top_up_stream(
            program_id,
            vault_id,
            stream_id,
            200 as Balance,
            &account_ids,
            Nonce(5),
            &owner_private_key,
        ),
        6 as BlockId,
        "top_up_stream failed",
    );

    let s_after_top_up =
        StreamConfig::from_bytes(&state.get_account_by_id(stream_pda).data).expect("stream");
    assert_eq!(s_after_top_up.state, StreamState::Active);
    assert_eq!(s_after_top_up.accrued_as_of, t2);
    assert_eq!(s_after_top_up.allocation, 300 as Balance);
    assert_eq!(s_after_top_up.accrued, 100 as Balance);

    let t3: Timestamp = 260;
    force_mock_timestamp_account(&mut state, mock_clock_account_id, MockTimestamp::new(t3));

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
        "sync_stream after top-up failed",
    );

    let s_after_follow_up_sync =
        StreamConfig::from_bytes(&state.get_account_by_id(stream_pda).data).expect("stream");
    assert_eq!(s_after_follow_up_sync.accrued, 200 as Balance);
    assert_eq!(s_after_follow_up_sync.state, StreamState::Active);
}

#[test]
fn test_topup_active_increases_allocation() {
    let t0: Timestamp = 10;
    let t1: Timestamp = 20;
    let (_, mock_clock_account_id) = create_keypair(203);
    let (_, provider_account_id) = create_keypair(204);

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

    force_mock_timestamp_account(&mut state, mock_clock_account_id, MockTimestamp::new(t1));

    transition_ok(
        &mut state,
        &signed_top_up_stream(
            program_id,
            vault_id,
            stream_id,
            100 as Balance,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
        "top_up_stream failed",
    );

    let s_after_top_up =
        StreamConfig::from_bytes(&state.get_account_by_id(stream_pda).data).expect("stream");
    assert_eq!(s_after_top_up.state, StreamState::Active);
    assert_eq!(s_after_top_up.allocation, 400 as Balance);
}

#[test]
fn test_topup_manual_pause_then_active() {
    let t0: Timestamp = 5;
    let t1: Timestamp = 15;
    let (_, mock_clock_account_id) = create_keypair(205);
    let (_, provider_account_id) = create_keypair(206);

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
            1 as TokensPerSecond,
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
        &signed_top_up_stream(
            program_id,
            vault_id,
            stream_id,
            50 as Balance,
            &account_ids,
            Nonce(4),
            &owner_private_key,
        ),
        5 as BlockId,
        "top_up_stream failed",
    );

    let s_resumed_after_top_up =
        StreamConfig::from_bytes(&state.get_account_by_id(stream_pda).data).expect("stream");
    assert_eq!(s_resumed_after_top_up.state, StreamState::Active);
    assert_eq!(s_resumed_after_top_up.accrued_as_of, t1);
    assert_eq!(s_resumed_after_top_up.allocation, 450 as Balance);
}

#[test]
fn test_topup_zero_fails() {
    let t0 = DEFAULT_MOCK_CLOCK_INITIAL_TS;
    let (_, mock_clock_account_id) = create_keypair(207);
    let (_, provider_account_id) = create_keypair(208);

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

    let r = state.transition_from_public_transaction(
        &signed_top_up_stream(
            program_id,
            vault_id,
            stream_id,
            0 as Balance,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_ZERO_TOP_UP_AMOUNT);
}

#[test]
fn test_topup_closed_fails() {
    let t0 = DEFAULT_MOCK_CLOCK_INITIAL_TS;
    let (_, mock_clock_account_id) = create_keypair(209);
    let (_, provider_account_id) = create_keypair(210);

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
            200 as Balance,
            &account_ids,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    force_stream_state_closed(&mut state, stream_pda);

    let r = state.transition_from_public_transaction(
        &signed_top_up_stream(
            program_id,
            vault_id,
            stream_id,
            10 as Balance,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_STREAM_CLOSED);
}

#[test]
fn test_topup_exceeds_unallocated_fails() {
    let t0 = DEFAULT_MOCK_CLOCK_INITIAL_TS;
    let (_, mock_clock_account_id) = create_keypair(211);
    let (_, provider_account_id) = create_keypair(212);

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
            DEFAULT_STREAM_TEST_DEPOSIT,
            &account_ids,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    let r = state.transition_from_public_transaction(
        &signed_top_up_stream(
            program_id,
            vault_id,
            stream_id,
            1 as Balance,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_ALLOCATION_EXCEEDS_UNALLOCATED);
}

#[test]
fn test_topup_allocation_overflow_fails() {
    let t0 = DEFAULT_MOCK_CLOCK_INITIAL_TS;
    let (_, mock_clock_account_id) = create_keypair(213);
    let (_, provider_account_id) = create_keypair(214);

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
            1 as TokensPerSecond,
            100 as Balance,
            &account_ids,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    let mut s_near_max_allocation =
        StreamConfig::from_bytes(&state.get_account_by_id(stream_pda).data).expect("stream");
    s_near_max_allocation.allocation = Balance::MAX - 5;
    s_near_max_allocation.accrued = 0 as Balance;
    let mut stream_account = state.get_account_by_id(stream_pda).clone();
    stream_account.data = nssa_core::account::Data::try_from(s_near_max_allocation.to_bytes())
        .expect("stream payload fits");
    state.force_insert_account(stream_pda, stream_account);

    let r = state.transition_from_public_transaction(
        &signed_top_up_stream(
            program_id,
            vault_id,
            stream_id,
            20 as Balance,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_ARITHMETIC_OVERFLOW);
}
