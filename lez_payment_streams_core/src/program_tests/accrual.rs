//! Cover sync_stream plus [`crate::StreamConfig::at_time`] folding.

use nssa_core::{
    account::{Balance, Nonce},
    BlockId,
};

use crate::Instruction;
use crate::{
    test_helpers::{
        build_signed_public_tx, create_keypair, create_state_with_guest_program, derive_stream_pda,
        derive_vault_pdas, force_clock_account, harness_clock_01_and_provider_account_ids,
        patch_stream_config, patch_vault_config,
    },
    StreamConfig, StreamId, StreamState, Timestamp, TokensPerSecond, VaultId,
    ERR_STREAM_ID_MISMATCH, ERR_TIME_REGRESSION, ERR_VAULT_ID_MISMATCH, ERR_VAULT_OWNER_MISMATCH,
};

use super::common::{
    assert_execution_failed_with_code, signed_create_stream, signed_deposit, signed_sync_stream,
    state_deposited_with_clock, transition_ok, DEFAULT_CLOCK_INITIAL_TS,
    DEFAULT_OWNER_GENESIS_BALANCE, DEFAULT_STREAM_TEST_DEPOSIT,
};
use crate::harness_seeds::{SEED_ALT_SIGNER, SEED_OWNER, SEED_PROVIDER_B};

#[test]
fn test_accrual_basic() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let allocation = 200 as Balance;
    let rate = 10 as TokensPerSecond;
    let t0: Timestamp = 12_345;
    let t1: Timestamp = t0 + 5;

    let (mock_clock_account_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_deposited_with_clock(
        owner_balance_start,
        deposit_amount,
        mock_clock_account_id,
        t0,
    );

    let stream_id = StreamId::MIN;
    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, stream_id);

    let account_ids_stream = [
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
            &account_ids_stream,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    force_clock_account(&mut state, mock_clock_account_id, 0, t1);

    transition_ok(
        &mut state,
        &signed_sync_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids_stream,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
        "sync_stream failed",
    );

    let s_after_sync =
        StreamConfig::from_bytes(&state.get_account_by_id(stream_pda).data).expect("stream config");
    assert_eq!(s_after_sync.accrued, 50 as Balance);
    assert_eq!(s_after_sync.accrued_as_of, t1);
    assert_eq!(s_after_sync.state, StreamState::Active);
}

#[test]
fn test_accrual_caps_at_allocation() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let allocation = 100 as Balance;
    let rate = 10 as TokensPerSecond;
    let t0: Timestamp = 0;
    let t1: Timestamp = 100;

    let (mock_clock_account_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_deposited_with_clock(
        owner_balance_start,
        deposit_amount,
        mock_clock_account_id,
        t0,
    );

    let stream_id = StreamId::MIN;
    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, stream_id);
    let account_ids = [
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
            &account_ids,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    force_clock_account(&mut state, mock_clock_account_id, 0, t1);

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
        StreamConfig::from_bytes(&state.get_account_by_id(stream_pda).data).expect("parse");
    let expected_depletion_instant: Timestamp = (allocation / u128::from(rate)) as Timestamp;
    assert_eq!(s_depleted_paused.accrued, allocation);
    assert_eq!(s_depleted_paused.state, StreamState::Paused);
    assert_eq!(s_depleted_paused.accrued_as_of, expected_depletion_instant);
}

#[test]
fn test_sync_stream_second_stream_accrues() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let t0: Timestamp = 100;
    let t1: Timestamp = 105;
    let t2: Timestamp = 110;

    let rate0 = 10 as TokensPerSecond;
    let allocation0 = 200 as Balance;
    let rate1 = 5 as TokensPerSecond;
    let allocation1 = 100 as Balance;

    let (mock_clock_account_id, provider_a) = harness_clock_01_and_provider_account_ids();
    let (_, provider_b) = create_keypair(SEED_PROVIDER_B);

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_deposited_with_clock(
        owner_balance_start,
        deposit_amount,
        mock_clock_account_id,
        t0,
    );

    let stream0 = derive_stream_pda(program_id, vault_config_account_id, 0);
    let stream1 = derive_stream_pda(program_id, vault_config_account_id, 1);

    let accounts0 = [
        vault_config_account_id,
        vault_holding_account_id,
        stream0,
        owner_account_id,
        mock_clock_account_id,
    ];
    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            0,
            provider_a,
            rate0,
            allocation0,
            &accounts0,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "first create_stream failed",
    );

    let accounts1 = [
        vault_config_account_id,
        vault_holding_account_id,
        stream1,
        owner_account_id,
        mock_clock_account_id,
    ];
    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            1,
            provider_b,
            rate1,
            allocation1,
            &accounts1,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
        "second create_stream failed",
    );

    force_clock_account(&mut state, mock_clock_account_id, 0, t1);

    transition_ok(
        &mut state,
        &signed_sync_stream(
            program_id,
            vault_id,
            0,
            &accounts0,
            Nonce(4),
            &owner_private_key,
        ),
        5 as BlockId,
        "sync_stream stream 0 failed",
    );

    let s0_after_sync_t1 =
        StreamConfig::from_bytes(&state.get_account_by_id(stream0).data).expect("stream 0");
    assert_eq!(s0_after_sync_t1.accrued, 50 as Balance);
    assert_eq!(s0_after_sync_t1.accrued_as_of, t1);

    force_clock_account(&mut state, mock_clock_account_id, 0, t2);

    transition_ok(
        &mut state,
        &signed_sync_stream(
            program_id,
            vault_id,
            1,
            &accounts1,
            Nonce(5),
            &owner_private_key,
        ),
        6 as BlockId,
        "sync_stream stream 1 failed",
    );

    let s0_unchanged =
        StreamConfig::from_bytes(&state.get_account_by_id(stream0).data).expect("stream 0");
    assert_eq!(s0_unchanged.accrued, s0_after_sync_t1.accrued);
    assert_eq!(s0_unchanged.accrued_as_of, s0_after_sync_t1.accrued_as_of);

    let s1_after_sync_t2 =
        StreamConfig::from_bytes(&state.get_account_by_id(stream1).data).expect("stream 1");
    assert_eq!(s1_after_sync_t2.accrued, 50 as Balance);
    assert_eq!(s1_after_sync_t2.accrued_as_of, t2);
}

#[test]
fn test_sync_stream_wrong_vault_id_fails() {
    let (mock_clock_account_id, provider_account_id) = harness_clock_01_and_provider_account_ids();
    let t0: Timestamp = 50;

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        mock_clock_account_id,
        t0,
    );
    let stream_id = StreamId::MIN;
    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, stream_id);
    let account_ids = [
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
            1,
            400,
            &account_ids,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    patch_vault_config(&mut state, vault_config_account_id, |vc| {
        vc.vault_id = VaultId::from(999u64);
    });

    let r = state.transition_from_public_transaction(
        &signed_sync_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ERR_VAULT_ID_MISMATCH);
}

#[test]
fn test_sync_stream_owner_mismatch_fails() {
    let signer_account_balance = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let block_init = 1 as BlockId;
    let block_deposit = 2 as BlockId;
    let block_stream = 3 as BlockId;
    let block_sync = 4 as BlockId;
    let nonce_init = Nonce(0);
    let nonce_deposit = Nonce(1);
    let nonce_stream = Nonce(2);
    let nonce_sync = Nonce(3);

    let (owner_private_key, owner_account_id) = create_keypair(SEED_OWNER);
    let (_, alt_signer_account_id) = create_keypair(SEED_ALT_SIGNER);
    let (mock_clock_account_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let initial_accounts_data = vec![
        (owner_account_id, signer_account_balance),
        (alt_signer_account_id, signer_account_balance),
    ];
    let (mut state, guest_program) = create_state_with_guest_program(&initial_accounts_data)
        .expect(
            "guest image present (cargo build -p lez_payment_streams-methods) and state genesis ok",
        );
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
        &signed_deposit(
            program_id,
            vault_id,
            deposit_amount,
            vault_config_account_id,
            vault_holding_account_id,
            owner_account_id,
            nonce_deposit,
            &owner_private_key,
        ),
        block_deposit,
        "deposit failed",
    );

    force_clock_account(
        &mut state,
        mock_clock_account_id,
        0,
        DEFAULT_CLOCK_INITIAL_TS,
    );

    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, 0);
    let allocation = 100 as Balance;
    let rate = 1 as TokensPerSecond;

    let accounts_create = [
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
            0,
            provider_account_id,
            rate,
            allocation,
            &accounts_create,
            nonce_stream,
            &owner_private_key,
        ),
        block_stream,
        "create_stream failed",
    );

    patch_vault_config(&mut state, vault_config_account_id, |vc| {
        vc.owner = alt_signer_account_id;
    });

    let account_ids_sync = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        mock_clock_account_id,
    ];
    let r = state.transition_from_public_transaction(
        &signed_sync_stream(
            program_id,
            vault_id,
            0,
            &account_ids_sync,
            nonce_sync,
            &owner_private_key,
        ),
        block_sync,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ERR_VAULT_OWNER_MISMATCH);
}

#[test]
fn test_sync_stream_stream_id_does_not_match_account_fails() {
    let (mock_clock_account_id, provider_account_id) = harness_clock_01_and_provider_account_ids();
    let t0: Timestamp = 20;

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        mock_clock_account_id,
        t0,
    );
    let stream0 = derive_stream_pda(program_id, vault_config_account_id, 0);
    let account_ids = [
        vault_config_account_id,
        vault_holding_account_id,
        stream0,
        owner_account_id,
        mock_clock_account_id,
    ];

    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            0,
            provider_account_id,
            2,
            300,
            &account_ids,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    let stream1 = derive_stream_pda(program_id, vault_config_account_id, 1);
    let accounts_s1 = [
        vault_config_account_id,
        vault_holding_account_id,
        stream1,
        owner_account_id,
        mock_clock_account_id,
    ];
    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            1,
            provider_account_id,
            1,
            100,
            &accounts_s1,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
        "create_stream stream 1 failed",
    );

    patch_stream_config(&mut state, stream1, |sc| {
        sc.stream_id = 0;
    });

    let account_ids_sync_s1 = [
        vault_config_account_id,
        vault_holding_account_id,
        stream1,
        owner_account_id,
        mock_clock_account_id,
    ];
    let r = state.transition_from_public_transaction(
        &signed_sync_stream(
            program_id,
            vault_id,
            1,
            &account_ids_sync_s1,
            Nonce(4),
            &owner_private_key,
        ),
        5 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ERR_STREAM_ID_MISMATCH);
}

#[test]
fn test_sync_stream_time_regression_fails() {
    let (mock_clock_account_id, provider_account_id) = harness_clock_01_and_provider_account_ids();
    let t0: Timestamp = 100;
    let t_bad: Timestamp = 50;

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        mock_clock_account_id,
        t0,
    );
    let stream_id = StreamId::MIN;
    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, stream_id);
    let account_ids = [
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
            1,
            400,
            &account_ids,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    force_clock_account(&mut state, mock_clock_account_id, 0, t_bad);

    let r = state.transition_from_public_transaction(
        &signed_sync_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ERR_TIME_REGRESSION);
}
