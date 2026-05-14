//! Scenario checks for vault solvency and allocation conservation (plan step 4).

use nssa_core::{account::Nonce, BlockId};

use crate::{test_helpers::derive_stream_pda, StreamId, Timestamp};

use super::common::{
    assert_vault_conservation_invariants, first_stream_ix_accounts, signed_claim_stream,
    signed_create_stream, signed_pause_stream, signed_resume_stream, state_deposited_with_clock,
    state_deposited_with_clock_and_provider, transition_ok, ClaimStreamIxAccounts,
    DEFAULT_OWNER_GENESIS_BALANCE, DEFAULT_STREAM_TEST_DEPOSIT,
};
use crate::harness_seeds::SEED_PROVIDER;
use crate::test_helpers::{
    create_keypair, force_clock_account_monotonic, harness_clock_01_and_provider_account_ids,
};

/// After two streams exist and are paused (which folds accrual), holding covers `total_allocated`
/// and `total_allocated` equals the sum of stream `allocation` fields.
#[test]
fn test_solvency_two_streams_after_pause_fold_succeeds() {
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();
    let t0: Timestamp = 10;
    let t1: Timestamp = 20;

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        t0,
    );

    let (stream0, pda0, accounts0) = first_stream_ix_accounts(&dep);
    assert_eq!(stream0, StreamId::MIN);
    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            0,
            provider_account_id,
            1,
            100,
            &accounts0,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream 0",
    );

    let pda1 = derive_stream_pda(dep.vault.program_id, dep.vault.vault_config_account_id, 1);
    let accounts1 = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        pda1,
        dep.vault.owner_account_id,
        clock_id,
    ];
    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            1,
            provider_account_id,
            1,
            150,
            &accounts1,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        "create_stream 1",
    );

    force_clock_account_monotonic(&mut dep.vault.state, clock_id, 0, t1);

    let accounts_s0 = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        pda0,
        dep.vault.owner_account_id,
        clock_id,
    ];
    transition_ok(
        &mut dep.vault.state,
        &signed_pause_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            0,
            &accounts_s0,
            Nonce(4),
            &dep.vault.owner_private_key,
        ),
        5 as BlockId,
        "pause 0",
    );

    let accounts_s1 = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        pda1,
        dep.vault.owner_account_id,
        clock_id,
    ];
    transition_ok(
        &mut dep.vault.state,
        &signed_pause_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            1,
            &accounts_s1,
            Nonce(5),
            &dep.vault.owner_private_key,
        ),
        6 as BlockId,
        "pause 1",
    );

    assert_vault_conservation_invariants(&dep.vault.state, dep.vault.program_id, &dep.vault);
}

/// After `claim` pays the full accrued balance at `now` (here accrued is below `allocation`, so
/// the stream stays active and commitments shrink by that payout), solvency and allocation
/// conservation still hold.
#[test]
fn test_solvency_after_full_accrued_claim_succeeds() {
    let (clock_id, _) = harness_clock_01_and_provider_account_ids();
    let (provider_private_key, provider_account_id) = create_keypair(SEED_PROVIDER);
    let t0: Timestamp = 12_345;
    let t1: Timestamp = t0 + 5;

    let mut wp = state_deposited_with_clock_and_provider(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        t0,
        provider_private_key,
        provider_account_id,
    );
    let dep = &mut wp.deposited;

    let (stream_id, stream_pda, stream_ix) = first_stream_ix_accounts(dep);
    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            10,
            200,
            &stream_ix,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream",
    );

    force_clock_account_monotonic(&mut dep.vault.state, clock_id, 0, t1);

    let claim_accounts: ClaimStreamIxAccounts = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        provider_account_id,
        clock_id,
    ];
    transition_ok(
        &mut dep.vault.state,
        &signed_claim_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &claim_accounts,
            Nonce(0),
            &wp.provider_private_key,
        ),
        4 as BlockId,
        "claim",
    );

    assert_vault_conservation_invariants(&dep.vault.state, dep.vault.program_id, &dep.vault);
}

/// Pause and resume a stream; `total_allocated` and per-stream `allocation` stay aligned with
/// holding balance throughout.
#[test]
fn test_solvency_after_pause_and_resume_succeeds() {
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();
    let t0: Timestamp = 100;
    let t1: Timestamp = 105;

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        t0,
    );

    let (stream_id, _stream_pda, account_ids) = first_stream_ix_accounts(&dep);
    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            5,
            400,
            &account_ids,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream",
    );

    assert_vault_conservation_invariants(&dep.vault.state, dep.vault.program_id, &dep.vault);

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
        "pause_stream",
    );

    assert_vault_conservation_invariants(&dep.vault.state, dep.vault.program_id, &dep.vault);

    force_clock_account_monotonic(&mut dep.vault.state, clock_id, 0, t1);
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
        "resume_stream",
    );

    assert_vault_conservation_invariants(&dep.vault.state, dep.vault.program_id, &dep.vault);
}
