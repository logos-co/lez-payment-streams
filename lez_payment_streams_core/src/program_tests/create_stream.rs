//! `create_stream` integration tests (happy paths, bounds, account layout).

use nssa_core::{
    account::{Account, Balance, Data, Nonce},
    BlockId,
};

use crate::Instruction;
use crate::{
    test_helpers::{
        build_signed_public_tx, create_keypair, create_state_with_guest_program, derive_stream_pda,
        derive_vault_pdas, force_clock_account_monotonic,
        harness_clock_01_and_provider_account_ids, patch_vault_config,
        state_with_initialized_vault,
    },
    StreamConfig, StreamId, StreamState, Timestamp, TokensPerSecond, VaultConfig, VaultId,
    DEFAULT_VERSION, ERR_ALLOCATION_EXCEEDS_UNALLOCATED, ERR_INVALID_CLOCK_ACCOUNT,
    ERR_NEXT_STREAM_ID_OVERFLOW, ERR_STREAM_ID_MISMATCH, ERR_VAULT_ID_MISMATCH,
    ERR_VAULT_OWNER_MISMATCH, ERR_ZERO_STREAM_ALLOCATION, ERR_ZERO_STREAM_RATE,
};

use super::common::{
    assert_execution_failed_with_code, signed_create_stream, signed_deposit,
    state_deposited_with_clock, transition_ok, DEFAULT_CLOCK_INITIAL_TS,
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
    // PDA seed order must match the guest `#[account(..., pda = [...])]` attributes in
    // `methods/guest/src/bin/lez_payment_streams.rs` (`initialize_vault` / `create_stream`):
    // vault_config: `vault_config`, owner, vault_id;
    // vault_holding: `vault_holding`, vault_config account id, `native`;
    // stream_config: `stream_config`, vault_config account id, stream_id.
    let (vault_config_account_id, vault_holding_account_id) =
        derive_vault_pdas(program_id, owner_account_id, vault_id);
    assert_ne!(
        vault_config_account_id, vault_holding_account_id,
        "vault config and holding PDAs must differ"
    );
    let s0 = derive_stream_pda(program_id, vault_config_account_id, 0);
    let s0_b = derive_stream_pda(program_id, vault_config_account_id, 0);
    assert_eq!(s0, s0_b);
    assert_ne!(
        s0,
        derive_stream_pda(program_id, vault_config_account_id, 1)
    );
}

#[test]
fn test_create_stream_succeeds() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let allocation = 200 as Balance;
    let rate = 10 as TokensPerSecond;
    let block_deposit = 2 as BlockId;
    let block_stream = 3 as BlockId;
    let nonce_deposit = Nonce(1);
    let nonce_stream = Nonce(2);

    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let mut v = state_with_initialized_vault(owner_balance_start);

    transition_ok(
        &mut v.state,
        &signed_deposit(
            v.program_id,
            v.vault_id,
            amount,
            v.vault_config_account_id,
            v.vault_holding_account_id,
            v.owner_account_id,
            nonce_deposit,
            &v.owner_private_key,
        ),
        block_deposit,
        "deposit failed",
    );

    let initial_ts: Timestamp = 12_345;
    force_clock_account_monotonic(&mut v.state, clock_id, 0, initial_ts);

    let stream_id = StreamId::MIN;
    let stream_pda = derive_stream_pda(v.program_id, v.vault_config_account_id, stream_id);

    let account_ids_stream = [
        v.vault_config_account_id,
        v.vault_holding_account_id,
        stream_pda,
        v.owner_account_id,
        clock_id,
    ];
    let tx_stream = signed_create_stream(
        v.program_id,
        v.vault_id,
        stream_id,
        provider_account_id,
        rate,
        allocation,
        &account_ids_stream,
        nonce_stream,
        &v.owner_private_key,
    );
    let result_stream = v.state.transition_from_public_transaction(
        &tx_stream,
        block_stream,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert!(
        result_stream.is_ok(),
        "create_stream tx failed: {:?}",
        result_stream
    );

    let vault_config =
        VaultConfig::from_bytes(&v.state.get_account_by_id(v.vault_config_account_id).data)
            .expect("vault config");
    assert_eq!(vault_config.next_stream_id, 1);
    assert_eq!(vault_config.total_allocated, allocation);

    let stream_data = v.state.get_account_by_id(stream_pda).data.clone();
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
        v.state
            .get_account_by_id(v.vault_holding_account_id)
            .balance,
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

    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let mut v = state_with_initialized_vault(owner_balance_start);

    transition_ok(
        &mut v.state,
        &signed_deposit(
            v.program_id,
            v.vault_id,
            deposit_amount,
            v.vault_config_account_id,
            v.vault_holding_account_id,
            v.owner_account_id,
            nonce_deposit,
            &v.owner_private_key,
        ),
        block_deposit,
        "deposit failed",
    );

    let stream_id = StreamId::MIN;
    let stream_pda = derive_stream_pda(v.program_id, v.vault_config_account_id, stream_id);

    let vault_config_before = v
        .state
        .get_account_by_id(v.vault_config_account_id)
        .data
        .clone();
    let vault_holding_balance_before = v
        .state
        .get_account_by_id(v.vault_holding_account_id)
        .balance;

    let account_ids_stream = [
        v.vault_config_account_id,
        v.vault_holding_account_id,
        stream_pda,
        v.owner_account_id,
        clock_id,
    ];
    let result = v.state.transition_from_public_transaction(
        &signed_create_stream(
            v.program_id,
            v.vault_id,
            stream_id,
            provider_account_id,
            rate,
            allocation,
            &account_ids_stream,
            nonce_stream,
            &v.owner_private_key,
        ),
        block_stream,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(result, ERR_ALLOCATION_EXCEEDS_UNALLOCATED);

    assert_eq!(
        v.state.get_account_by_id(v.vault_config_account_id).data,
        vault_config_before
    );
    assert_eq!(
        v.state
            .get_account_by_id(v.vault_holding_account_id)
            .balance,
        vault_holding_balance_before
    );
    let stream_account = v.state.get_account_by_id(stream_pda);
    assert!(
        stream_account.data.is_empty(),
        "stream PDA should not be initialized on failure"
    );
}
#[test]
fn test_create_stream_zero_rate_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let clock_initial_ts = DEFAULT_CLOCK_INITIAL_TS;
    let allocation = 1 as Balance;
    let rate = 0 as TokensPerSecond;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();
    let mut dep = state_deposited_with_clock(
        owner_balance_start,
        deposit_amount,
        clock_id,
        clock_initial_ts,
    );

    let stream_pda = derive_stream_pda(dep.vault.program_id, dep.vault.vault_config_account_id, 0);
    let account_ids_stream = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        clock_id,
    ];
    let result = dep.vault.state.transition_from_public_transaction(
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            0,
            provider_account_id,
            rate,
            allocation,
            &account_ids_stream,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(result, ERR_ZERO_STREAM_RATE);
}

#[test]
fn test_create_stream_zero_allocation_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let clock_initial_ts = DEFAULT_CLOCK_INITIAL_TS;
    let allocation = 0 as Balance;
    let rate = 1 as TokensPerSecond;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();
    let mut dep = state_deposited_with_clock(
        owner_balance_start,
        deposit_amount,
        clock_id,
        clock_initial_ts,
    );

    let stream_pda = derive_stream_pda(dep.vault.program_id, dep.vault.vault_config_account_id, 0);
    let account_ids_stream = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        clock_id,
    ];
    let result = dep.vault.state.transition_from_public_transaction(
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            0,
            provider_account_id,
            rate,
            allocation,
            &account_ids_stream,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(result, ERR_ZERO_STREAM_ALLOCATION);
}

#[test]
fn test_create_stream_stream_id_mismatch_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let clock_initial_ts = DEFAULT_CLOCK_INITIAL_TS;
    let allocation = 1 as Balance;
    let rate = 1 as TokensPerSecond;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();
    let mut dep = state_deposited_with_clock(
        owner_balance_start,
        deposit_amount,
        clock_id,
        clock_initial_ts,
    );

    // `next_stream_id` is 0; instruction asks for stream 1 with matching PDA — guest rejects.
    let stream_pda = derive_stream_pda(dep.vault.program_id, dep.vault.vault_config_account_id, 1);
    let account_ids_stream = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        clock_id,
    ];
    let result = dep.vault.state.transition_from_public_transaction(
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            1,
            provider_account_id,
            rate,
            allocation,
            &account_ids_stream,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(result, ERR_STREAM_ID_MISMATCH);
}

/// `stream_config` account must match the PDA for `stream_id` (SPEL account validation).
#[test]
fn test_create_stream_mismatched_stream_pda_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let clock_initial_ts = DEFAULT_CLOCK_INITIAL_TS;
    let allocation = 1 as Balance;
    let rate = 1 as TokensPerSecond;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();
    let mut dep = state_deposited_with_clock(
        owner_balance_start,
        deposit_amount,
        clock_id,
        clock_initial_ts,
    );

    let wrong_stream_pda =
        derive_stream_pda(dep.vault.program_id, dep.vault.vault_config_account_id, 1);
    let stream_id = 0 as StreamId;
    let account_ids_stream = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        wrong_stream_pda,
        dep.vault.owner_account_id,
        clock_id,
    ];
    let result = dep.vault.state.transition_from_public_transaction(
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            rate,
            allocation,
            &account_ids_stream,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert!(
        result.is_err(),
        "expected PdaMismatch failure, got {:?}",
        result
    );
}

#[test]
fn test_create_stream_wrong_vault_id_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let clock_initial_ts = DEFAULT_CLOCK_INITIAL_TS;
    let allocation = 1 as Balance;
    let rate = 1 as TokensPerSecond;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();
    let mut dep = state_deposited_with_clock(
        owner_balance_start,
        deposit_amount,
        clock_id,
        clock_initial_ts,
    );

    let stream_pda = derive_stream_pda(dep.vault.program_id, dep.vault.vault_config_account_id, 0);
    patch_vault_config(
        &mut dep.vault.state,
        dep.vault.vault_config_account_id,
        |vc| {
            vc.vault_id = VaultId::from(999u64);
        },
    );
    let account_ids_stream = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        clock_id,
    ];
    let result = dep.vault.state.transition_from_public_transaction(
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            0,
            provider_account_id,
            rate,
            allocation,
            &account_ids_stream,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
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
    let nonce_stream = Nonce(2);

    let (owner_private_key, owner_account_id) = create_keypair(SEED_OWNER);
    let (_, alt_signer_account_id) = create_keypair(SEED_ALT_SIGNER);
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

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
            Instruction::initialize_vault_public(vault_id),
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

    let clock_ts_after_deposit = DEFAULT_CLOCK_INITIAL_TS;
    force_clock_account_monotonic(&mut state, clock_id, 0, clock_ts_after_deposit);

    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, 0);
    let allocation = 1 as Balance;
    let rate = 1 as TokensPerSecond;

    patch_vault_config(&mut state, vault_config_account_id, |vc| {
        vc.owner = alt_signer_account_id;
    });
    let vault_config_before = state
        .get_account_by_id(vault_config_account_id)
        .data
        .clone();

    let account_ids_stream = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        clock_id,
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
            &owner_private_key,
        ),
        block_stream,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(result, ERR_VAULT_OWNER_MISMATCH);
    assert_eq!(
        state.get_account_by_id(vault_config_account_id).data,
        vault_config_before
    );
}

#[test]
fn test_create_stream_invalid_clock_account_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let clock_initial_ts = DEFAULT_CLOCK_INITIAL_TS;
    let allocation = 1 as Balance;
    let rate = 1 as TokensPerSecond;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();
    let mut dep = state_deposited_with_clock(
        owner_balance_start,
        deposit_amount,
        clock_id,
        clock_initial_ts,
    );

    let stream_pda = derive_stream_pda(dep.vault.program_id, dep.vault.vault_config_account_id, 0);
    let (_dk, decoy_id) = create_keypair(201);
    dep.vault.state.force_insert_account(
        decoy_id,
        Account {
            balance: 0,
            program_owner: dep.vault.program_id,
            data: Data::default(),
            ..Account::default()
        },
    );
    // Last slot must be a system clock account id; arbitrary accounts are rejected.
    let account_ids_stream = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        decoy_id,
    ];
    let result = dep.vault.state.transition_from_public_transaction(
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            0,
            provider_account_id,
            rate,
            allocation,
            &account_ids_stream,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        super::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(result, ERR_INVALID_CLOCK_ACCOUNT);
}

#[test]
fn test_create_stream_allocation_equals_unallocated_succeeds() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let clock_initial_ts = 99 as Timestamp;
    let rate = 1 as TokensPerSecond;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();
    let mut dep = state_deposited_with_clock(
        owner_balance_start,
        deposit_amount,
        clock_id,
        clock_initial_ts,
    );

    let stream_pda = derive_stream_pda(dep.vault.program_id, dep.vault.vault_config_account_id, 0);
    let allocation = deposit_amount;
    let account_ids_stream = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        clock_id,
    ];
    let result = dep.vault.state.transition_from_public_transaction(
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            0,
            provider_account_id,
            rate,
            allocation,
            &account_ids_stream,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert!(
        result.is_ok(),
        "create_stream at exact unallocated failed: {:?}",
        result
    );
    let vc = VaultConfig::from_bytes(
        &dep.vault
            .state
            .get_account_by_id(dep.vault.vault_config_account_id)
            .data,
    )
    .expect("vault config");
    assert_eq!(vc.total_allocated, allocation);
}

#[test]
fn test_create_stream_second_stream_succeeds() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let clock_initial_ts = 7 as Timestamp;
    let first_stream_allocation = 200 as Balance;
    let second_stream_allocation = 100 as Balance;
    let expected_total_allocated = first_stream_allocation + second_stream_allocation;
    let first_stream_rate = 2 as TokensPerSecond;
    let second_stream_rate = 3 as TokensPerSecond;
    let (clock_id, provider_a) = harness_clock_01_and_provider_account_ids();
    let (_, provider_b) = create_keypair(SEED_PROVIDER_B);
    let mut dep = state_deposited_with_clock(
        owner_balance_start,
        deposit_amount,
        clock_id,
        clock_initial_ts,
    );

    let stream0 = derive_stream_pda(dep.vault.program_id, dep.vault.vault_config_account_id, 0);
    let rate = first_stream_rate;
    let allocation = first_stream_allocation;
    let accounts0 = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream0,
        dep.vault.owner_account_id,
        clock_id,
    ];
    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            0,
            provider_a,
            rate,
            allocation,
            &accounts0,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "first create_stream failed",
    );

    let stream1 = derive_stream_pda(dep.vault.program_id, dep.vault.vault_config_account_id, 1);
    let rate = second_stream_rate;
    let allocation = second_stream_allocation;
    let accounts1 = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream1,
        dep.vault.owner_account_id,
        clock_id,
    ];
    let result = dep.vault.state.transition_from_public_transaction(
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            1,
            provider_b,
            rate,
            allocation,
            &accounts1,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert!(result.is_ok(), "second create_stream failed: {:?}", result);

    let vc = VaultConfig::from_bytes(
        &dep.vault
            .state
            .get_account_by_id(dep.vault.vault_config_account_id)
            .data,
    )
    .expect("vault config");
    assert_eq!(vc.next_stream_id, 2);
    assert_eq!(vc.total_allocated, expected_total_allocated);

    let s1 = StreamConfig::from_bytes(&dep.vault.state.get_account_by_id(stream1).data)
        .expect("stream 1");
    assert_eq!(s1.stream_id, 1);
    assert_eq!(s1.provider, provider_b);
    assert_eq!(s1.rate, second_stream_rate);
    assert_eq!(s1.allocation, second_stream_allocation);
}

#[test]
fn test_create_stream_next_stream_id_overflow_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let clock_initial_ts = DEFAULT_CLOCK_INITIAL_TS;
    let allocation = 1 as Balance;
    let rate = 1 as TokensPerSecond;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();
    let mut dep = state_deposited_with_clock(
        owner_balance_start,
        deposit_amount,
        clock_id,
        clock_initial_ts,
    );

    let mut vc = VaultConfig::from_bytes(
        &dep.vault
            .state
            .get_account_by_id(dep.vault.vault_config_account_id)
            .data,
    )
    .expect("vault config");
    vc.next_stream_id = u64::MAX;
    vc.total_allocated = 0 as Balance;
    let mut config_account = dep
        .vault
        .state
        .get_account_by_id(dep.vault.vault_config_account_id)
        .clone();
    config_account.data =
        Data::try_from(vc.to_bytes()).expect("vault config payload fits Data limits");
    dep.vault
        .state
        .force_insert_account(dep.vault.vault_config_account_id, config_account);

    let stream_pda = derive_stream_pda(
        dep.vault.program_id,
        dep.vault.vault_config_account_id,
        u64::MAX,
    );
    let account_ids_stream = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        clock_id,
    ];
    let result = dep.vault.state.transition_from_public_transaction(
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            u64::MAX,
            provider_account_id,
            rate,
            allocation,
            &account_ids_stream,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(result, ERR_NEXT_STREAM_ID_OVERFLOW);
}
