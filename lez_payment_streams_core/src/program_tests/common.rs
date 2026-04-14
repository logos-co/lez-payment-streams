//! Shared constants and helpers for the sibling `program_tests::*` modules.
//! This file does not define any `#[test]` functions (those live in modules).

use nssa::{error::NssaError, program::Program, PrivateKey, ProgramId, V03State};
use nssa_core::{
    account::{AccountId, Balance, Nonce},
    program::BlockId,
};

use crate::Instruction;
use crate::{
    test_helpers::{
        build_signed_public_tx, force_mock_timestamp_account, state_with_initialized_vault,
    },
    MockTimestamp, Timestamp, VaultId,
};

/// Well-funded owner balance for typical integration tests.
pub(crate) const DEFAULT_OWNER_GENESIS_BALANCE: Balance = 1_000;
/// Mock clock reading after [`state_deposited_with_mock_clock`] unless a test overrides it.
pub(crate) const DEFAULT_MOCK_CLOCK_INITIAL_TS: Timestamp = 1;
/// Single deposit into vault holding after `initialize_vault` for stream-focused tests (unified fixture).
pub(crate) const DEFAULT_STREAM_TEST_DEPOSIT: Balance = 500;

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
    let account_ids_deposit = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
    ];
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
                    &account_ids_deposit,
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
