#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;

use nssa::{
    program::Program,
    program_deployment_transaction::{Message as DeployMessage, ProgramDeploymentTransaction},
    public_transaction::{Message, WitnessSet},
    PrivateKey, ProgramId, PublicKey, PublicTransaction, V03State,
};
use nssa_core::account::{AccountId, Balance, Nonce};
use serde::Serialize;
use spel_framework_core::pda::{compute_pda, seed_from_str};

use crate::VaultId;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("core crate should be inside workspace root")
        .to_path_buf()
}

fn guest_binary_path() -> PathBuf {
    workspace_root().join(
        "methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin",
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
        &[&vault_config_seed_1, &vault_config_seed_2, &vault_config_seed_3],
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