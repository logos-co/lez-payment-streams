//! `create_stream` integration tests (happy paths, bounds, account layout).

use nssa_core::{
    account::{Account, Balance, Data, Nonce},
    program::BlockId,
};

use crate::Instruction;
use crate::{
    test_helpers::{
        build_signed_public_tx, create_keypair, create_state_with_guest_program, derive_stream_pda,
        derive_vault_pdas, force_mock_timestamp_account,
        harness_mock_clock_and_provider_account_ids, state_with_initialized_vault,
    },
    MockTimestamp, StreamConfig, StreamId, StreamState, Timestamp, TokensPerSecond, VaultConfig,
    VaultId, DEFAULT_VERSION, ERR_ALLOCATION_EXCEEDS_UNALLOCATED, ERR_INVALID_MOCK_TIMESTAMP,
    ERR_NEXT_STREAM_ID_OVERFLOW, ERR_STREAM_ID_MISMATCH, ERR_VAULT_ID_MISMATCH,
    ERR_VAULT_OWNER_MISMATCH, ERR_ZERO_STREAM_ALLOCATION, ERR_ZERO_STREAM_RATE,
};

use super::common::{
    assert_execution_failed_with_code, signed_create_stream, signed_deposit,
    state_deposited_with_mock_clock, transition_ok, DEFAULT_MOCK_CLOCK_INITIAL_TS,
    DEFAULT_OWNER_GENESIS_BALANCE, DEFAULT_STREAM_TEST_DEPOSIT,
};
use crate::harness_seeds::{SEED_ALT_SIGNER, SEED_OWNER, SEED_PROVIDER_B};

#[test]
fn test_derive_stream_pda_stable() {
    let owner_genesis_balance = DEFAULT_OWNER_GENESIS_BALANCE;
    let (_, owner_account_id) = create_keypair(SEED_OWNER);
    let initial_accounts_data = vec![(owner_account_id, owner_genesis_balance)];
    let (_, guest_program) = create_state_with_guest_program(&initial_accounts_data).expect(
        "guest image present (cargo build -p lez_payment_streams-methods) and state genesis ok",
    );
    let program_id = guest_program.id();
    let vault_id = VaultId::from(1u64);
    let (vault_config_account_id, _) = derive_vault_pdas(program_id, owner_account_id, vault_id);
    let s0 = derive_stream_pda(program_id, vault_config_account_id, 0);
    let s0_b = derive_stream_pda(program_id, vault_config_account_id, 0);
    assert_eq!(s0, s0_b);
    assert_ne!(
        s0,
        derive_stream_pda(program_id, vault_config_account_id, 1)
    );
}

#[test]
fn test_create_stream() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let allocation = 200 as Balance;
    let rate = 10 as TokensPerSecond;
    let block_deposit = 2 as BlockId;
    let block_stream = 3 as BlockId;
    let nonce_deposit = Nonce(1);
    let nonce_stream = Nonce(2);

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
    ) = state_with_initialized_vault(owner_balance_start);

    transition_ok(
        &mut state,
        &signed_deposit(
            program_id,
            vault_id,
            amount,
            vault_config_account_id,
            vault_holding_account_id,
            owner_account_id,
            nonce_deposit,
            &owner_private_key,
        ),
        block_deposit,
        "deposit failed",
    );

    let initial_ts: Timestamp = 12_345;
    force_mock_timestamp_account(
        &mut state,
        mock_clock_account_id,
        MockTimestamp::new(initial_ts),
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
    let tx_stream = signed_create_stream(
        program_id,
        vault_id,
        stream_id,
        provider_account_id,
        rate,
        allocation,
        &account_ids_stream,
        nonce_stream,
        &owner_private_key,
    );
    let result_stream = state.transition_from_public_transaction(&tx_stream, block_stream);
    assert!(
        result_stream.is_ok(),
        "create_stream tx failed: {:?}",
        result_stream
    );

    let vault_config =
        VaultConfig::from_bytes(&state.get_account_by_id(vault_config_account_id).data)
            .expect("vault config");
    assert_eq!(vault_config.next_stream_id, 1);
    assert_eq!(vault_config.total_allocated, allocation);

    let stream_data = state.get_account_by_id(stream_pda).data.clone();
    let stream_cfg = StreamConfig::from_bytes(&stream_data).expect("stream config");
    assert_eq!(stream_cfg.version, DEFAULT_VERSION);
    assert_eq!(stream_cfg.stream_id, stream_id);
    assert_eq!(stream_cfg.provider, provider_account_id);
    assert_eq!(stream_cfg.rate, rate);
    assert_eq!(stream_cfg.allocation, allocation);
    assert_eq!(stream_cfg.accrued, 0 as Balance);
    assert_eq!(stream_cfg.state, StreamState::Active);
    assert_eq!(stream_cfg.accrued_as_of, initial_ts);

    assert_eq!(
        state.get_account_by_id(vault_holding_account_id).balance,
        amount
    );
}

#[test]
fn test_create_stream_exceeds_unallocated_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let allocation = deposit_amount + 1 as Balance;
    let rate = 1 as TokensPerSecond;
    let block_deposit = 2 as BlockId;
    let block_stream = 3 as BlockId;
    let nonce_deposit = Nonce(1);
    let nonce_stream = Nonce(2);

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
    ) = state_with_initialized_vault(owner_balance_start);

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

    let stream_id = StreamId::MIN;
    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, stream_id);

    let vault_config_before = state
        .get_account_by_id(vault_config_account_id)
        .data
        .clone();
    let vault_holding_balance_before = state.get_account_by_id(vault_holding_account_id).balance;

    let account_ids_stream = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        mock_clock_account_id,
    ];
    let result = state.transition_from_public_transaction(
        &signed_create_stream(
            program_id,
            vault_id,
            stream_id,
            provider_account_id,
            rate,
            allocation,
            &account_ids_stream,
            nonce_stream,
            &owner_private_key,
        ),
        block_stream,
    );
    assert_execution_failed_with_code(result, ERR_ALLOCATION_EXCEEDS_UNALLOCATED);

    assert_eq!(
        state.get_account_by_id(vault_config_account_id).data,
        vault_config_before
    );
    assert_eq!(
        state.get_account_by_id(vault_holding_account_id).balance,
        vault_holding_balance_before
    );
    let stream_account = state.get_account_by_id(stream_pda);
    assert!(
        stream_account.data.is_empty(),
        "stream PDA should not be initialized on failure"
    );
}
#[test]
fn test_create_stream_zero_rate_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let mock_clock_initial_ts = DEFAULT_MOCK_CLOCK_INITIAL_TS;
    let allocation = 1 as Balance;
    let rate = 0 as TokensPerSecond;
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
        owner_balance_start,
        deposit_amount,
        mock_clock_account_id,
        mock_clock_initial_ts,
    );

    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, 0);
    let account_ids_stream = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        mock_clock_account_id,
    ];
    let result = state.transition_from_public_transaction(
        &signed_create_stream(
            program_id,
            vault_id,
            0,
            provider_account_id,
            rate,
            allocation,
            &account_ids_stream,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
    );
    assert_execution_failed_with_code(result, ERR_ZERO_STREAM_RATE);
}

#[test]
fn test_create_stream_zero_allocation_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let mock_clock_initial_ts = DEFAULT_MOCK_CLOCK_INITIAL_TS;
    let allocation = 0 as Balance;
    let rate = 1 as TokensPerSecond;
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
        owner_balance_start,
        deposit_amount,
        mock_clock_account_id,
        mock_clock_initial_ts,
    );

    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, 0);
    let account_ids_stream = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        mock_clock_account_id,
    ];
    let result = state.transition_from_public_transaction(
        &signed_create_stream(
            program_id,
            vault_id,
            0,
            provider_account_id,
            rate,
            allocation,
            &account_ids_stream,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
    );
    assert_execution_failed_with_code(result, ERR_ZERO_STREAM_ALLOCATION);
}

#[test]
fn test_create_stream_stream_id_mismatch_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let mock_clock_initial_ts = DEFAULT_MOCK_CLOCK_INITIAL_TS;
    let allocation = 1 as Balance;
    let rate = 1 as TokensPerSecond;
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
        owner_balance_start,
        deposit_amount,
        mock_clock_account_id,
        mock_clock_initial_ts,
    );

    // `next_stream_id` is 0; instruction asks for stream 1 with matching PDA — guest rejects.
    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, 1);
    let account_ids_stream = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        mock_clock_account_id,
    ];
    let result = state.transition_from_public_transaction(
        &signed_create_stream(
            program_id,
            vault_id,
            1,
            provider_account_id,
            rate,
            allocation,
            &account_ids_stream,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
    );
    assert_execution_failed_with_code(result, ERR_STREAM_ID_MISMATCH);
}

/// List an arbitrary stream account address.
/// Write state to that account even when it differs from `derive_stream_pda(program_id, vault_config, stream_id)`.
/// Update only listed accounts.
#[test]
fn test_create_stream_listed_stream_account_can_differ_from_derived_pda_for_stream_id() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let mock_clock_initial_ts = DEFAULT_MOCK_CLOCK_INITIAL_TS;
    let allocation = 1 as Balance;
    let rate = 1 as TokensPerSecond;
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
        owner_balance_start,
        deposit_amount,
        mock_clock_account_id,
        mock_clock_initial_ts,
    );

    let listed_stream_slot = derive_stream_pda(program_id, vault_config_account_id, 1);
    let canonical_stream_pda = derive_stream_pda(program_id, vault_config_account_id, 0);
    let stream_id = 0 as StreamId;
    let account_ids_stream = [
        vault_config_account_id,
        vault_holding_account_id,
        listed_stream_slot,
        owner_account_id,
        mock_clock_account_id,
    ];
    let result = state.transition_from_public_transaction(
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
    );
    assert!(result.is_ok(), "create_stream failed: {:?}", result);

    let stream_at_listed =
        StreamConfig::from_bytes(&state.get_account_by_id(listed_stream_slot).data)
            .expect("stream state at listed stream account id");
    assert_eq!(stream_at_listed.stream_id, stream_id);
    assert!(
        StreamConfig::from_bytes(&state.get_account_by_id(canonical_stream_pda).data).is_none(),
        "canonical PDA for stream_id should not hold stream data when another id was listed"
    );
}

#[test]
fn test_create_stream_wrong_vault_id_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let mock_clock_initial_ts = DEFAULT_MOCK_CLOCK_INITIAL_TS;
    let allocation = 1 as Balance;
    let rate = 1 as TokensPerSecond;
    let (mock_clock_account_id, provider_account_id) =
        harness_mock_clock_and_provider_account_ids();
    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        _vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_deposited_with_mock_clock(
        owner_balance_start,
        deposit_amount,
        mock_clock_account_id,
        mock_clock_initial_ts,
    );

    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, 0);
    let wrong_vault_id = VaultId::from(999u64);
    let account_ids_stream = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        mock_clock_account_id,
    ];
    let result = state.transition_from_public_transaction(
        &signed_create_stream(
            program_id,
            wrong_vault_id,
            0,
            provider_account_id,
            rate,
            allocation,
            &account_ids_stream,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
    );
    assert_execution_failed_with_code(result, ERR_VAULT_ID_MISMATCH);
}

#[test]
fn test_create_stream_owner_mismatch_fails() {
    let signer_account_balance = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let block_init = 1 as BlockId;
    let block_deposit = 2 as BlockId;
    let block_stream = 3 as BlockId;
    let nonce_init = Nonce(0);
    let nonce_deposit = Nonce(1);
    // Signer is `other`; only `owner` ran init + deposit, so `other`'s nonce is still 0.
    let nonce_stream = Nonce(0);

    let (owner_private_key, owner_account_id) = create_keypair(SEED_OWNER);
    let (alt_signer_private_key, alt_signer_account_id) = create_keypair(SEED_ALT_SIGNER);
    let (mock_clock_account_id, provider_account_id) =
        harness_mock_clock_and_provider_account_ids();

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

    let mock_clock_ts_after_deposit = DEFAULT_MOCK_CLOCK_INITIAL_TS;
    force_mock_timestamp_account(
        &mut state,
        mock_clock_account_id,
        MockTimestamp::new(mock_clock_ts_after_deposit),
    );

    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, 0);
    let vault_config_before = state
        .get_account_by_id(vault_config_account_id)
        .data
        .clone();
    let allocation = 1 as Balance;
    let rate = 1 as TokensPerSecond;

    let account_ids_stream = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        alt_signer_account_id,
        mock_clock_account_id,
    ];
    let result = state.transition_from_public_transaction(
        &signed_create_stream(
            program_id,
            vault_id,
            0,
            provider_account_id,
            rate,
            allocation,
            &account_ids_stream,
            nonce_stream,
            &alt_signer_private_key,
        ),
        block_stream,
    );
    assert_execution_failed_with_code(result, ERR_VAULT_OWNER_MISMATCH);
    assert_eq!(
        state.get_account_by_id(vault_config_account_id).data,
        vault_config_before
    );
}

#[test]
fn test_create_stream_invalid_mock_timestamp_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let mock_clock_initial_ts = DEFAULT_MOCK_CLOCK_INITIAL_TS;
    let allocation = 1 as Balance;
    let rate = 1 as TokensPerSecond;
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
        owner_balance_start,
        deposit_amount,
        mock_clock_account_id,
        mock_clock_initial_ts,
    );

    // Undo mock clock payload so account data is empty / invalid for `MockTimestamp::from_bytes`.
    state.force_insert_account(mock_clock_account_id, Account::default());

    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, 0);
    let account_ids_stream = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        mock_clock_account_id,
    ];
    let result = state.transition_from_public_transaction(
        &signed_create_stream(
            program_id,
            vault_id,
            0,
            provider_account_id,
            rate,
            allocation,
            &account_ids_stream,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
    );
    assert_execution_failed_with_code(result, ERR_INVALID_MOCK_TIMESTAMP);
}

#[test]
fn test_create_stream_allocation_equals_unallocated_succeeds() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let mock_clock_initial_ts = 99 as Timestamp;
    let rate = 1 as TokensPerSecond;
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
        owner_balance_start,
        deposit_amount,
        mock_clock_account_id,
        mock_clock_initial_ts,
    );

    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, 0);
    let allocation = deposit_amount;
    let account_ids_stream = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        mock_clock_account_id,
    ];
    let result = state.transition_from_public_transaction(
        &signed_create_stream(
            program_id,
            vault_id,
            0,
            provider_account_id,
            rate,
            allocation,
            &account_ids_stream,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
    );
    assert!(
        result.is_ok(),
        "create_stream at exact unallocated failed: {:?}",
        result
    );
    let vc = VaultConfig::from_bytes(&state.get_account_by_id(vault_config_account_id).data)
        .expect("vault config");
    assert_eq!(vc.total_allocated, allocation);
}

#[test]
fn test_create_stream_second_stream_succeeds() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let mock_clock_initial_ts = 7 as Timestamp;
    let first_stream_allocation = 200 as Balance;
    let second_stream_allocation = 100 as Balance;
    let expected_total_allocated = first_stream_allocation + second_stream_allocation;
    let first_stream_rate = 2 as TokensPerSecond;
    let second_stream_rate = 3 as TokensPerSecond;
    let (mock_clock_account_id, provider_a) = harness_mock_clock_and_provider_account_ids();
    let (_, provider_b) = create_keypair(SEED_PROVIDER_B);
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
        mock_clock_initial_ts,
    );

    let stream0 = derive_stream_pda(program_id, vault_config_account_id, 0);
    let rate = first_stream_rate;
    let allocation = first_stream_allocation;
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
            rate,
            allocation,
            &accounts0,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "first create_stream failed",
    );

    let stream1 = derive_stream_pda(program_id, vault_config_account_id, 1);
    let rate = second_stream_rate;
    let allocation = second_stream_allocation;
    let accounts1 = [
        vault_config_account_id,
        vault_holding_account_id,
        stream1,
        owner_account_id,
        mock_clock_account_id,
    ];
    let result = state.transition_from_public_transaction(
        &signed_create_stream(
            program_id,
            vault_id,
            1,
            provider_b,
            rate,
            allocation,
            &accounts1,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
    );
    assert!(result.is_ok(), "second create_stream failed: {:?}", result);

    let vc = VaultConfig::from_bytes(&state.get_account_by_id(vault_config_account_id).data)
        .expect("vault config");
    assert_eq!(vc.next_stream_id, 2);
    assert_eq!(vc.total_allocated, expected_total_allocated);

    let s1 = StreamConfig::from_bytes(&state.get_account_by_id(stream1).data).expect("stream 1");
    assert_eq!(s1.stream_id, 1);
    assert_eq!(s1.provider, provider_b);
    assert_eq!(s1.rate, second_stream_rate);
    assert_eq!(s1.allocation, second_stream_allocation);
}

#[test]
fn test_create_stream_next_stream_id_overflow_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let mock_clock_initial_ts = DEFAULT_MOCK_CLOCK_INITIAL_TS;
    let allocation = 1 as Balance;
    let rate = 1 as TokensPerSecond;
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
        owner_balance_start,
        deposit_amount,
        mock_clock_account_id,
        mock_clock_initial_ts,
    );

    let mut vc = VaultConfig::from_bytes(&state.get_account_by_id(vault_config_account_id).data)
        .expect("vault config");
    vc.next_stream_id = u64::MAX;
    vc.total_allocated = 0 as Balance;
    let mut config_account = state.get_account_by_id(vault_config_account_id).clone();
    config_account.data =
        Data::try_from(vc.to_bytes()).expect("vault config payload fits Data limits");
    state.force_insert_account(vault_config_account_id, config_account);

    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, u64::MAX);
    let account_ids_stream = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        mock_clock_account_id,
    ];
    let result = state.transition_from_public_transaction(
        &signed_create_stream(
            program_id,
            vault_id,
            u64::MAX,
            provider_account_id,
            rate,
            allocation,
            &account_ids_stream,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
    );
    assert_execution_failed_with_code(result, ERR_NEXT_STREAM_ID_OVERFLOW);
}
