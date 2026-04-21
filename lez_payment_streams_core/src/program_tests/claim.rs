//! Test `claim` payouts to the provider.

use nssa_core::{
    account::{Balance, Nonce},
    program::BlockId,
};

use crate::{
    test_helpers::{create_keypair, derive_stream_pda, force_mock_timestamp_account},
    MockTimestamp, StreamConfig, StreamId, StreamState, Timestamp, TokensPerSecond, VaultConfig,
    ERR_CLAIM_UNAUTHORIZED, ERR_ZERO_CLAIM_AMOUNT,
};

use super::common::{
    assert_execution_failed_with_code, signed_claim_stream, signed_close_stream,
    signed_create_stream, signed_sync_stream, state_deposited_with_mock_clock_and_provider,
    transition_ok, ClaimStreamIxAccounts, CloseStreamIxAccounts, DEFAULT_OWNER_GENESIS_BALANCE,
    DEFAULT_STREAM_TEST_DEPOSIT,
};
use crate::harness_seeds::{SEED_ALT_SIGNER, SEED_MOCK_CLOCK, SEED_PROVIDER};

#[test]
fn test_claim_transfers_balance() {
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
    ) = state_deposited_with_mock_clock_and_provider(
        DEFAULT_OWNER_GENESIS_BALANCE,
        deposit_amount,
        mock_clock_account_id,
        t0,
        provider_account_id,
    );

    let stream_id = StreamId::MIN;
    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, stream_id);

    let stream_accounts_owner = [
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
            &stream_accounts_owner,
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
            &stream_accounts_owner,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
        "sync_stream failed",
    );

    let provider_balance_before = state.get_account_by_id(provider_account_id).balance;

    let claim_accounts: ClaimStreamIxAccounts = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        provider_account_id,
        mock_clock_account_id,
    ];

    transition_ok(
        &mut state,
        &signed_claim_stream(
            program_id,
            vault_id,
            stream_id,
            &claim_accounts,
            Nonce(0),
            &provider_private_key,
        ),
        5 as BlockId,
        "claim failed",
    );

    let payout = 50 as Balance;
    let vault_after =
        VaultConfig::from_bytes(&state.get_account_by_id(vault_config_account_id).data)
            .expect("vault config");
    assert_eq!(vault_after.total_allocated, allocation - payout);

    let stream_after =
        StreamConfig::from_bytes(&state.get_account_by_id(stream_pda).data).expect("stream");
    assert_eq!(stream_after.state, StreamState::Active);
    assert_eq!(stream_after.accrued, 0 as Balance);
    assert_eq!(stream_after.allocation, allocation - payout);

    let holding_after = state.get_account_by_id(vault_holding_account_id).balance;
    assert_eq!(holding_after, deposit_amount - payout);

    let provider_balance_after = state.get_account_by_id(provider_account_id).balance;
    assert_eq!(
        provider_balance_after,
        provider_balance_before.saturating_add(payout)
    );
}

#[test]
fn test_claim_unauthorized_fails() {
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
    ) = state_deposited_with_mock_clock_and_provider(
        DEFAULT_OWNER_GENESIS_BALANCE,
        deposit_amount,
        mock_clock_account_id,
        t0,
        provider_account_id,
    );

    let stream_id = StreamId::MIN;
    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, stream_id);

    let stream_accounts_owner = [
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
            &stream_accounts_owner,
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
            &stream_accounts_owner,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
        "sync_stream failed",
    );

    let claim_accounts: ClaimStreamIxAccounts = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        alt_signer_account_id,
        mock_clock_account_id,
    ];

    let r = state.transition_from_public_transaction(
        &signed_claim_stream(
            program_id,
            vault_id,
            stream_id,
            &claim_accounts,
            Nonce(0),
            &alt_signer_private_key,
        ),
        5 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_CLAIM_UNAUTHORIZED);
}

#[test]
fn test_claim_after_close() {
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
    ) = state_deposited_with_mock_clock_and_provider(
        DEFAULT_OWNER_GENESIS_BALANCE,
        deposit_amount,
        mock_clock_account_id,
        t0,
        provider_account_id,
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

    let claim_accounts: ClaimStreamIxAccounts = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        provider_account_id,
        mock_clock_account_id,
    ];

    transition_ok(
        &mut state,
        &signed_claim_stream(
            program_id,
            vault_id,
            stream_id,
            &claim_accounts,
            Nonce(1),
            &provider_private_key,
        ),
        6 as BlockId,
        "claim failed",
    );

    let vault_after =
        VaultConfig::from_bytes(&state.get_account_by_id(vault_config_account_id).data)
            .expect("vault config");
    assert_eq!(vault_after.total_allocated, 0 as Balance);

    let stream_after =
        StreamConfig::from_bytes(&state.get_account_by_id(stream_pda).data).expect("stream");
    assert_eq!(stream_after.state, StreamState::Closed);
    assert_eq!(stream_after.allocation, 0 as Balance);
    assert_eq!(stream_after.accrued, 0 as Balance);

    let r = state.transition_from_public_transaction(
        &signed_claim_stream(
            program_id,
            vault_id,
            stream_id,
            &claim_accounts,
            Nonce(2),
            &provider_private_key,
        ),
        7 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_ZERO_CLAIM_AMOUNT);
}
