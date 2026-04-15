//! Guest-backed accrual tests (`sync_stream` + `StreamConfig::at_time`).

use nssa_core::{
    account::{Balance, Nonce},
    program::BlockId,
};

use crate::Instruction;
use crate::{
    test_helpers::{
        build_signed_public_tx, create_keypair, derive_stream_pda, force_mock_timestamp_account,
    },
    MockTimestamp, StreamConfig, StreamId, StreamState, Timestamp, TokensPerSecond,
    ERR_TIME_REGRESSION,
};

use super::common::{
    assert_execution_failed_with_code, state_deposited_with_mock_clock,
    DEFAULT_OWNER_GENESIS_BALANCE, DEFAULT_STREAM_TEST_DEPOSIT,
};

#[test]
fn test_accrual_basic() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let allocation = 200 as Balance;
    let rate = 10 as TokensPerSecond;
    let t0: Timestamp = 12_345;
    let t1: Timestamp = t0 + 5;

    let (_, mock_clock_account_id) = create_keypair(77);
    let (_, provider_account_id) = create_keypair(42);

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

    let account_ids_stream = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        mock_clock_account_id,
    ];
    let tx_create = build_signed_public_tx(
        program_id,
        Instruction::CreateStream {
            vault_id,
            stream_id,
            provider: provider_account_id,
            rate,
            allocation,
        },
        &account_ids_stream,
        &[Nonce(2)],
        &[&owner_private_key],
    );
    assert!(
        state
            .transition_from_public_transaction(&tx_create, 3 as BlockId)
            .is_ok(),
        "create_stream failed"
    );

    force_mock_timestamp_account(&mut state, mock_clock_account_id, MockTimestamp::new(t1));

    let tx_sync = build_signed_public_tx(
        program_id,
        Instruction::SyncStream {
            vault_id,
            stream_id,
        },
        &account_ids_stream,
        &[Nonce(3)],
        &[&owner_private_key],
    );
    assert!(
        state
            .transition_from_public_transaction(&tx_sync, 4 as BlockId)
            .is_ok(),
        "sync_stream failed"
    );

    let cfg =
        StreamConfig::from_bytes(&state.get_account_by_id(stream_pda).data).expect("stream config");
    assert_eq!(cfg.accrued, 50 as Balance);
    assert_eq!(cfg.accrued_as_of, t1);
    assert_eq!(cfg.state, StreamState::Active);
}

#[test]
fn test_accrual_caps_at_allocation() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let allocation = 100 as Balance;
    let rate = 10 as TokensPerSecond;
    let t0: Timestamp = 0;
    let t1: Timestamp = 100;

    let (_, mock_clock_account_id) = create_keypair(88);
    let (_, provider_account_id) = create_keypair(43);

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
    let account_ids = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        mock_clock_account_id,
    ];

    let tx_create = build_signed_public_tx(
        program_id,
        Instruction::CreateStream {
            vault_id,
            stream_id,
            provider: provider_account_id,
            rate,
            allocation,
        },
        &account_ids,
        &[Nonce(2)],
        &[&owner_private_key],
    );
    assert!(state
        .transition_from_public_transaction(&tx_create, 3 as BlockId)
        .is_ok());

    force_mock_timestamp_account(&mut state, mock_clock_account_id, MockTimestamp::new(t1));

    let tx_sync = build_signed_public_tx(
        program_id,
        Instruction::SyncStream {
            vault_id,
            stream_id,
        },
        &account_ids,
        &[Nonce(3)],
        &[&owner_private_key],
    );
    assert!(state
        .transition_from_public_transaction(&tx_sync, 4 as BlockId)
        .is_ok());

    let cfg = StreamConfig::from_bytes(&state.get_account_by_id(stream_pda).data).expect("parse");
    let expected_depletion_instant: Timestamp = (allocation / u128::from(rate)) as Timestamp;
    assert_eq!(cfg.accrued, allocation);
    assert_eq!(cfg.state, StreamState::Paused);
    assert_eq!(cfg.accrued_as_of, expected_depletion_instant);
}

#[test]
fn test_sync_stream_time_regression_fails() {
    let (_, mock_clock_account_id) = create_keypair(99);
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
    ) = state_deposited_with_mock_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        mock_clock_account_id,
        t0,
    );

    let (_, provider_account_id) = create_keypair(44);
    let stream_id = StreamId::MIN;
    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, stream_id);
    let account_ids = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        mock_clock_account_id,
    ];

    let tx_create = build_signed_public_tx(
        program_id,
        Instruction::CreateStream {
            vault_id,
            stream_id,
            provider: provider_account_id,
            rate: 1,
            allocation: 400,
        },
        &account_ids,
        &[Nonce(2)],
        &[&owner_private_key],
    );
    assert!(state
        .transition_from_public_transaction(&tx_create, 3 as BlockId)
        .is_ok());

    force_mock_timestamp_account(&mut state, mock_clock_account_id, MockTimestamp::new(t_bad));

    let tx_sync = build_signed_public_tx(
        program_id,
        Instruction::SyncStream {
            vault_id,
            stream_id,
        },
        &account_ids,
        &[Nonce(3)],
        &[&owner_private_key],
    );
    let r = state.transition_from_public_transaction(&tx_sync, 4 as BlockId);
    assert_execution_failed_with_code(r, ERR_TIME_REGRESSION);
}
