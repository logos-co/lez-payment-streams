//! Test `claim` payouts to the provider.

use nssa_core::{
    account::{Balance, Nonce},
    BlockId,
};

use crate::{
    test_helpers::create_keypair, StreamConfig, StreamState, Timestamp, TokensPerSecond,
    VaultConfig, CLOCK_01_PROGRAM_ACCOUNT_ID, ERR_CLAIM_UNAUTHORIZED, ERR_ZERO_CLAIM_AMOUNT,
};

use super::common::{
    assert_execution_failed_with_code, claim_stream_prelude_synced_at_t1, signed_claim_stream,
    signed_close_stream, transition_ok, ClaimStreamIxAccounts, CloseStreamIxAccounts,
    DEFAULT_OWNER_GENESIS_BALANCE, DEFAULT_STREAM_TEST_DEPOSIT,
};
use crate::harness_seeds::{SEED_ALT_SIGNER, SEED_PROVIDER};

const CLAIM_T0: Timestamp = 12_345;
const CLAIM_T1: Timestamp = CLAIM_T0 + 5;
const CLAIM_ALLOCATION: Balance = 200;
const CLAIM_RATE: TokensPerSecond = 10;

#[test]
fn test_claim_balance_succeeds() {
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    let (provider_private_key, provider_account_id) = create_keypair(SEED_PROVIDER);

    let mut scenario = claim_stream_prelude_synced_at_t1(
        DEFAULT_OWNER_GENESIS_BALANCE,
        deposit_amount,
        clock_id,
        CLAIM_T0,
        CLAIM_T1,
        provider_private_key,
        provider_account_id,
        CLAIM_RATE,
        CLAIM_ALLOCATION,
    );
    let wp = &mut scenario.with_provider;
    let stream_id = scenario.stream_id;
    let stream_pda = scenario.stream_pda;

    let provider_balance_before = wp
        .deposited
        .vault
        .state
        .get_account_by_id(wp.provider_account_id)
        .balance;

    let claim_accounts: ClaimStreamIxAccounts = [
        wp.deposited.vault.vault_config_account_id,
        wp.deposited.vault.vault_holding_account_id,
        stream_pda,
        wp.deposited.vault.owner_account_id,
        wp.provider_account_id,
        wp.deposited.clock_id,
    ];

    transition_ok(
        &mut wp.deposited.vault.state,
        &signed_claim_stream(
            wp.deposited.vault.program_id,
            wp.deposited.vault.vault_id,
            stream_id,
            &claim_accounts,
            Nonce(0),
            &wp.provider_private_key,
        ),
        5 as BlockId,
        "claim failed",
    );

    let payout = 50 as Balance;
    let vault_after = VaultConfig::from_bytes(
        &wp.deposited
            .vault
            .state
            .get_account_by_id(wp.deposited.vault.vault_config_account_id)
            .data,
    )
    .expect("vault config");
    assert_eq!(vault_after.total_allocated, CLAIM_ALLOCATION - payout);

    let stream_after =
        StreamConfig::from_bytes(&wp.deposited.vault.state.get_account_by_id(stream_pda).data)
            .expect("stream");
    assert_eq!(stream_after.state, StreamState::Active);
    assert_eq!(stream_after.accrued, 0 as Balance);
    assert_eq!(stream_after.allocation, CLAIM_ALLOCATION - payout);

    let holding_after = wp
        .deposited
        .vault
        .state
        .get_account_by_id(wp.deposited.vault.vault_holding_account_id)
        .balance;
    assert_eq!(holding_after, deposit_amount - payout);

    let provider_balance_after = wp
        .deposited
        .vault
        .state
        .get_account_by_id(wp.provider_account_id)
        .balance;
    assert_eq!(
        provider_balance_after,
        provider_balance_before.saturating_add(payout)
    );
}

#[test]
fn test_claim_unauthorized_fails() {
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    let (provider_private_key, provider_account_id) = create_keypair(SEED_PROVIDER);
    let (alt_signer_private_key, alt_signer_account_id) = create_keypair(SEED_ALT_SIGNER);

    let mut scenario = claim_stream_prelude_synced_at_t1(
        DEFAULT_OWNER_GENESIS_BALANCE,
        deposit_amount,
        clock_id,
        CLAIM_T0,
        CLAIM_T1,
        provider_private_key,
        provider_account_id,
        CLAIM_RATE,
        CLAIM_ALLOCATION,
    );
    let wp = &mut scenario.with_provider;
    let stream_id = scenario.stream_id;
    let stream_pda = scenario.stream_pda;

    let claim_accounts: ClaimStreamIxAccounts = [
        wp.deposited.vault.vault_config_account_id,
        wp.deposited.vault.vault_holding_account_id,
        stream_pda,
        wp.deposited.vault.owner_account_id,
        alt_signer_account_id,
        wp.deposited.clock_id,
    ];

    let r = wp.deposited.vault.state.transition_from_public_transaction(
        &signed_claim_stream(
            wp.deposited.vault.program_id,
            wp.deposited.vault.vault_id,
            stream_id,
            &claim_accounts,
            Nonce(0),
            &alt_signer_private_key,
        ),
        5 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ERR_CLAIM_UNAUTHORIZED);
}

#[test]
fn test_claim_after_close_succeeds() {
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    let (provider_private_key, provider_account_id) = create_keypair(SEED_PROVIDER);

    let mut scenario = claim_stream_prelude_synced_at_t1(
        DEFAULT_OWNER_GENESIS_BALANCE,
        deposit_amount,
        clock_id,
        CLAIM_T0,
        CLAIM_T1,
        provider_private_key,
        provider_account_id,
        CLAIM_RATE,
        CLAIM_ALLOCATION,
    );
    let wp = &mut scenario.with_provider;
    let stream_id = scenario.stream_id;
    let stream_pda = scenario.stream_pda;

    let close_accounts: CloseStreamIxAccounts = [
        wp.deposited.vault.vault_config_account_id,
        wp.deposited.vault.vault_holding_account_id,
        stream_pda,
        wp.deposited.vault.owner_account_id,
        wp.provider_account_id,
        wp.deposited.clock_id,
    ];

    transition_ok(
        &mut wp.deposited.vault.state,
        &signed_close_stream(
            wp.deposited.vault.program_id,
            wp.deposited.vault.vault_id,
            stream_id,
            &close_accounts,
            Nonce(0),
            &wp.provider_private_key,
        ),
        5 as BlockId,
        "close_stream failed",
    );

    let claim_accounts: ClaimStreamIxAccounts = [
        wp.deposited.vault.vault_config_account_id,
        wp.deposited.vault.vault_holding_account_id,
        stream_pda,
        wp.deposited.vault.owner_account_id,
        wp.provider_account_id,
        wp.deposited.clock_id,
    ];

    transition_ok(
        &mut wp.deposited.vault.state,
        &signed_claim_stream(
            wp.deposited.vault.program_id,
            wp.deposited.vault.vault_id,
            stream_id,
            &claim_accounts,
            Nonce(1),
            &wp.provider_private_key,
        ),
        6 as BlockId,
        "claim failed",
    );

    let vault_after = VaultConfig::from_bytes(
        &wp.deposited
            .vault
            .state
            .get_account_by_id(wp.deposited.vault.vault_config_account_id)
            .data,
    )
    .expect("vault config");
    assert_eq!(vault_after.total_allocated, 0 as Balance);

    let stream_after =
        StreamConfig::from_bytes(&wp.deposited.vault.state.get_account_by_id(stream_pda).data)
            .expect("stream");
    assert_eq!(stream_after.state, StreamState::Closed);
    assert_eq!(stream_after.allocation, 0 as Balance);
    assert_eq!(stream_after.accrued, 0 as Balance);

    let r = wp.deposited.vault.state.transition_from_public_transaction(
        &signed_claim_stream(
            wp.deposited.vault.program_id,
            wp.deposited.vault.vault_id,
            stream_id,
            &claim_accounts,
            Nonce(2),
            &wp.provider_private_key,
        ),
        7 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ERR_ZERO_CLAIM_AMOUNT);
}
