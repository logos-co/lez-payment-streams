//! Guest-backed pause and resume tests.

use nssa::{program::Program, PrivateKey, ProgramId, PublicTransaction, V03State};
use nssa_core::{
    account::{AccountId, Balance, Data, Nonce},
    program::BlockId,
};

use crate::Instruction;
use crate::{
    test_helpers::{
        build_signed_public_tx, create_keypair, create_state_with_guest_program, derive_stream_pda,
        derive_vault_pdas, force_mock_timestamp_account,
    },
    MockTimestamp, StreamConfig, StreamId, StreamState, Timestamp, TokensPerSecond, VaultId,
    ERR_RESUME_ZERO_REMAINING_ALLOCATION, ERR_STREAM_NOT_ACTIVE, ERR_STREAM_NOT_PAUSED,
    ERR_TIME_REGRESSION, ERR_VAULT_ID_MISMATCH, ERR_VAULT_OWNER_MISMATCH,
};

use super::common::{
    assert_execution_failed_with_code, state_deposited_with_mock_clock,
    DEFAULT_MOCK_CLOCK_INITIAL_TS, DEFAULT_OWNER_GENESIS_BALANCE, DEFAULT_STREAM_TEST_DEPOSIT,
};

/// Account order for stream instructions: vault config, holding, stream PDA, owner, mock clock.
type StreamIxAccounts = [AccountId; 5];

fn first_stream_accounts(
    program_id: ProgramId,
    vault_config_account_id: AccountId,
    vault_holding_account_id: AccountId,
    owner_account_id: AccountId,
    mock_clock_account_id: AccountId,
) -> (StreamId, AccountId, StreamIxAccounts) {
    let stream_id = StreamId::MIN;
    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, stream_id);
    let account_ids = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        mock_clock_account_id,
    ];
    (stream_id, stream_pda, account_ids)
}

fn transition_ok(
    state: &mut V03State,
    tx: &PublicTransaction,
    block: BlockId,
    label: &'static str,
) {
    assert!(
        state.transition_from_public_transaction(tx, block).is_ok(),
        "{label}",
    );
}

fn signed_create_stream(
    program_id: ProgramId,
    vault_id: VaultId,
    stream_id: StreamId,
    provider: AccountId,
    rate: TokensPerSecond,
    allocation: Balance,
    accounts: &StreamIxAccounts,
    nonce: Nonce,
    owner: &PrivateKey,
) -> PublicTransaction {
    build_signed_public_tx(
        program_id,
        Instruction::CreateStream {
            vault_id,
            stream_id,
            provider,
            rate,
            allocation,
        },
        accounts,
        &[nonce],
        &[owner],
    )
}

fn signed_pause_stream(
    program_id: ProgramId,
    vault_id: VaultId,
    stream_id: StreamId,
    accounts: &StreamIxAccounts,
    nonce: Nonce,
    owner: &PrivateKey,
) -> PublicTransaction {
    build_signed_public_tx(
        program_id,
        Instruction::PauseStream {
            vault_id,
            stream_id,
        },
        accounts,
        &[nonce],
        &[owner],
    )
}

fn signed_resume_stream(
    program_id: ProgramId,
    vault_id: VaultId,
    stream_id: StreamId,
    accounts: &StreamIxAccounts,
    nonce: Nonce,
    owner: &PrivateKey,
) -> PublicTransaction {
    build_signed_public_tx(
        program_id,
        Instruction::ResumeStream {
            vault_id,
            stream_id,
        },
        accounts,
        &[nonce],
        &[owner],
    )
}

fn signed_sync_stream(
    program_id: ProgramId,
    vault_id: VaultId,
    stream_id: StreamId,
    accounts: &StreamIxAccounts,
    nonce: Nonce,
    owner: &PrivateKey,
) -> PublicTransaction {
    build_signed_public_tx(
        program_id,
        Instruction::SyncStream {
            vault_id,
            stream_id,
        },
        accounts,
        &[nonce],
        &[owner],
    )
}

fn force_stream_state_closed(state: &mut V03State, stream_pda: AccountId) {
    let mut cfg =
        StreamConfig::from_bytes(&state.get_account_by_id(stream_pda).data).expect("stream");
    cfg.state = StreamState::Closed;
    let mut stream_account = state.get_account_by_id(stream_pda).clone();
    stream_account.data = Data::try_from(cfg.to_bytes()).expect("stream payload fits");
    state.force_insert_account(stream_pda, stream_account);
}

#[test]
fn test_pause() {
    let t0: Timestamp = 12_345;
    let (_, mock_clock_account_id) = create_keypair(81);
    let (_, provider_account_id) = create_keypair(82);

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

    let (stream_id, stream_pda, account_ids) = first_stream_accounts(
        program_id,
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        mock_clock_account_id,
    );

    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            stream_id,
            provider_account_id,
            10 as TokensPerSecond,
            200 as Balance,
            &account_ids,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    transition_ok(
        &mut state,
        &signed_pause_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
        "pause_stream failed",
    );

    let cfg = StreamConfig::from_bytes(&state.get_account_by_id(stream_pda).data).expect("stream");
    assert_eq!(cfg.state, StreamState::Paused);
    assert_eq!(cfg.accrued, 0 as Balance);
    assert_eq!(cfg.accrued_as_of, t0);
}

#[test]
fn test_resume() {
    let t0: Timestamp = 100;
    let t1: Timestamp = 200;
    let (_, mock_clock_account_id) = create_keypair(83);
    let (_, provider_account_id) = create_keypair(84);

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

    let (stream_id, stream_pda, account_ids) = first_stream_accounts(
        program_id,
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        mock_clock_account_id,
    );

    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            stream_id,
            provider_account_id,
            5 as TokensPerSecond,
            400 as Balance,
            &account_ids,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    transition_ok(
        &mut state,
        &signed_pause_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
        "pause_stream failed",
    );

    force_mock_timestamp_account(&mut state, mock_clock_account_id, MockTimestamp::new(t1));

    transition_ok(
        &mut state,
        &signed_resume_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(4),
            &owner_private_key,
        ),
        5 as BlockId,
        "resume_stream failed",
    );

    let cfg = StreamConfig::from_bytes(&state.get_account_by_id(stream_pda).data).expect("stream");
    assert_eq!(cfg.state, StreamState::Active);
    assert_eq!(cfg.accrued, 0 as Balance);
    assert_eq!(cfg.accrued_as_of, t1);
}

#[test]
fn test_pause_twice_fails() {
    let t0: Timestamp = 1;
    let (_, mock_clock_account_id) = create_keypair(85);
    let (_, provider_account_id) = create_keypair(86);

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

    let (stream_id, _, account_ids) = first_stream_accounts(
        program_id,
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        mock_clock_account_id,
    );

    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            stream_id,
            provider_account_id,
            1 as TokensPerSecond,
            100 as Balance,
            &account_ids,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    transition_ok(
        &mut state,
        &signed_pause_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
        "first pause_stream failed",
    );

    let r = state.transition_from_public_transaction(
        &signed_pause_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(4),
            &owner_private_key,
        ),
        5 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_STREAM_NOT_ACTIVE);
}

#[test]
fn test_pause_when_at_time_depletes_fails() {
    let t0: Timestamp = 0;
    let t_deplete: Timestamp = 100;
    let (_, mock_clock_account_id) = create_keypair(87);
    let (_, provider_account_id) = create_keypair(88);

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

    let (stream_id, _, account_ids) = first_stream_accounts(
        program_id,
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        mock_clock_account_id,
    );

    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            stream_id,
            provider_account_id,
            10 as TokensPerSecond,
            100 as Balance,
            &account_ids,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    force_mock_timestamp_account(
        &mut state,
        mock_clock_account_id,
        MockTimestamp::new(t_deplete),
    );

    let r = state.transition_from_public_transaction(
        &signed_pause_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_STREAM_NOT_ACTIVE);
}

#[test]
fn test_resume_active_fails() {
    let t0: Timestamp = 50;
    let (_, mock_clock_account_id) = create_keypair(89);
    let (_, provider_account_id) = create_keypair(90);

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

    let (stream_id, _, account_ids) = first_stream_accounts(
        program_id,
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        mock_clock_account_id,
    );

    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            stream_id,
            provider_account_id,
            1 as TokensPerSecond,
            400 as Balance,
            &account_ids,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    let r = state.transition_from_public_transaction(
        &signed_resume_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_STREAM_NOT_PAUSED);
}

#[test]
fn test_resume_zero_remaining_fails() {
    let t0: Timestamp = 0;
    let t1: Timestamp = 100;
    let (_, mock_clock_account_id) = create_keypair(91);
    let (_, provider_account_id) = create_keypair(92);

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

    let (stream_id, _, account_ids) = first_stream_accounts(
        program_id,
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        mock_clock_account_id,
    );

    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            stream_id,
            provider_account_id,
            10 as TokensPerSecond,
            100 as Balance,
            &account_ids,
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
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
        "sync_stream failed",
    );

    let r = state.transition_from_public_transaction(
        &signed_resume_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(4),
            &owner_private_key,
        ),
        5 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_RESUME_ZERO_REMAINING_ALLOCATION);
}

#[test]
fn test_resume_twice_fails() {
    let t0: Timestamp = 10;
    let t1: Timestamp = 20;
    let (_, mock_clock_account_id) = create_keypair(93);
    let (_, provider_account_id) = create_keypair(94);

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

    let (stream_id, _, account_ids) = first_stream_accounts(
        program_id,
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        mock_clock_account_id,
    );

    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            stream_id,
            provider_account_id,
            1 as TokensPerSecond,
            500 as Balance,
            &account_ids,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    transition_ok(
        &mut state,
        &signed_pause_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
        "pause_stream failed",
    );

    force_mock_timestamp_account(&mut state, mock_clock_account_id, MockTimestamp::new(t1));

    transition_ok(
        &mut state,
        &signed_resume_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(4),
            &owner_private_key,
        ),
        5 as BlockId,
        "resume_stream failed",
    );

    let r = state.transition_from_public_transaction(
        &signed_resume_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(5),
            &owner_private_key,
        ),
        6 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_STREAM_NOT_PAUSED);
}

#[test]
fn test_pause_closed_fails() {
    let t0: Timestamp = 7;
    let (_, mock_clock_account_id) = create_keypair(95);
    let (_, provider_account_id) = create_keypair(96);

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

    let (stream_id, stream_pda, account_ids) = first_stream_accounts(
        program_id,
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        mock_clock_account_id,
    );

    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            stream_id,
            provider_account_id,
            2 as TokensPerSecond,
            300 as Balance,
            &account_ids,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    force_stream_state_closed(&mut state, stream_pda);

    let r = state.transition_from_public_transaction(
        &signed_pause_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_STREAM_NOT_ACTIVE);
}

#[test]
fn test_resume_closed_fails() {
    let t0: Timestamp = 8;
    let (_, mock_clock_account_id) = create_keypair(97);
    let (_, provider_account_id) = create_keypair(98);

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

    let (stream_id, stream_pda, account_ids) = first_stream_accounts(
        program_id,
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        mock_clock_account_id,
    );

    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            stream_id,
            provider_account_id,
            2 as TokensPerSecond,
            300 as Balance,
            &account_ids,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    force_stream_state_closed(&mut state, stream_pda);

    let r = state.transition_from_public_transaction(
        &signed_resume_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_STREAM_NOT_PAUSED);
}

#[test]
fn test_pause_stream_time_regression_fails() {
    let t0: Timestamp = 100;
    let t_bad: Timestamp = 50;
    let (_, mock_clock_account_id) = create_keypair(99);
    let (_, provider_account_id) = create_keypair(100);

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

    let (stream_id, _, account_ids) = first_stream_accounts(
        program_id,
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        mock_clock_account_id,
    );

    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            stream_id,
            provider_account_id,
            1 as TokensPerSecond,
            400 as Balance,
            &account_ids,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    force_mock_timestamp_account(&mut state, mock_clock_account_id, MockTimestamp::new(t_bad));

    let r = state.transition_from_public_transaction(
        &signed_pause_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_TIME_REGRESSION);
}

#[test]
fn test_resume_then_accrual_ignores_paused_gap() {
    let t0: Timestamp = 100;
    let t1: Timestamp = 105;
    let t_gap: Timestamp = 200;
    let t2: Timestamp = 210;
    let (_, mock_clock_account_id) = create_keypair(101);
    let (_, provider_account_id) = create_keypair(102);

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

    let (stream_id, stream_pda, account_ids) = first_stream_accounts(
        program_id,
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        mock_clock_account_id,
    );

    let rate = 10 as TokensPerSecond;
    let allocation = 500 as Balance;
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

    force_mock_timestamp_account(&mut state, mock_clock_account_id, MockTimestamp::new(t1));
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

    transition_ok(
        &mut state,
        &signed_pause_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(4),
            &owner_private_key,
        ),
        5 as BlockId,
        "pause_stream failed",
    );

    force_mock_timestamp_account(&mut state, mock_clock_account_id, MockTimestamp::new(t_gap));
    transition_ok(
        &mut state,
        &signed_resume_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(5),
            &owner_private_key,
        ),
        6 as BlockId,
        "resume_stream failed",
    );

    force_mock_timestamp_account(&mut state, mock_clock_account_id, MockTimestamp::new(t2));
    transition_ok(
        &mut state,
        &signed_sync_stream(
            program_id,
            vault_id,
            stream_id,
            &account_ids,
            Nonce(6),
            &owner_private_key,
        ),
        7 as BlockId,
        "sync_stream after resume failed",
    );

    let cfg = StreamConfig::from_bytes(&state.get_account_by_id(stream_pda).data).expect("stream");
    let expected_accrued = 50 + (u128::from(rate) * u128::from(t2 - t_gap));
    assert_eq!(cfg.accrued, expected_accrued);
    assert_eq!(cfg.accrued_as_of, t2);
    assert_eq!(cfg.state, StreamState::Active);
}

#[test]
fn test_pause_stream_owner_mismatch_fails() {
    let signer_account_balance = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let block_init = 1 as BlockId;
    let block_deposit = 2 as BlockId;
    let block_stream = 3 as BlockId;
    let block_pause = 4 as BlockId;
    let nonce_init = Nonce(0);
    let nonce_deposit = Nonce(1);
    let nonce_stream = Nonce(2);
    let nonce_pause = Nonce(0);

    let (owner_private_key, owner_account_id) = create_keypair(1);
    let (other_private_key, other_account_id) = create_keypair(2);
    let (_, mock_clock_account_id) = create_keypair(103);
    let (_, provider_account_id) = create_keypair(104);

    let initial_accounts_data = vec![
        (owner_account_id, signer_account_balance),
        (other_account_id, signer_account_balance),
    ];
    let (mut state, guest_program) = create_state_with_guest_program(&initial_accounts_data)
        .expect("guest image present and state genesis ok");
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
        &build_signed_public_tx(
            program_id,
            Instruction::Deposit {
                vault_id,
                amount: deposit_amount,
                authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
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
        "deposit failed",
    );

    force_mock_timestamp_account(
        &mut state,
        mock_clock_account_id,
        MockTimestamp::new(DEFAULT_MOCK_CLOCK_INITIAL_TS),
    );

    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, 0);
    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            0,
            provider_account_id,
            1 as TokensPerSecond,
            100 as Balance,
            &[
                vault_config_account_id,
                vault_holding_account_id,
                stream_pda,
                owner_account_id,
                mock_clock_account_id,
            ],
            nonce_stream,
            &owner_private_key,
        ),
        block_stream,
        "create_stream failed",
    );

    let account_ids_pause = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        other_account_id,
        mock_clock_account_id,
    ];
    let r = state.transition_from_public_transaction(
        &signed_pause_stream(
            program_id,
            vault_id,
            0,
            &account_ids_pause,
            nonce_pause,
            &other_private_key,
        ),
        block_pause,
    );
    assert_execution_failed_with_code(r, ERR_VAULT_OWNER_MISMATCH);
}

#[test]
fn test_pause_stream_wrong_vault_id_fails() {
    let (_, mock_clock_account_id) = create_keypair(105);
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

    let (_, provider_account_id) = create_keypair(106);
    let (stream_id, _, account_ids) = first_stream_accounts(
        program_id,
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        mock_clock_account_id,
    );

    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            _vault_id,
            stream_id,
            provider_account_id,
            1 as TokensPerSecond,
            400 as Balance,
            &account_ids,
            Nonce(2),
            &owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    let wrong_vault_id = VaultId::from(999u64);
    let r = state.transition_from_public_transaction(
        &signed_pause_stream(
            program_id,
            wrong_vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &owner_private_key,
        ),
        4 as BlockId,
    );
    assert_execution_failed_with_code(r, ERR_VAULT_ID_MISMATCH);
}
