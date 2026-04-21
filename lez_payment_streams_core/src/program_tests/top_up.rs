//! `top_up_stream` allocation increases and pause handling.

use nssa_core::{
    account::{Balance, Nonce},
    BlockId,
};

use crate::{
    test_helpers::{force_clock_account, harness_clock_01_and_provider_account_ids},
    StreamConfig, StreamState, Timestamp, TokensPerSecond, ERR_ALLOCATION_EXCEEDS_UNALLOCATED,
    ERR_ARITHMETIC_OVERFLOW, ERR_STREAM_CLOSED, ERR_ZERO_TOP_UP_AMOUNT,
};

use super::common::{
    assert_execution_failed_with_code, first_stream_ix_accounts, force_stream_state_closed,
    signed_create_stream, signed_deposit, signed_pause_stream, signed_sync_stream,
    signed_top_up_stream, state_deposited_with_clock, transition_ok, DEFAULT_CLOCK_INITIAL_TS,
    DEFAULT_OWNER_GENESIS_BALANCE, DEFAULT_STREAM_TEST_DEPOSIT,
};

#[test]
fn test_topup_resumes() {
    let t0: Timestamp = 0;
    let t1: Timestamp = 100;
    let t2: Timestamp = 250;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        t0,
    );

    let (stream_id, stream_pda, account_ids) = first_stream_ix_accounts(&dep);

    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            10 as TokensPerSecond,
            100 as Balance,
            &account_ids,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    force_clock_account(&mut dep.vault.state, clock_id, 0, t1);

    transition_ok(
        &mut dep.vault.state,
        &signed_sync_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        "sync_stream failed",
    );

    let s_depleted_paused =
        StreamConfig::from_bytes(&dep.vault.state.get_account_by_id(stream_pda).data).expect("stream");
    assert_eq!(s_depleted_paused.state, StreamState::Paused);
    assert_eq!(s_depleted_paused.accrued, 100 as Balance);

    transition_ok(
        &mut dep.vault.state,
        &signed_deposit(
            dep.vault.program_id,
            dep.vault.vault_id,
            200 as Balance,
            dep.vault.vault_config_account_id,
            dep.vault.vault_holding_account_id,
            dep.vault.owner_account_id,
            Nonce(4),
            &dep.vault.owner_private_key,
        ),
        5 as BlockId,
        "deposit failed",
    );

    force_clock_account(&mut dep.vault.state, clock_id, 0, t2);

    transition_ok(
        &mut dep.vault.state,
        &signed_top_up_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            200 as Balance,
            &account_ids,
            Nonce(5),
            &dep.vault.owner_private_key,
        ),
        6 as BlockId,
        "top_up_stream failed",
    );

    let s_after_top_up =
        StreamConfig::from_bytes(&dep.vault.state.get_account_by_id(stream_pda).data).expect("stream");
    assert_eq!(s_after_top_up.state, StreamState::Active);
    assert_eq!(s_after_top_up.accrued_as_of, t2);
    assert_eq!(s_after_top_up.allocation, 300 as Balance);
    assert_eq!(s_after_top_up.accrued, 100 as Balance);

    let t3: Timestamp = 260;
    force_clock_account(&mut dep.vault.state, clock_id, 0, t3);

    transition_ok(
        &mut dep.vault.state,
        &signed_sync_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &account_ids,
            Nonce(6),
            &dep.vault.owner_private_key,
        ),
        7 as BlockId,
        "sync_stream after top-up failed",
    );

    let s_after_follow_up_sync =
        StreamConfig::from_bytes(&dep.vault.state.get_account_by_id(stream_pda).data).expect("stream");
    assert_eq!(s_after_follow_up_sync.accrued, 200 as Balance);
    assert_eq!(s_after_follow_up_sync.state, StreamState::Active);
}

#[test]
fn test_topup_active_increases_allocation() {
    let t0: Timestamp = 10;
    let t1: Timestamp = 20;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        t0,
    );

    let (stream_id, stream_pda, account_ids) = first_stream_ix_accounts(&dep);

    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            2 as TokensPerSecond,
            300 as Balance,
            &account_ids,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    force_clock_account(&mut dep.vault.state, clock_id, 0, t1);

    transition_ok(
        &mut dep.vault.state,
        &signed_top_up_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            100 as Balance,
            &account_ids,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        "top_up_stream failed",
    );

    let s_after_top_up =
        StreamConfig::from_bytes(&dep.vault.state.get_account_by_id(stream_pda).data).expect("stream");
    assert_eq!(s_after_top_up.state, StreamState::Active);
    assert_eq!(s_after_top_up.allocation, 400 as Balance);
}

#[test]
fn test_topup_manual_pause_then_active() {
    let t0: Timestamp = 5;
    let t1: Timestamp = 15;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        t0,
    );

    let (stream_id, stream_pda, account_ids) = first_stream_ix_accounts(&dep);

    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            1 as TokensPerSecond,
            400 as Balance,
            &account_ids,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    transition_ok(
        &mut dep.vault.state,
        &signed_pause_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        "pause_stream failed",
    );

    force_clock_account(&mut dep.vault.state, clock_id, 0, t1);

    transition_ok(
        &mut dep.vault.state,
        &signed_top_up_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            50 as Balance,
            &account_ids,
            Nonce(4),
            &dep.vault.owner_private_key,
        ),
        5 as BlockId,
        "top_up_stream failed",
    );

    let s_resumed_after_top_up =
        StreamConfig::from_bytes(&dep.vault.state.get_account_by_id(stream_pda).data).expect("stream");
    assert_eq!(s_resumed_after_top_up.state, StreamState::Active);
    assert_eq!(s_resumed_after_top_up.accrued_as_of, t1);
    assert_eq!(s_resumed_after_top_up.allocation, 450 as Balance);
}

#[test]
fn test_topup_zero_fails() {
    let t0 = DEFAULT_CLOCK_INITIAL_TS;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        t0,
    );

    let (stream_id, _, account_ids) = first_stream_ix_accounts(&dep);

    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            1 as TokensPerSecond,
            100 as Balance,
            &account_ids,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    let r = dep.vault.state.transition_from_public_transaction(
        &signed_top_up_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            0 as Balance,
            &account_ids,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ERR_ZERO_TOP_UP_AMOUNT);
}

#[test]
fn test_topup_closed_fails() {
    let t0 = DEFAULT_CLOCK_INITIAL_TS;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        t0,
    );

    let (stream_id, stream_pda, account_ids) = first_stream_ix_accounts(&dep);

    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            2 as TokensPerSecond,
            200 as Balance,
            &account_ids,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    force_stream_state_closed(&mut dep.vault.state, stream_pda);

    let r = dep.vault.state.transition_from_public_transaction(
        &signed_top_up_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            10 as Balance,
            &account_ids,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ERR_STREAM_CLOSED);
}

#[test]
fn test_topup_exceeds_unallocated_fails() {
    let t0 = DEFAULT_CLOCK_INITIAL_TS;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        t0,
    );

    let (stream_id, _, account_ids) = first_stream_ix_accounts(&dep);

    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            1 as TokensPerSecond,
            DEFAULT_STREAM_TEST_DEPOSIT,
            &account_ids,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    let r = dep.vault.state.transition_from_public_transaction(
        &signed_top_up_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            1 as Balance,
            &account_ids,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ERR_ALLOCATION_EXCEEDS_UNALLOCATED);
}

#[test]
fn test_topup_allocation_overflow_fails() {
    let t0 = DEFAULT_CLOCK_INITIAL_TS;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        t0,
    );

    let (stream_id, stream_pda, account_ids) = first_stream_ix_accounts(&dep);

    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            1 as TokensPerSecond,
            100 as Balance,
            &account_ids,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    let mut s_near_max_allocation =
        StreamConfig::from_bytes(&dep.vault.state.get_account_by_id(stream_pda).data).expect("stream");
    s_near_max_allocation.allocation = Balance::MAX - 5;
    s_near_max_allocation.accrued = 0 as Balance;
    let mut stream_account = dep.vault.state.get_account_by_id(stream_pda).clone();
    stream_account.data = nssa_core::account::Data::try_from(s_near_max_allocation.to_bytes())
        .expect("stream payload fits");
    dep.vault.state.force_insert_account(stream_pda, stream_account);

    let r = dep.vault.state.transition_from_public_transaction(
        &signed_top_up_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            20 as Balance,
            &account_ids,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ERR_ARITHMETIC_OVERFLOW);
}
