//! `close_stream` payouts and authorization.

use nssa_core::{
    account::{Balance, Nonce},
    program::BlockId,
};

use crate::{
    test_helpers::{create_keypair, derive_stream_pda, force_mock_timestamp_account},
    MockTimestamp, StreamConfig, StreamId, StreamState, Timestamp, TokensPerSecond, VaultConfig,
    ERR_CLOSE_UNAUTHORIZED, ERR_STREAM_CLOSED,
};

use super::common::{
    assert_execution_failed_with_code, force_stream_state_closed, signed_close_stream,
    signed_create_stream, signed_sync_stream, state_deposited_with_mock_clock, transition_ok,
    CloseStreamIxAccounts, DEFAULT_MOCK_CLOCK_INITIAL_TS, DEFAULT_OWNER_GENESIS_BALANCE,
    DEFAULT_STREAM_TEST_DEPOSIT,
};
use crate::harness_seeds::{SEED_ALT_SIGNER, SEED_MOCK_CLOCK, SEED_PROVIDER};

#[test]
fn test_close_returns_unaccrued() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let allocation = 200 as Balance;
    let rate = 10 as TokensPerSecond;
    let t0: Timestamp = 12_345;
    let t1: Timestamp = t0 + 5;

    let (_, mock_clock_account_id) = create_keypair(SEED_MOCK_CLOCK);
    let (provider_private_key, provider_account_id) = create_keypair(SEED_PROVIDER);

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
        deposit_amount,
        mock_clock_account_id,
        t0,
    );

    let stream_id = StreamId::MIN;
    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, stream_id);

    let stream_accounts = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        mock_clock_account_id,
    ];
    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            stream_id,
            provider_account_id,
            rate,
            allocation,
            &stream_accounts,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    let vault_before =
        VaultConfig::from_bytes(&state.get_account_by_id(vault_config_account_id).data)
            .expect("vault config");
    assert_eq!(vault_before.total_allocated, allocation);

    force_mock_timestamp_account(&mut state, mock_clock_account_id, MockTimestamp::new(t1));

    transition_ok(
        &mut state,
        &signed_sync_stream(
            program_id,
            vault_id,
            stream_id,
            &stream_accounts,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
        "sync_stream failed",
    );

    let close_accounts: CloseStreamIxAccounts = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        provider_account_id,
        mock_clock_account_id,
    ];

    transition_ok(
        &mut state,
        &signed_close_stream(
            program_id,
            vault_id,
            stream_id,
            &close_accounts,
            Nonce(0),
            &provider_private_key,
        ),
        5 as BlockId,
        "close_stream failed",
    );

    let vault_after =
        VaultConfig::from_bytes(&state.get_account_by_id(vault_config_account_id).data)
            .expect("vault config");
    assert_eq!(vault_after.total_allocated, 50 as Balance);

    let stream_after =
        StreamConfig::from_bytes(&state.get_account_by_id(stream_pda).data).expect("stream");
    assert_eq!(stream_after.state, StreamState::Closed);
    assert_eq!(stream_after.allocation, 50 as Balance);
    assert_eq!(stream_after.accrued, 50 as Balance);

    let holding_balance = state.get_account_by_id(vault_holding_account_id).balance;
    let unallocated = holding_balance.saturating_sub(vault_after.total_allocated);
    assert_eq!(unallocated, 450 as Balance);
}

#[test]
fn test_close_stream_unauthorized_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let allocation = 200 as Balance;
    let rate = 10 as TokensPerSecond;
    let t0: Timestamp = 12_345;
    let t1: Timestamp = t0 + 5;

    let (_, mock_clock_account_id) = create_keypair(SEED_MOCK_CLOCK);
    let (_, provider_account_id) = create_keypair(SEED_PROVIDER);
    let (alt_signer_private_key, alt_signer_account_id) = create_keypair(SEED_ALT_SIGNER);

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
        deposit_amount,
        mock_clock_account_id,
        t0,
    );

    let stream_id = StreamId::MIN;
    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, stream_id);

    let stream_accounts = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        mock_clock_account_id,
    ];
    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            stream_id,
            provider_account_id,
            rate,
            allocation,
            &stream_accounts,
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
            &stream_accounts,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
        "sync_stream failed",
    );

    let close_accounts: CloseStreamIxAccounts = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        alt_signer_account_id,
        mock_clock_account_id,
    ];

    let r = state.transition_from_public_transaction(
        &signed_close_stream(
            program_id,
            vault_id,
            stream_id,
            &close_accounts,
            Nonce(0),
            &alt_signer_private_key,
        ),
        5 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_CLOSE_UNAUTHORIZED);
}

#[test]
fn test_close_already_closed_fails() {
    let (_, mock_clock_account_id) = create_keypair(SEED_MOCK_CLOCK);
    let (provider_private_key, provider_account_id) = create_keypair(SEED_PROVIDER);

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
        DEFAULT_MOCK_CLOCK_INITIAL_TS,
    );

    let stream_id = StreamId::MIN;
    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, stream_id);

    let stream_accounts = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        mock_clock_account_id,
    ];
    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            stream_id,
            provider_account_id,
            1 as TokensPerSecond,
            100 as Balance,
            &stream_accounts,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    force_stream_state_closed(&mut state, stream_pda);

    let close_accounts: CloseStreamIxAccounts = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        provider_account_id,
        mock_clock_account_id,
    ];

    let r = state.transition_from_public_transaction(
        &signed_close_stream(
            program_id,
            vault_id,
            stream_id,
            &close_accounts,
            Nonce(0),
            &provider_private_key,
        ),
        4 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_STREAM_CLOSED);
}
