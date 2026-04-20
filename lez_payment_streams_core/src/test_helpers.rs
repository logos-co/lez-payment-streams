#![allow(dead_code)]

//! Block id and nonce ladder for tests: see [`build_signed_public_tx`] and [`state_with_initialized_vault`].

use std::fs;
use std::path::PathBuf;

use nssa::{
    program::Program,
    program_deployment_transaction::{Message as DeployMessage, ProgramDeploymentTransaction},
    public_transaction::{Message, WitnessSet},
    PrivateKey, ProgramId, PublicKey, PublicTransaction, V03State,
};
use nssa_core::{
    account::{Account, AccountId, Balance, Data, Nonce},
    program::BlockId,
};
use serde::Serialize;
use spel_framework_core::pda::{compute_pda, seed_from_str};

use crate::harness_seeds::{SEED_MOCK_CLOCK, SEED_OWNER, SEED_PROVIDER, SEED_RECIPIENT};
use crate::{Instruction, MockTimestamp, StreamId, VaultId};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("core crate should be inside workspace root")
        .to_path_buf()
}

fn guest_binary_path() -> PathBuf {
    // `Program::new` expects the risc0 `ProgramBinary` blob (`*.bin`), not the raw ELF.
    // Produced by `cargo build -p lez_payment_streams-methods` (`risc0_build::embed_methods`).
    workspace_root().join(
        "target/riscv-guest/lez_payment_streams-methods/lez_payment_streams-guest/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin",
    )
}

pub(crate) fn create_keypair(seed: u8) -> (PrivateKey, AccountId) {
    let mut seed_bytes = [0u8; 32];
    seed_bytes[0] = seed;
    let private_key = PrivateKey::try_new(seed_bytes).expect("seed should produce valid key");
    let public_key = PublicKey::new_from_private_key(&private_key);
    let account_id = AccountId::from(&public_key);
    (private_key, account_id)
}

/// Default mock timestamp account and stream provider [`AccountId`]s (see [`crate::harness_seeds`]).
pub(crate) fn harness_mock_clock_and_provider_account_ids() -> (AccountId, AccountId) {
    let (_, mock_clock_account_id) = create_keypair(SEED_MOCK_CLOCK);
    let (_, provider_account_id) = create_keypair(SEED_PROVIDER);
    (mock_clock_account_id, provider_account_id)
}

pub(crate) fn create_state_with_guest_program(
    initial_accounts_data: &[(AccountId, Balance)],
) -> Option<(V03State, Program)> {
    let guest_bytecode = fs::read(guest_binary_path()).ok()?;
    let guest_program = Program::new(guest_bytecode.clone()).ok()?;
    let mut state = V03State::new_with_genesis_accounts(initial_accounts_data, &[]);

    let deploy_message = DeployMessage::new(guest_bytecode);
    let deploy_tx = ProgramDeploymentTransaction::new(deploy_message);
    state
        .transition_from_program_deployment_transaction(&deploy_tx)
        .ok()?;

    Some((state, guest_program))
}

pub(crate) fn build_public_tx<T: Serialize>(
    program_id: ProgramId,
    account_ids: &[AccountId],
    nonces: &[Nonce],
    instruction: T,
    private_keys: &[&PrivateKey],
) -> PublicTransaction {
    let message = Message::try_new(
        program_id,
        account_ids.to_vec(),
        nonces.to_vec(),
        instruction,
    )
    .expect("instruction should serialize into message");
    let witness_set = WitnessSet::for_message(&message, private_keys);
    PublicTransaction::new(message, witness_set)
}

/// Build a signed public transaction for a vault program instruction.
///
/// Argument order is meant for call sites: program id, instruction payload, account list,
/// signer nonces, then signer private keys. Caller submits with
/// [`V03State::transition_from_public_transaction`].
/// Typical ladder: `initialize_vault` at block `1` / `Nonce(0)`; next public tx at block `2` / `Nonce(1)` per signer; further txs increment block and nonce (e.g. stream tests often use block `3` / `Nonce(2)`).
pub(crate) fn build_signed_public_tx<T: Serialize>(
    program_id: ProgramId,
    instruction: T,
    account_ids: &[AccountId],
    nonces: &[Nonce],
    signer_private_keys: &[&PrivateKey],
) -> PublicTransaction {
    build_public_tx(
        program_id,
        account_ids,
        nonces,
        instruction,
        signer_private_keys,
    )
}

pub(crate) fn seed_from_u64(value: u64) -> [u8; 32] {
    let mut seed = [0u8; 32];
    seed[..8].copy_from_slice(&value.to_le_bytes());
    seed
}

pub(crate) fn derive_vault_pdas(
    program_id: ProgramId,
    owner_account_id: AccountId,
    vault_id: VaultId,
) -> (AccountId, AccountId) {
    // vault config PDA: [b"vault_config", owner, vault_id]
    let vault_config_seed_1 = seed_from_str("vault_config");
    let vault_config_seed_2 = *owner_account_id.value();
    let vault_config_seed_3 = seed_from_u64(vault_id);
    let vault_config_account_id = compute_pda(
        &program_id,
        &[
            &vault_config_seed_1,
            &vault_config_seed_2,
            &vault_config_seed_3,
        ],
    );

    // vault holding PDA: [b"vault_holding", vault_config_pda, b"native"]
    let vault_holding_seed_1 = seed_from_str("vault_holding");
    let vault_holding_seed_2 = *vault_config_account_id.value();
    let vault_holding_seed_3 = seed_from_str("native");
    let vault_holding_account_id = compute_pda(
        &program_id,
        &[
            &vault_holding_seed_1,
            &vault_holding_seed_2,
            &vault_holding_seed_3,
        ],
    );

    (vault_config_account_id, vault_holding_account_id)
}

/// Stream PDA: `[b"stream_config", vault_config_pda, stream_id]` (matches SPEL `create_stream` seeds).
pub(crate) fn derive_stream_pda(
    program_id: ProgramId,
    vault_config_account_id: AccountId,
    stream_id: StreamId,
) -> AccountId {
    let stream_seed_1 = seed_from_str("stream_config");
    let stream_seed_2 = *vault_config_account_id.value();
    let stream_seed_3 = seed_from_u64(stream_id);
    compute_pda(
        &program_id,
        &[&stream_seed_1, &stream_seed_2, &stream_seed_3],
    )
}

/// Insert or replace a read-only mock clock account (tests only).
///
/// Prefer advancing time with [`MockTimestamp::advance_by`] / [`MockTimestamp::increment`] when
/// building a [`MockTimestamp`] value. This helper accepts any `clock` payload, including a lower
/// [`MockTimestamp::timestamp`] than before, for negative tests (e.g. time regression).
pub(crate) fn force_mock_timestamp_account(
    state: &mut V03State,
    account_id: AccountId,
    clock: MockTimestamp,
) {
    let account = Account {
        program_owner: Program::authenticated_transfer_program().id(),
        balance: Balance::MIN,
        data: Data::try_from(clock.to_bytes()).expect("mock clock payload fits Data limits"),
        ..Account::default()
    };
    state.force_insert_account(account_id, account);
}

/// Single-owner genesis, guest deployed, `initialize_vault` applied.
/// Next public tx should use `block_id` `2` and signer nonce `Nonce(1)`.
pub(crate) fn state_with_initialized_vault(
    owner_balance: Balance,
) -> (
    V03State,
    ProgramId,
    PrivateKey,
    AccountId,
    VaultId,
    AccountId,
    AccountId,
) {
    state_with_initialized_vault_with_preseeded_genesis_accounts(owner_balance, &[])
}

/// Same as [`state_with_initialized_vault`], but genesis also pre-seeds extra accounts (same layout
/// as [`V03State::new_with_genesis_accounts`]), e.g. the stream [`StreamConfig::provider`] at
/// balance `0` for claim (NSSA balance rules), or a withdraw recipient.
pub(crate) fn state_with_initialized_vault_with_preseeded_genesis_accounts(
    owner_balance: Balance,
    extra_genesis: &[(AccountId, Balance)],
) -> (
    V03State,
    ProgramId,
    PrivateKey,
    AccountId,
    VaultId,
    AccountId,
    AccountId,
) {
    let (owner_private_key, owner_account_id) = create_keypair(SEED_OWNER);
    let mut initial_accounts_data = vec![(owner_account_id, owner_balance)];
    initial_accounts_data.extend_from_slice(extra_genesis);
    let (mut state, guest_program) = create_state_with_guest_program(&initial_accounts_data)
        .expect(
            "guest image present (cargo build -p lez_payment_streams-methods) and state genesis ok",
        );
    let program_id = guest_program.id();

    let vault_id = VaultId::from(1u64);
    let (vault_config_account_id, vault_holding_account_id) =
        derive_vault_pdas(program_id, owner_account_id, vault_id);
    let account_ids_init = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
    ];

    let block_init = 1 as BlockId;
    let nonce_init = Nonce(0);
    let tx_init = build_signed_public_tx(
        program_id,
        Instruction::InitializeVault { vault_id },
        &account_ids_init,
        &[nonce_init],
        &[&owner_private_key],
    );
    let result_init = state.transition_from_public_transaction(&tx_init, block_init);
    assert!(
        result_init.is_ok(),
        "initialize_vault tx failed: {:?}",
        result_init
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

/// Like [`state_with_initialized_vault`], but genesis also includes `recipient_account_id` with
/// balance `0` for four-account withdraw tests.
pub(crate) fn state_with_initialized_vault_with_recipient(
    owner_balance: Balance,
) -> (
    V03State,
    ProgramId,
    PrivateKey,
    AccountId,
    AccountId,
    VaultId,
    AccountId,
    AccountId,
) {
    let (_, recipient_account_id) = create_keypair(SEED_RECIPIENT);
    let (
        state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_with_initialized_vault_with_preseeded_genesis_accounts(
        owner_balance,
        &[(recipient_account_id, Balance::MIN)],
    );

    (
        state,
        program_id,
        owner_private_key,
        owner_account_id,
        recipient_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    )
}

pub(crate) fn assert_vault_state_unchanged(
    state: &V03State,
    owner_account_id: AccountId,
    vault_holding_account_id: AccountId,
    vault_config_account_id: AccountId,
    owner_balance: Balance,
    vault_holding_balance: Balance,
    vault_config_data_before: Data,
) {
    assert_eq!(
        state.get_account_by_id(owner_account_id).balance,
        owner_balance
    );
    assert_eq!(
        state.get_account_by_id(vault_holding_account_id).balance,
        vault_holding_balance
    );
    assert_eq!(
        state.get_account_by_id(vault_config_account_id).data,
        vault_config_data_before
    );
}

pub(crate) fn assert_vault_state_unchanged_with_recipient(
    state: &V03State,
    owner_account_id: AccountId,
    vault_holding_account_id: AccountId,
    vault_config_account_id: AccountId,
    recipient_account_id: AccountId,
    owner_balance: Balance,
    vault_holding_balance: Balance,
    recipient_balance: Balance,
    vault_config_data_before: Data,
) {
    assert_vault_state_unchanged(
        state,
        owner_account_id,
        vault_holding_account_id,
        vault_config_account_id,
        owner_balance,
        vault_holding_balance,
        vault_config_data_before,
    );
    assert_eq!(
        state.get_account_by_id(recipient_account_id).balance,
        recipient_balance
    );
}
