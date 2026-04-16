//! Shared constants and helpers for the sibling `program_tests::*` modules.
//! This file does not define any `#[test]` functions (those live in modules).

use nssa::{
    error::NssaError, program::Program, PrivateKey, ProgramId, PublicTransaction, V03State,
};
use nssa_core::{
    account::{AccountId, Balance, Data, Nonce},
    program::BlockId,
};

use crate::Instruction;
use crate::{
    test_helpers::{
        build_signed_public_tx, derive_stream_pda, force_mock_timestamp_account,
        state_with_initialized_vault,
    },
    MockTimestamp, StreamConfig, StreamId, StreamState, Timestamp, TokensPerSecond, VaultId,
};

/// Well-funded owner balance for typical integration tests.
pub(crate) const DEFAULT_OWNER_GENESIS_BALANCE: Balance = 1_000;
/// Mock clock reading after [`state_deposited_with_mock_clock`] unless a test overrides it.
pub(crate) const DEFAULT_MOCK_CLOCK_INITIAL_TS: Timestamp = 1;
/// Single deposit into vault holding after `initialize_vault` for stream-focused tests (unified fixture).
pub(crate) const DEFAULT_STREAM_TEST_DEPOSIT: Balance = 500;

/// Account order for stream instructions: vault config, holding, stream PDA, owner, mock clock.
pub(crate) type StreamIxAccounts = [AccountId; 5];

fn signed_stream_public_tx(
    program_id: ProgramId,
    instruction: Instruction,
    accounts: &StreamIxAccounts,
    nonce: Nonce,
    owner: &PrivateKey,
) -> PublicTransaction {
    build_signed_public_tx(program_id, instruction, accounts, &[nonce], &[owner])
}

pub(crate) fn first_stream_accounts(
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

pub(crate) fn signed_create_stream(
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
    signed_stream_public_tx(
        program_id,
        Instruction::CreateStream {
            vault_id,
            stream_id,
            provider,
            rate,
            allocation,
        },
        accounts,
        nonce,
        owner,
    )
}

pub(crate) fn signed_sync_stream(
    program_id: ProgramId,
    vault_id: VaultId,
    stream_id: StreamId,
    accounts: &StreamIxAccounts,
    nonce: Nonce,
    owner: &PrivateKey,
) -> PublicTransaction {
    signed_stream_public_tx(
        program_id,
        Instruction::SyncStream {
            vault_id,
            stream_id,
        },
        accounts,
        nonce,
        owner,
    )
}

pub(crate) fn signed_pause_stream(
    program_id: ProgramId,
    vault_id: VaultId,
    stream_id: StreamId,
    accounts: &StreamIxAccounts,
    nonce: Nonce,
    owner: &PrivateKey,
) -> PublicTransaction {
    signed_stream_public_tx(
        program_id,
        Instruction::PauseStream {
            vault_id,
            stream_id,
        },
        accounts,
        nonce,
        owner,
    )
}

pub(crate) fn signed_resume_stream(
    program_id: ProgramId,
    vault_id: VaultId,
    stream_id: StreamId,
    accounts: &StreamIxAccounts,
    nonce: Nonce,
    owner: &PrivateKey,
) -> PublicTransaction {
    signed_stream_public_tx(
        program_id,
        Instruction::ResumeStream {
            vault_id,
            stream_id,
        },
        accounts,
        nonce,
        owner,
    )
}

pub(crate) fn signed_top_up_stream(
    program_id: ProgramId,
    vault_id: VaultId,
    stream_id: StreamId,
    vault_total_allocated_increase: Balance,
    accounts: &StreamIxAccounts,
    nonce: Nonce,
    owner: &PrivateKey,
) -> PublicTransaction {
    signed_stream_public_tx(
        program_id,
        Instruction::TopUpStream {
            vault_id,
            stream_id,
            vault_total_allocated_increase,
        },
        accounts,
        nonce,
        owner,
    )
}

/// Deposit layout: vault config, vault holding, owner (signer).
pub(crate) fn signed_deposit(
    program_id: ProgramId,
    vault_id: VaultId,
    amount: Balance,
    vault_config: AccountId,
    vault_holding: AccountId,
    owner: AccountId,
    nonce: Nonce,
    owner_key: &PrivateKey,
) -> PublicTransaction {
    let accounts = [vault_config, vault_holding, owner];
    build_signed_public_tx(
        program_id,
        Instruction::Deposit {
            vault_id,
            amount,
            authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
        },
        &accounts,
        &[nonce],
        &[owner_key],
    )
}

pub(crate) fn transition_ok(
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

/// Test-only: set stream lifecycle to `Closed` without going through `close_stream`.
pub(crate) fn force_stream_state_closed(state: &mut V03State, stream_pda: AccountId) {
    let mut stream_cfg =
        StreamConfig::from_bytes(&state.get_account_by_id(stream_pda).data).expect("stream");
    stream_cfg.state = StreamState::Closed;
    let mut stream_account = state.get_account_by_id(stream_pda).clone();
    stream_account.data = Data::try_from(stream_cfg.to_bytes()).expect("stream payload fits");
    state.force_insert_account(stream_pda, stream_account);
}

pub(crate) fn assert_execution_failed_with_code(result: Result<(), NssaError>, code: u32) {
    match result {
        Err(NssaError::ProgramExecutionFailed(msg)) => assert!(
            msg.contains(&format!("{code}")),
            "expected error code {code} in message, got: {msg}"
        ),
        Err(other) => panic!("expected ProgramExecutionFailed with code {code}, got: {other:?}"),
        Ok(()) => panic!("expected failure with code {code}, got Ok"),
    }
}

/// Vault initialized, one deposit, mock clock inserted; ready for `create_stream` at block 3 / nonce 2.
/// Typical args: [`DEFAULT_OWNER_GENESIS_BALANCE`], [`DEFAULT_STREAM_TEST_DEPOSIT`], clock id, [`DEFAULT_MOCK_CLOCK_INITIAL_TS`].
pub(crate) fn state_deposited_with_mock_clock(
    owner_balance_start: Balance,
    deposit_amount: Balance,
    mock_clock_account_id: AccountId,
    initial_ts: Timestamp,
) -> (
    V03State,
    ProgramId,
    PrivateKey,
    AccountId,
    VaultId,
    AccountId,
    AccountId,
) {
    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_with_initialized_vault(owner_balance_start);

    let block_deposit = 2 as BlockId;
    let nonce_deposit = Nonce(1);
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

    force_mock_timestamp_account(
        &mut state,
        mock_clock_account_id,
        MockTimestamp::new(initial_ts),
    );

    (
        state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    )
}
