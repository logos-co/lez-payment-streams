//! Constants and helpers for [`crate::program_tests`] submodules.
//!
//! Default balances, account-layout type aliases, signed [`Instruction`] builders,
//! deposit fixtures with a system clock account, `transition_ok`, and test-only stream tweaks.
//! Guest deployment, genesis, PDAs, and raw tx wiring live in [`crate::test_helpers`].

use nssa::{
    error::NssaError, program::Program, PrivateKey, ProgramId, PublicTransaction, V03State,
};
use nssa_core::{
    account::{AccountId, Balance, Data, Nonce},
    BlockId,
};

use crate::Instruction;
use crate::{
    test_helpers::{
        build_signed_public_tx, derive_stream_pda, force_clock_account_monotonic,
        state_with_initialized_vault,
        state_with_initialized_vault_with_preseeded_genesis_accounts, VaultFixture,
    },
    StreamConfig, StreamId, StreamState, Timestamp, TokensPerSecond, VaultConfig, VaultId,
};

/// After one deposit and initial clock write (see [`state_deposited_with_clock`]).
pub(crate) struct DepositedVaultFixture {
    pub vault: VaultFixture,
    pub clock_id: AccountId,
}

/// Like [`DepositedVaultFixture`], with provider key material for `claim` flows.
pub(crate) struct DepositedVaultWithProviderFixture {
    pub deposited: DepositedVaultFixture,
    pub provider_private_key: PrivateKey,
    pub provider_account_id: AccountId,
}

/// First stream created and synced at `t1` after [`state_deposited_with_clock_and_provider`].
pub(crate) struct ClaimStreamSyncedScenario {
    pub with_provider: DepositedVaultWithProviderFixture,
    pub stream_id: StreamId,
    pub stream_pda: AccountId,
    pub _stream_ix_accounts: StreamIxAccounts,
}

/// Timestamp argument for [`V03State::transition_from_public_transaction`] in tests.
pub(crate) const TEST_PUBLIC_TX_TIMESTAMP: Timestamp = 0;

/// Well-funded owner balance for typical integration tests.
pub(crate) const DEFAULT_OWNER_GENESIS_BALANCE: Balance = 1_000;
/// Clock account timestamp after [`state_deposited_with_clock`] unless a test overrides it.
pub(crate) const DEFAULT_CLOCK_INITIAL_TS: Timestamp = 1;
/// Single deposit into vault holding after `initialize_vault` for stream-focused tests (unified fixture).
pub(crate) const DEFAULT_STREAM_TEST_DEPOSIT: Balance = 500;

/// Account order for stream instructions: vault config, holding, stream PDA, owner, clock account.
pub(crate) type StreamIxAccounts = [AccountId; 5];

/// `close_stream`: vault config, holding, stream PDA, owner (vault pubkey), authority (signer), clock.
pub(crate) type CloseStreamIxAccounts = [AccountId; 6];

/// `claim`: same six slots as [`CloseStreamIxAccounts`].
/// Index 4, stream provider (signer, payout).
/// Index 3, vault owner (non-signer).
pub(crate) type ClaimStreamIxAccounts = CloseStreamIxAccounts;

fn signed_stream_public_tx(
    program_id: ProgramId,
    instruction: Instruction,
    accounts: &StreamIxAccounts,
    nonce: Nonce,
    owner: &PrivateKey,
) -> PublicTransaction {
    build_signed_public_tx(program_id, instruction, accounts, &[nonce], &[owner])
}

/// First stream (`StreamId::MIN`) account layout for owner-signed stream instructions.
pub(crate) fn first_stream_ix_accounts(
    deposited: &DepositedVaultFixture,
) -> (StreamId, AccountId, StreamIxAccounts) {
    let stream_id = StreamId::MIN;
    let stream_pda = derive_stream_pda(
        deposited.vault.program_id,
        deposited.vault.vault_config_account_id,
        stream_id,
    );
    let account_ids = [
        deposited.vault.vault_config_account_id,
        deposited.vault.vault_holding_account_id,
        stream_pda,
        deposited.vault.owner_account_id,
        deposited.clock_id,
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

pub(crate) fn signed_close_stream(
    program_id: ProgramId,
    vault_id: VaultId,
    stream_id: StreamId,
    accounts: &CloseStreamIxAccounts,
    nonce: Nonce,
    authority: &PrivateKey,
) -> PublicTransaction {
    build_signed_public_tx(
        program_id,
        Instruction::CloseStream {
            vault_id,
            stream_id,
        },
        accounts,
        &[nonce],
        &[authority],
    )
}

pub(crate) fn signed_claim_stream(
    program_id: ProgramId,
    vault_id: VaultId,
    stream_id: StreamId,
    accounts: &ClaimStreamIxAccounts,
    nonce: Nonce,
    provider: &PrivateKey,
) -> PublicTransaction {
    build_signed_public_tx(
        program_id,
        Instruction::Claim {
            vault_id,
            stream_id,
        },
        accounts,
        &[nonce],
        &[provider],
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
        state
            .transition_from_public_transaction(tx, block, TEST_PUBLIC_TX_TIMESTAMP)
            .is_ok(),
        "{label}",
    );
}

/// `VaultHolding.balance >= VaultConfig.total_allocated` and
/// `total_allocated` equals the sum of `StreamConfig.allocation` for stream ids
/// `0 .. next_stream_id` with full stream account payloads.
pub(crate) fn assert_vault_conservation_invariants(
    state: &V03State,
    program_id: ProgramId,
    vault: &VaultFixture,
) {
    let vc = VaultConfig::from_bytes(&state.get_account_by_id(vault.vault_config_account_id).data)
        .expect("vault config");
    let holding_bal: Balance = state.get_account_by_id(vault.vault_holding_account_id).balance;
    assert!(
        holding_bal >= vc.total_allocated,
        "solvency: holding {holding_bal} < total_allocated {}",
        vc.total_allocated
    );
    let mut sum: Balance = 0;
    for stream_id in 0u64..vc.next_stream_id {
        let pda = derive_stream_pda(program_id, vault.vault_config_account_id, stream_id);
        let data = &state.get_account_by_id(pda).data;
        if data.len() != StreamConfig::SIZE {
            continue;
        }
        let sc = StreamConfig::from_bytes(data).expect("stream row");
        sum = sum
            .checked_add(sc.allocation)
            .expect("allocation sum overflow");
    }
    assert_eq!(
        sum, vc.total_allocated,
        "conservation: sum(stream.allocation) must equal total_allocated"
    );
}

/// Test-only: set stream state to `Closed` by rewriting the stream account (bypasses `close_stream`).
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

/// Vault after `initialize_vault`, one deposit, clock payload written on `clock_account_id`.
/// Typical next step is `create_stream` at block 3, nonce 2.
/// Args often match [`DEFAULT_OWNER_GENESIS_BALANCE`], [`DEFAULT_STREAM_TEST_DEPOSIT`],
/// clock id, [`DEFAULT_CLOCK_INITIAL_TS`].
pub(crate) fn state_deposited_with_clock(
    owner_balance_start: Balance,
    deposit_amount: Balance,
    clock_id: AccountId,
    initial_ts: Timestamp,
) -> DepositedVaultFixture {
    state_deposited_with_clock_impl(
        owner_balance_start,
        deposit_amount,
        clock_id,
        initial_ts,
        None,
    )
}

/// Like [`state_deposited_with_clock`],
/// with `provider_account_id` in genesis at balance `0`
/// so `claim` can credit it (NSSA needs a non-default `program_owner` when balances move).
pub(crate) fn state_deposited_with_clock_and_provider(
    owner_balance_start: Balance,
    deposit_amount: Balance,
    clock_id: AccountId,
    initial_ts: Timestamp,
    provider_private_key: PrivateKey,
    provider_account_id: AccountId,
) -> DepositedVaultWithProviderFixture {
    let deposited = state_deposited_with_clock_impl(
        owner_balance_start,
        deposit_amount,
        clock_id,
        initial_ts,
        Some((provider_account_id, 0 as Balance)),
    );
    DepositedVaultWithProviderFixture {
        deposited,
        provider_private_key,
        provider_account_id,
    }
}

/// `initialize_vault` (block 1), `deposit` (block 2, `Nonce(1)`), `create_stream` (block 3, `Nonce(2)`),
/// clock advanced to `t1`, then `sync_stream` (block 4, `Nonce(3)`).
pub(crate) fn claim_stream_prelude_synced_at_t1(
    owner_genesis_balance: Balance,
    deposit_amount: Balance,
    clock_id: AccountId,
    t0: Timestamp,
    t1: Timestamp,
    provider_private_key: PrivateKey,
    provider_account_id: AccountId,
    rate: TokensPerSecond,
    allocation: Balance,
) -> ClaimStreamSyncedScenario {
    let mut with_provider = state_deposited_with_clock_and_provider(
        owner_genesis_balance,
        deposit_amount,
        clock_id,
        t0,
        provider_private_key,
        provider_account_id,
    );
    let (stream_id, stream_pda, stream_ix_accounts) =
        first_stream_ix_accounts(&with_provider.deposited);
    transition_ok(
        &mut with_provider.deposited.vault.state,
        &signed_create_stream(
            with_provider.deposited.vault.program_id,
            with_provider.deposited.vault.vault_id,
            stream_id,
            provider_account_id,
            rate,
            allocation,
            &stream_ix_accounts,
            Nonce(2),
            &with_provider.deposited.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );
    force_clock_account_monotonic(
        &mut with_provider.deposited.vault.state,
        clock_id,
        0,
        t1,
    );
    transition_ok(
        &mut with_provider.deposited.vault.state,
        &signed_sync_stream(
            with_provider.deposited.vault.program_id,
            with_provider.deposited.vault.vault_id,
            stream_id,
            &stream_ix_accounts,
            Nonce(3),
            &with_provider.deposited.vault.owner_private_key,
        ),
        4 as BlockId,
        "sync_stream failed",
    );
    ClaimStreamSyncedScenario {
        with_provider,
        stream_id,
        stream_pda,
        _stream_ix_accounts: stream_ix_accounts,
    }
}

fn state_deposited_with_clock_impl(
    owner_balance_start: Balance,
    deposit_amount: Balance,
    clock_id: AccountId,
    initial_ts: Timestamp,
    extra_genesis: Option<(AccountId, Balance)>,
) -> DepositedVaultFixture {
    let mut vault = if let Some((account_id, balance)) = extra_genesis {
        state_with_initialized_vault_with_preseeded_genesis_accounts(
            owner_balance_start,
            &[(account_id, balance)],
        )
    } else {
        state_with_initialized_vault(owner_balance_start)
    };

    let block_deposit = 2 as BlockId;
    let nonce_deposit = Nonce(1);
    transition_ok(
        &mut vault.state,
        &signed_deposit(
            vault.program_id,
            vault.vault_id,
            deposit_amount,
            vault.vault_config_account_id,
            vault.vault_holding_account_id,
            vault.owner_account_id,
            nonce_deposit,
            &vault.owner_private_key,
        ),
        block_deposit,
        "deposit failed",
    );

    // When `initial_ts == 0`, use `block_id == 1` so the first payload is not `(0, 0)`; otherwise a
    // test's first `force_clock_account_monotonic(..., 0, 0)` would not advance `(timestamp,
    // block_id)` strictly past the fixture write.
    let init_block_id = if initial_ts == 0 { 1u64 } else { 0u64 };
    force_clock_account_monotonic(&mut vault.state, clock_id, init_block_id, initial_ts);

    DepositedVaultFixture { vault, clock_id }
}
