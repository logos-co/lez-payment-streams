//! Program-derived addresses for vault, vault holding, and stream config.
//!
//! Seeds match the LEZ payment streams SPEL guest account constraints
//! (`literal(...)`, `account(...)`, `arg(...)` with `ToSeed` / `compute_pda`).
//!
//! Public PDA addresses use LEE `/v0.2/` derivation via patched
//! [`spel_framework_core::pda::compute_pda`] (aligned with LEZ 491 host validation).

use crate::{StreamId, VaultId};
use nssa_core::account::AccountId;
use nssa_core::program::ProgramId;
use spel_framework_core::pda::{compute_pda, seed_from_str};

fn seed_from_u64(value: u64) -> [u8; 32] {
    let mut seed = [0_u8; 32];
    seed[..8].copy_from_slice(&value.to_le_bytes());
    seed
}

/// Vault config and vault holding account ids for `(owner, vault_id)`.
#[must_use]
pub fn derive_vault_account_ids(
    program_id: &ProgramId,
    owner_account_id: AccountId,
    vault_id: VaultId,
) -> (AccountId, AccountId) {
    let vault_config_seed_1 = seed_from_str("vault_config");
    let vault_config_seed_2 = *owner_account_id.value();
    let vault_config_seed_3 = seed_from_u64(vault_id);
    let vault_config_account_id = compute_pda(
        program_id,
        &[
            &vault_config_seed_1,
            &vault_config_seed_2,
            &vault_config_seed_3,
        ],
    );

    let vault_holding_seed_1 = seed_from_str("vault_holding");
    let vault_holding_seed_2 = *vault_config_account_id.value();
    let vault_holding_seed_3 = seed_from_str("native");
    let vault_holding_account_id = compute_pda(
        program_id,
        &[
            &vault_holding_seed_1,
            &vault_holding_seed_2,
            &vault_holding_seed_3,
        ],
    );

    (vault_config_account_id, vault_holding_account_id)
}

/// Stream config account id from vault config plus `stream_id`.
#[must_use]
pub fn derive_stream_config_account_id(
    program_id: &ProgramId,
    vault_config_account_id: AccountId,
    stream_id: StreamId,
) -> AccountId {
    let stream_seed_1 = seed_from_str("stream_config");
    let stream_seed_2 = *vault_config_account_id.value();
    let stream_seed_3 = seed_from_u64(stream_id);
    compute_pda(
        program_id,
        &[&stream_seed_1, &stream_seed_2, &stream_seed_3],
    )
}
