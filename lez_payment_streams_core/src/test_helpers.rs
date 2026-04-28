//! NSSA test harness: deploy the guest, build genesis ([`create_state_with_guest_program`]),
//! derive PDAs, sign txs ([`build_signed_public_tx`], [`state_with_initialized_vault`]).
//!
//! After you have a [`nssa::V03State`] and program id,
//! see [`crate::program_tests::common`] for deposit fixtures,
//! [`Instruction`] builders, and helpers like `transition_ok`.
//!
//! Clock overrides: prefer [`force_clock_account_monotonic`] for forward-only time; use
//! [`force_clock_account_unchecked`] when the clock must move backward or repeat the same
//! `(timestamp, block_id)` (for example time-regression failure tests).

use std::fs;
use std::path::PathBuf;

use crate::harness_seeds::{SEED_OWNER, SEED_PROVIDER, SEED_RECIPIENT};
use crate::{ClockAccountData, CLOCK_01_PROGRAM_ACCOUNT_ID};
use crate::{Instruction, StreamId, VaultConfig, VaultId, VaultPrivacyTier};
use nssa::{
    error::NssaError,
    program::Program,
    program_deployment_transaction::{Message as DeployMessage, ProgramDeploymentTransaction},
    public_transaction::{Message, WitnessSet},
    PrivateKey, ProgramId, PublicKey, PublicTransaction, V03State,
};
use nssa_core::{
    account::{Account, AccountId, Balance, Data, Nonce},
    BlockId,
};
use serde::Serialize;
use spel_framework_core::pda::{compute_pda, seed_from_str};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("core crate should be inside workspace root")
        .to_path_buf()
}

/// Load the payment-streams guest [`Program`] for PP `execute_and_prove` (same blob as deployment).
pub(crate) fn load_guest_program() -> Program {
    let guest_bytecode = fs::read(guest_binary_path())
        .expect("guest binary missing; run `cargo build -p lez_payment_streams-methods`");
    Program::new(guest_bytecode).expect("guest bytecode should be a valid Program")
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

/// System `CLOCK_01` account id (genesis) and stream provider [`AccountId`] from [`crate::harness_seeds`].
pub(crate) fn harness_clock_01_and_provider_account_ids() -> (AccountId, AccountId) {
    let provider_account_id = create_keypair(SEED_PROVIDER).1;
    (CLOCK_01_PROGRAM_ACCOUNT_ID, provider_account_id)
}

pub(crate) fn create_state_with_guest_program(
    initial_accounts_data: &[(AccountId, Balance)],
) -> Option<(V03State, Program)> {
    let guest_bytecode = fs::read(guest_binary_path()).ok()?;
    let guest_program = Program::new(guest_bytecode.clone()).ok()?;
    let mut state = V03State::new_with_genesis_accounts(initial_accounts_data, &[], 0u64);

    let deploy_message = DeployMessage::new(guest_bytecode);
    let deploy_tx = ProgramDeploymentTransaction::new(deploy_message);
    state
        .transition_from_program_deployment_transaction(&deploy_tx)
        .ok()?;

    Some((state, guest_program))
}

fn build_public_tx<T: Serialize>(
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

/// Build a signed public tx for a vault program instruction.
///
/// Arguments: program id, instruction, accounts, signer nonces, signer keys.
/// Submit via [`V03State::transition_from_public_transaction`].
/// Example ladder: `initialize_vault` at block 1, `Nonce(0)`.
/// Next public tx usually block 2, `Nonce(1)`.
/// Stream flows often use block 3, `Nonce(2)`.
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

/// Stream PDA seeds: `stream_config`, vault config PDA, `stream_id` (same as SPEL `create_stream`).
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

/// Overwrite a system clock account payload for tests (Borsh [`ClockAccountData`], clock program owner).
///
/// Does not enforce time order. Use for negative tests (time regression) and any case where the
/// clock must move backward or repeat an identical `(timestamp, block_id)` pair.
pub(crate) fn force_clock_account_unchecked(
    state: &mut V03State,
    clock_account_id: AccountId,
    block_id: u64,
    timestamp: nssa_core::Timestamp,
) {
    let clock_program_id = Program::clock().id();
    let data = ClockAccountData {
        block_id,
        timestamp,
    }
    .to_bytes();
    let account = Account {
        program_owner: clock_program_id,
        balance: 0 as Balance,
        data: Data::try_from(data).expect("clock payload fits Data limits"),
        ..Account::default()
    };
    state.force_insert_account(clock_account_id, account);
}

/// Like [`force_clock_account_unchecked`], but in debug builds asserts the new payload is strictly
/// after the previous `(timestamp, block_id)` when the account already holds a parsable
/// [`ClockAccountData`] (otherwise the first write is unconstrained).
pub(crate) fn force_clock_account_monotonic(
    state: &mut V03State,
    clock_account_id: AccountId,
    block_id: u64,
    timestamp: nssa_core::Timestamp,
) {
    let prev_acc = state.get_account_by_id(clock_account_id);
    if let Ok(prev) = borsh::from_slice::<ClockAccountData>(&prev_acc.data) {
        let before = (prev.timestamp, prev.block_id);
        let after = (timestamp, block_id);
        debug_assert!(
            after > before,
            "force_clock_account_monotonic: clock must move strictly forward (was {before:?}, set {after:?})",
        );
    }
    force_clock_account_unchecked(state, clock_account_id, block_id, timestamp);
}

/// Rewrite `VaultConfig` account data in the test harness (bypasses normal transitions).
pub(crate) fn patch_vault_config(
    state: &mut V03State,
    vault_config_account_id: AccountId,
    f: impl FnOnce(&mut VaultConfig),
) {
    let existing = state.get_account_by_id(vault_config_account_id).clone();
    let mut vc = VaultConfig::from_bytes(&existing.data).expect("vault config");
    f(&mut vc);
    let mut acc = existing;
    acc.data = Data::try_from(vc.to_bytes()).expect("vault config payload fits Data");
    state.force_insert_account(vault_config_account_id, acc);
}

/// Genesis plus `initialize_vault` (see [`state_with_initialized_vault`]).
pub(crate) struct VaultFixture {
    pub state: V03State,
    pub program_id: ProgramId,
    pub owner_private_key: PrivateKey,
    pub owner_account_id: AccountId,
    pub vault_id: VaultId,
    pub vault_config_account_id: AccountId,
    pub vault_holding_account_id: AccountId,
}

/// Like [`VaultFixture`], with a zero-balance recipient in genesis for withdraw tests.
pub(crate) struct VaultFixtureWithRecipient {
    pub vault: VaultFixture,
    pub recipient_account_id: AccountId,
}

/// Single-owner genesis with guest deployed and `initialize_vault` done.
/// Next public tx is usually block 2, signer nonce `Nonce(1)`.
pub(crate) fn state_with_initialized_vault(owner_balance: Balance) -> VaultFixture {
    state_with_initialized_vault_with_preseeded_genesis_accounts(owner_balance, &[])
}

/// Like [`state_with_initialized_vault`],
/// with extra genesis rows ([`V03State::new_with_genesis_accounts`] layout).
/// Typical uses: provider at `0` for `claim`, or a withdraw recipient.
pub(crate) fn state_with_initialized_vault_with_preseeded_genesis_accounts(
    owner_balance: Balance,
    extra_genesis_accounts: &[(AccountId, Balance)],
) -> VaultFixture {
    state_with_initialized_vault_with_preseeded_genesis_accounts_and_privacy(
        owner_balance,
        extra_genesis_accounts,
        VaultPrivacyTier::Public,
    )
}

/// Like [`state_with_initialized_vault_with_preseeded_genesis_accounts`], with explicit
/// [`VaultPrivacyTier`] on the initialized [`VaultConfig`].
pub(crate) fn state_with_initialized_vault_with_privacy_tier(
    owner_balance: Balance,
    privacy_tier: VaultPrivacyTier,
) -> VaultFixture {
    state_with_initialized_vault_with_preseeded_genesis_accounts_and_privacy(
        owner_balance,
        &[],
        privacy_tier,
    )
}

/// [`VaultPrivacyTier::PseudonymousFunder`] vault with extra genesis rows (for example a stream
/// provider at balance zero).
pub(crate) fn state_with_initialized_vault_pseudonymous_funder_preseeded(
    owner_balance: Balance,
    extra_genesis_accounts: &[(AccountId, Balance)],
) -> VaultFixture {
    state_with_initialized_vault_with_preseeded_genesis_accounts_and_privacy(
        owner_balance,
        extra_genesis_accounts,
        VaultPrivacyTier::PseudonymousFunder,
    )
}

fn state_with_initialized_vault_with_preseeded_genesis_accounts_and_privacy(
    owner_balance: Balance,
    extra_genesis_accounts: &[(AccountId, Balance)],
    privacy_tier: VaultPrivacyTier,
) -> VaultFixture {
    let (owner_private_key, owner_account_id) = create_keypair(SEED_OWNER);
    let mut initial_accounts_data = vec![(owner_account_id, owner_balance)];
    initial_accounts_data.extend_from_slice(extra_genesis_accounts);
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
        Instruction::initialize_vault(vault_id, privacy_tier),
        &account_ids_init,
        &[nonce_init],
        &[&owner_private_key],
    );
    let result_init = state.transition_from_public_transaction(&tx_init, block_init, 0u64);
    assert!(
        result_init.is_ok(),
        "initialize_vault tx failed: {:?}",
        result_init
    );

    VaultFixture {
        state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    }
}

/// Host or harness policy helper: refuse public payment-stream transitions that touch a vault
/// configured as [`VaultPrivacyTier::PseudonymousFunder`].
///
/// The guest does not enforce this; wallets or tests call this before
/// [`V03State::transition_from_public_transaction`].
pub(crate) fn assert_public_payment_streams_instruction_allowed(
    state: &V03State,
    vault_config_account_id: AccountId,
) -> Result<(), &'static str> {
    let acc = state.get_account_by_id(vault_config_account_id);
    let cfg = VaultConfig::from_bytes(acc.data.as_ref()).ok_or("invalid vault config bytes")?;
    if cfg.privacy_tier == VaultPrivacyTier::PseudonymousFunder {
        return Err("public instruction disallowed for PseudonymousFunder vault");
    }
    Ok(())
}

/// Like [`V03State::transition_from_public_transaction`], but refuses first when the touched vault
/// is [`VaultPrivacyTier::PseudonymousFunder`] (harness or product policy, not guest-enforced).
pub(crate) fn transition_public_payment_streams_tx_respecting_privacy_tier(
    state: &mut V03State,
    vault_config_account_id: AccountId,
    tx: &PublicTransaction,
    block: BlockId,
    timestamp: nssa_core::Timestamp,
) -> Result<(), NssaError> {
    assert_public_payment_streams_instruction_allowed(state, vault_config_account_id)
        .map_err(|msg| NssaError::InvalidInput(msg.into()))?;
    state.transition_from_public_transaction(tx, block, timestamp)
}

/// Test-only native transfer from `owner_id` to `vault_holding_id` without a `Deposit`
/// instruction. Used when public `Deposit` is blocked for [`VaultPrivacyTier::PseudonymousFunder`]
/// but a PP `withdraw` test still needs funded vault holding liquidity.
pub(crate) fn transfer_native_balance_for_tests(
    state: &mut V03State,
    owner_id: AccountId,
    vault_holding_id: AccountId,
    amount: Balance,
) {
    let mut owner = state.get_account_by_id(owner_id);
    let mut holding = state.get_account_by_id(vault_holding_id);
    owner.balance = owner.balance.saturating_sub(amount);
    holding.balance = holding.balance.saturating_add(amount);
    state.force_insert_account(owner_id, owner);
    state.force_insert_account(vault_holding_id, holding);
}

/// Like [`state_with_initialized_vault`],
/// plus `recipient_account_id` at balance `0` for four-account withdraw flows.
pub(crate) fn state_with_initialized_vault_with_recipient(
    owner_balance: Balance,
) -> VaultFixtureWithRecipient {
    let (_, recipient_account_id) = create_keypair(SEED_RECIPIENT);
    let vault = state_with_initialized_vault_with_preseeded_genesis_accounts(
        owner_balance,
        &[(recipient_account_id, 0 as Balance)],
    );

    VaultFixtureWithRecipient {
        vault,
        recipient_account_id,
    }
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
