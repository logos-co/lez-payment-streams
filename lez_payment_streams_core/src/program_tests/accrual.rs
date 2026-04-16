//! Guest-backed accrual tests (`sync_stream` + `StreamConfig::at_time`).

use nssa::program::Program;
use nssa_core::{
    account::{Balance, Nonce},
    program::BlockId,
};

use crate::Instruction;
use crate::{
    test_helpers::{
        build_signed_public_tx, create_keypair, create_state_with_guest_program, derive_stream_pda,
        derive_vault_pdas, force_mock_timestamp_account,
    },
    MockTimestamp, StreamConfig, StreamId, StreamState, Timestamp, TokensPerSecond, VaultId,
    ERR_STREAM_ID_MISMATCH, ERR_TIME_REGRESSION, ERR_VAULT_ID_MISMATCH, ERR_VAULT_OWNER_MISMATCH,
};

use super::common::{
    assert_execution_failed_with_code, state_deposited_with_mock_clock,
    DEFAULT_MOCK_CLOCK_INITIAL_TS, DEFAULT_OWNER_GENESIS_BALANCE, DEFAULT_STREAM_TEST_DEPOSIT,
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

    let (_, mock_clock_account_id) = create_keypair(70);
    let (_, provider_a) = create_keypair(41);
    let (_, provider_b) = create_keypair(42);

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

    let stream0 = derive_stream_pda(program_id, vault_config_account_id, 0);
    let stream1 = derive_stream_pda(program_id, vault_config_account_id, 1);

    let accounts0 = [
        vault_config_account_id,
        vault_holding_account_id,
        stream0,
        owner_account_id,
        mock_clock_account_id,
    ];
    assert!(
        state
            .transition_from_public_transaction(
                &build_signed_public_tx(
                    program_id,
                    Instruction::CreateStream {
                        vault_id,
                        stream_id: 0,
                        provider: provider_a,
                        rate: rate0,
                        allocation: allocation0,
                    },
                    &accounts0,
                    &[Nonce(2)],
                    &[&owner_private_key],
                ),
                3 as BlockId,
            )
            .is_ok(),
        "first create_stream failed"
    );

    let accounts1 = [
        vault_config_account_id,
        vault_holding_account_id,
        stream1,
        owner_account_id,
        mock_clock_account_id,
    ];
    assert!(
        state
            .transition_from_public_transaction(
                &build_signed_public_tx(
                    program_id,
                    Instruction::CreateStream {
                        vault_id,
                        stream_id: 1,
                        provider: provider_b,
                        rate: rate1,
                        allocation: allocation1,
                    },
                    &accounts1,
                    &[Nonce(3)],
                    &[&owner_private_key],
                ),
                4 as BlockId,
            )
            .is_ok(),
        "second create_stream failed"
    );

    force_mock_timestamp_account(&mut state, mock_clock_account_id, MockTimestamp::new(t1));

    assert!(
        state
            .transition_from_public_transaction(
                &build_signed_public_tx(
                    program_id,
                    Instruction::SyncStream {
                        vault_id,
                        stream_id: 0,
                    },
                    &accounts0,
                    &[Nonce(4)],
                    &[&owner_private_key],
                ),
                5 as BlockId,
            )
            .is_ok(),
        "sync_stream stream 0 failed"
    );

    let cfg0_after_first =
        StreamConfig::from_bytes(&state.get_account_by_id(stream0).data).expect("stream 0");
    assert_eq!(cfg0_after_first.accrued, 50 as Balance);
    assert_eq!(cfg0_after_first.accrued_as_of, t1);

    force_mock_timestamp_account(&mut state, mock_clock_account_id, MockTimestamp::new(t2));

    assert!(
        state
            .transition_from_public_transaction(
                &build_signed_public_tx(
                    program_id,
                    Instruction::SyncStream {
                        vault_id,
                        stream_id: 1,
                    },
                    &accounts1,
                    &[Nonce(5)],
                    &[&owner_private_key],
                ),
                6 as BlockId,
            )
            .is_ok(),
        "sync_stream stream 1 failed"
    );

    let cfg0 = StreamConfig::from_bytes(&state.get_account_by_id(stream0).data).expect("stream 0");
    assert_eq!(cfg0.accrued, cfg0_after_first.accrued);
    assert_eq!(cfg0.accrued_as_of, cfg0_after_first.accrued_as_of);

    let cfg1 = StreamConfig::from_bytes(&state.get_account_by_id(stream1).data).expect("stream 1");
    assert_eq!(cfg1.accrued, 50 as Balance);
    assert_eq!(cfg1.accrued_as_of, t2);
}

#[test]
fn test_sync_stream_wrong_vault_id_fails() {
    let (_, mock_clock_account_id) = create_keypair(71);
    let t0: Timestamp = 50;

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        _vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_deposited_with_mock_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        mock_clock_account_id,
        t0,
    );

    let (_, provider_account_id) = create_keypair(45);
    let stream_id = StreamId::MIN;
    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, stream_id);
    let account_ids = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        mock_clock_account_id,
    ];

    assert!(
        state
            .transition_from_public_transaction(
                &build_signed_public_tx(
                    program_id,
                    Instruction::CreateStream {
                        vault_id: _vault_id,
                        stream_id,
                        provider: provider_account_id,
                        rate: 1,
                        allocation: 400,
                    },
                    &account_ids,
                    &[Nonce(2)],
                    &[&owner_private_key],
                ),
                3 as BlockId,
            )
            .is_ok(),
        "create_stream failed"
    );

    let wrong_vault_id = VaultId::from(999u64);
    let r = state.transition_from_public_transaction(
        &build_signed_public_tx(
            program_id,
            Instruction::SyncStream {
                vault_id: wrong_vault_id,
                stream_id,
            },
            &account_ids,
            &[Nonce(3)],
            &[&owner_private_key],
        ),
        4 as BlockId,
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
    // Signer is `other`; only `owner` ran init + deposit + create_stream.
    let nonce_sync = Nonce(0);

    let (owner_private_key, owner_account_id) = create_keypair(1);
    let (other_private_key, other_account_id) = create_keypair(2);
    let (_, mock_clock_account_id) = create_keypair(77);
    let (_, provider_account_id) = create_keypair(42);

    let initial_accounts_data = vec![
        (owner_account_id, signer_account_balance),
        (other_account_id, signer_account_balance),
    ];
    let (mut state, guest_program) = create_state_with_guest_program(&initial_accounts_data)
        .expect(
            "guest image present (cargo build -p lez_payment_streams-methods) and state genesis ok",
        );
    let program_id = guest_program.id();

    let vault_id = VaultId::from(1u64);
    let (vault_config_account_id, vault_holding_account_id) =
        derive_vault_pdas(program_id, owner_account_id, vault_id);
    assert!(
        state
            .transition_from_public_transaction(
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
            )
            .is_ok(),
        "initialize_vault failed"
    );

    assert!(
        state
            .transition_from_public_transaction(
                &build_signed_public_tx(
                    program_id,
                    Instruction::Deposit {
                        vault_id,
                        amount: deposit_amount,
                        authenticated_transfer_program_id: Program::authenticated_transfer_program(
                        )
                        .id(),
                    },
                    &[
                        vault_config_account_id,
                        vault_holding_account_id,
                        owner_account_id,
                    ],
                    &[nonce_deposit],
                    &[&owner_private_key],
                ),
                block_deposit,
            )
            .is_ok(),
        "deposit failed"
    );

    force_mock_timestamp_account(
        &mut state,
        mock_clock_account_id,
        MockTimestamp::new(DEFAULT_MOCK_CLOCK_INITIAL_TS),
    );

    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, 0);
    let allocation = 100 as Balance;
    let rate = 1 as TokensPerSecond;

    assert!(
        state
            .transition_from_public_transaction(
                &build_signed_public_tx(
                    program_id,
                    Instruction::CreateStream {
                        vault_id,
                        stream_id: 0,
                        provider: provider_account_id,
                        rate,
                        allocation,
                    },
                    &[
                        vault_config_account_id,
                        vault_holding_account_id,
                        stream_pda,
                        owner_account_id,
                        mock_clock_account_id,
                    ],
                    &[nonce_stream],
                    &[&owner_private_key],
                ),
                block_stream,
            )
            .is_ok(),
        "create_stream failed"
    );

    let account_ids_sync = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        other_account_id,
        mock_clock_account_id,
    ];
    let r = state.transition_from_public_transaction(
        &build_signed_public_tx(
            program_id,
            Instruction::SyncStream {
                vault_id,
                stream_id: 0,
            },
            &account_ids_sync,
            &[nonce_sync],
            &[&other_private_key],
        ),
        block_sync,
    );
    assert_execution_failed_with_code(r, ERR_VAULT_OWNER_MISMATCH);
}

#[test]
fn test_sync_stream_stream_id_does_not_match_account_fails() {
    let (_, mock_clock_account_id) = create_keypair(72);
    let t0: Timestamp = 20;

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

    let (_, provider_account_id) = create_keypair(46);
    let stream0 = derive_stream_pda(program_id, vault_config_account_id, 0);
    let account_ids = [
        vault_config_account_id,
        vault_holding_account_id,
        stream0,
        owner_account_id,
        mock_clock_account_id,
    ];

    assert!(
        state
            .transition_from_public_transaction(
                &build_signed_public_tx(
                    program_id,
                    Instruction::CreateStream {
                        vault_id,
                        stream_id: 0,
                        provider: provider_account_id,
                        rate: 2,
                        allocation: 300,
                    },
                    &account_ids,
                    &[Nonce(2)],
                    &[&owner_private_key],
                ),
                3 as BlockId,
            )
            .is_ok(),
        "create_stream failed"
    );

    let r = state.transition_from_public_transaction(
        &build_signed_public_tx(
            program_id,
            Instruction::SyncStream {
                vault_id,
                stream_id: 1,
            },
            &account_ids,
            &[Nonce(3)],
            &[&owner_private_key],
        ),
        4 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_STREAM_ID_MISMATCH);
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
