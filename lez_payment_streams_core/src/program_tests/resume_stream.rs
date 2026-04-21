//! `resume_stream` success and failure cases.

use nssa_core::{
    account::{Balance, Nonce},
    BlockId,
};

use crate::{
    test_helpers::{force_clock_account, harness_clock_01_and_provider_account_ids},
    StreamConfig, StreamState, Timestamp, TokensPerSecond, ERR_RESUME_ZERO_UNACCRUED,
    ERR_STREAM_NOT_PAUSED,
};

use super::common::{
    assert_execution_failed_with_code, first_stream_ix_accounts, force_stream_state_closed,
    signed_create_stream, signed_pause_stream, signed_resume_stream, signed_sync_stream,
    state_deposited_with_clock, transition_ok, DEFAULT_OWNER_GENESIS_BALANCE,
    DEFAULT_STREAM_TEST_DEPOSIT,
};

#[test]
fn test_resume() {
    let t0: Timestamp = 100;
    let t1: Timestamp = 200;
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
            5 as TokensPerSecond,
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
        &signed_resume_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &account_ids,
            Nonce(4),
            &dep.vault.owner_private_key,
        ),
        5 as BlockId,
        "resume_stream failed",
    );

    let s_resumed =
        StreamConfig::from_bytes(&dep.vault.state.get_account_by_id(stream_pda).data).expect("stream");
    assert_eq!(s_resumed.state, StreamState::Active);
    assert_eq!(s_resumed.accrued, 0 as Balance);
    assert_eq!(s_resumed.accrued_as_of, t1);
}

#[test]
fn test_resume_active_fails() {
    let t0: Timestamp = 50;
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
            400 as Balance,
            &account_ids,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    let r = dep.vault.state.transition_from_public_transaction(
        &signed_resume_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ERR_STREAM_NOT_PAUSED);
}

#[test]
fn test_resume_zero_remaining_fails() {
    let t0: Timestamp = 0;
    let t1: Timestamp = 100;
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

    let r = dep.vault.state.transition_from_public_transaction(
        &signed_resume_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &account_ids,
            Nonce(4),
            &dep.vault.owner_private_key,
        ),
        5 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ERR_RESUME_ZERO_UNACCRUED);
}

#[test]
fn test_resume_twice_fails() {
    let t0: Timestamp = 10;
    let t1: Timestamp = 20;
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
            500 as Balance,
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
        &signed_resume_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &account_ids,
            Nonce(4),
            &dep.vault.owner_private_key,
        ),
        5 as BlockId,
        "resume_stream failed",
    );

    let r = dep.vault.state.transition_from_public_transaction(
        &signed_resume_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &account_ids,
            Nonce(5),
            &dep.vault.owner_private_key,
        ),
        6 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ERR_STREAM_NOT_PAUSED);
}

#[test]
fn test_resume_closed_fails() {
    let t0: Timestamp = 8;
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

    force_stream_state_closed(&mut dep.vault.state, stream_pda);

    let r = dep.vault.state.transition_from_public_transaction(
        &signed_resume_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ERR_STREAM_NOT_PAUSED);
}

#[test]
fn test_resume_then_accrual_ignores_paused_gap() {
    let t0: Timestamp = 100;
    let t1: Timestamp = 105;
    let t_gap: Timestamp = 200;
    let t2: Timestamp = 210;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        t0,
    );

    let (stream_id, stream_pda, account_ids) = first_stream_ix_accounts(&dep);

    let rate = 10 as TokensPerSecond;
    let allocation = 500 as Balance;
    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            rate,
            allocation,
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

    transition_ok(
        &mut dep.vault.state,
        &signed_pause_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &account_ids,
            Nonce(4),
            &dep.vault.owner_private_key,
        ),
        5 as BlockId,
        "pause_stream failed",
    );

    force_clock_account(&mut dep.vault.state, clock_id, 0, t_gap);
    transition_ok(
        &mut dep.vault.state,
        &signed_resume_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &account_ids,
            Nonce(5),
            &dep.vault.owner_private_key,
        ),
        6 as BlockId,
        "resume_stream failed",
    );

    force_clock_account(&mut dep.vault.state, clock_id, 0, t2);
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
        "sync_stream after resume failed",
    );

    let s_after_resume_and_accrual =
        StreamConfig::from_bytes(&dep.vault.state.get_account_by_id(stream_pda).data).expect("stream");
    let expected_accrued = 50 + (u128::from(rate) * u128::from(t2 - t_gap));
    assert_eq!(s_after_resume_and_accrual.accrued, expected_accrued);
    assert_eq!(s_after_resume_and_accrual.accrued_as_of, t2);
    assert_eq!(s_after_resume_and_accrual.state, StreamState::Active);
}
