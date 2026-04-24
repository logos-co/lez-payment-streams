//! `close_stream` payouts and authorization.

use nssa_core::{
    account::{Balance, Nonce},
    BlockId,
};

use crate::{
    test_helpers::{create_keypair, derive_stream_pda, force_clock_account_monotonic},
    error_codes::ErrorCode, StreamConfig, StreamId, StreamState, Timestamp, TokensPerSecond,
    VaultConfig, CLOCK_01_PROGRAM_ACCOUNT_ID,
};

use super::common::{
    assert_execution_failed_with_code, force_stream_state_closed, signed_close_stream,
    signed_create_stream, signed_sync_stream, state_deposited_with_clock, transition_ok,
    CloseStreamIxAccounts, DEFAULT_CLOCK_INITIAL_TS, DEFAULT_OWNER_GENESIS_BALANCE,
    DEFAULT_STREAM_TEST_DEPOSIT,
};
use crate::harness_seeds::{SEED_ALT_SIGNER, SEED_PROVIDER};

#[test]
fn test_close_unaccrued_succeeds() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let allocation = 200 as Balance;
    let rate = 10 as TokensPerSecond;
    let t0: Timestamp = 12_345;
    let t1: Timestamp = t0 + 5;

    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    let (provider_private_key, provider_account_id) = create_keypair(SEED_PROVIDER);

    let mut dep = state_deposited_with_clock(owner_balance_start, deposit_amount, clock_id, t0);

    let stream_id = StreamId::MIN;
    let stream_pda = derive_stream_pda(
        dep.vault.program_id,
        dep.vault.vault_config_account_id,
        stream_id,
    );

    let stream_accounts = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        clock_id,
    ];
    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            rate,
            allocation,
            &stream_accounts,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    let vault_before = VaultConfig::from_bytes(
        &dep.vault
            .state
            .get_account_by_id(dep.vault.vault_config_account_id)
            .data,
    )
    .expect("vault config");
    assert_eq!(vault_before.total_allocated, allocation);

    force_clock_account_monotonic(&mut dep.vault.state, clock_id, 0, t1);

    transition_ok(
        &mut dep.vault.state,
        &signed_sync_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &stream_accounts,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        "sync_stream failed",
    );

    let close_accounts: CloseStreamIxAccounts = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        provider_account_id,
        clock_id,
    ];

    transition_ok(
        &mut dep.vault.state,
        &signed_close_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &close_accounts,
            Nonce(0),
            &provider_private_key,
        ),
        5 as BlockId,
        "close_stream failed",
    );

    let vault_after = VaultConfig::from_bytes(
        &dep.vault
            .state
            .get_account_by_id(dep.vault.vault_config_account_id)
            .data,
    )
    .expect("vault config");
    assert_eq!(vault_after.total_allocated, 50 as Balance);

    let stream_after =
        StreamConfig::from_bytes(&dep.vault.state.get_account_by_id(stream_pda).data)
            .expect("stream");
    assert_eq!(stream_after.state, StreamState::Closed);
    assert_eq!(stream_after.allocation, 50 as Balance);
    assert_eq!(stream_after.accrued, 50 as Balance);

    let holding_balance = dep
        .vault
        .state
        .get_account_by_id(dep.vault.vault_holding_account_id)
        .balance;
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

    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    let (_, provider_account_id) = create_keypair(SEED_PROVIDER);
    let (alt_signer_private_key, alt_signer_account_id) = create_keypair(SEED_ALT_SIGNER);

    let mut dep = state_deposited_with_clock(owner_balance_start, deposit_amount, clock_id, t0);

    let stream_id = StreamId::MIN;
    let stream_pda = derive_stream_pda(
        dep.vault.program_id,
        dep.vault.vault_config_account_id,
        stream_id,
    );

    let stream_accounts = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        clock_id,
    ];
    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            rate,
            allocation,
            &stream_accounts,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    force_clock_account_monotonic(&mut dep.vault.state, clock_id, 0, t1);

    transition_ok(
        &mut dep.vault.state,
        &signed_sync_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &stream_accounts,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        "sync_stream failed",
    );

    let close_accounts: CloseStreamIxAccounts = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        alt_signer_account_id,
        clock_id,
    ];

    let r = dep.vault.state.transition_from_public_transaction(
        &signed_close_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &close_accounts,
            Nonce(0),
            &alt_signer_private_key,
        ),
        5 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ErrorCode::CloseUnauthorized);
}

#[test]
fn test_close_already_closed_fails() {
    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    let (provider_private_key, provider_account_id) = create_keypair(SEED_PROVIDER);

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        DEFAULT_CLOCK_INITIAL_TS,
    );

    let stream_id = StreamId::MIN;
    let stream_pda = derive_stream_pda(
        dep.vault.program_id,
        dep.vault.vault_config_account_id,
        stream_id,
    );

    let stream_accounts = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        clock_id,
    ];
    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            1 as TokensPerSecond,
            100 as Balance,
            &stream_accounts,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    force_stream_state_closed(&mut dep.vault.state, stream_pda);

    let close_accounts: CloseStreamIxAccounts = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        provider_account_id,
        clock_id,
    ];

    let r = dep.vault.state.transition_from_public_transaction(
        &signed_close_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &close_accounts,
            Nonce(0),
            &provider_private_key,
        ),
        4 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ErrorCode::StreamClosed);
}
