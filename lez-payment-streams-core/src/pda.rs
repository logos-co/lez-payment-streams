//! Program-derived addresses for vault, vault holding, and stream config.
//!
//! Seeds match the LEZ payment streams SPEL guest account constraints
//! (`literal(...)`, `account(...)`, `arg(...)` with `ToSeed` / `compute_pda`).
//!
//! Public PDA addresses use LEE `/v0.2/` derivation via patched
//! [`spel_framework_core::pda::compute_pda`] (aligned with LEZ 491 host validation).

use crate::{StreamId, VaultId, NATIVE_TOKEN_ID};
use lee_core::account::AccountId;
use lee_core::program::ProgramId;
use spel_framework_core::pda::{compute_pda, seed_from_str};

fn seed_from_u64(value: u64) -> [u8; 32] {
    let mut seed = [0_u8; 32];
    seed[..8].copy_from_slice(&value.to_le_bytes());
    seed
}

/// Vault config and native vault holding account ids for `(owner, vault_id)`.
#[must_use]
pub fn derive_vault_account_ids(
    program_id: &ProgramId,
    owner_account_id: AccountId,
    vault_id: VaultId,
) -> (AccountId, AccountId) {
    derive_vault_account_ids_for_token(program_id, owner_account_id, vault_id, NATIVE_TOKEN_ID)
}

/// Vault config and vault holding account ids for `(owner, vault_id, token_id)`.
///
/// Holding PDA seed 3 is the 32-byte LIP-155 `token_id` (all-zeroes for native).
#[must_use]
pub fn derive_vault_account_ids_for_token(
    program_id: &ProgramId,
    owner_account_id: AccountId,
    vault_id: VaultId,
    token_id: [u8; 32],
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
    let vault_holding_account_id = compute_pda(
        program_id,
        &[&vault_holding_seed_1, &vault_holding_seed_2, &token_id],
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

#[cfg(test)]
mod pda_seed_tests {
    use super::{derive_vault_account_ids, derive_vault_account_ids_for_token, seed_from_u64};
    use crate::NATIVE_TOKEN_ID;
    use lee_core::account::AccountId;
    use lee_core::program::ProgramId;
    use spel_framework_core::pda::{compute_pda, seed_from_str};

    fn sample_program_id() -> ProgramId {
        [0x11; 8]
    }

    #[test]
    fn native_holding_seed_is_all_zeroes_not_native_string() {
        let program_id = sample_program_id();
        let owner = AccountId::new([0x22; 32]);
        let vault_id = 7_u64;
        let (_config, native_holding) = derive_vault_account_ids(&program_id, owner, vault_id);

        let vault_config_account_id = {
            let seed_1 = seed_from_str("vault_config");
            let seed_2 = *owner.value();
            let seed_3 = seed_from_u64(vault_id);
            compute_pda(&program_id, &[&seed_1, &seed_2, &seed_3])
        };
        let legacy_string_holding = {
            let seed_1 = seed_from_str("vault_holding");
            let seed_2 = *vault_config_account_id.value();
            let seed_3 = seed_from_str("native");
            compute_pda(&program_id, &[&seed_1, &seed_2, &seed_3])
        };
        let zero_token_holding = {
            let seed_1 = seed_from_str("vault_holding");
            let seed_2 = *vault_config_account_id.value();
            compute_pda(&program_id, &[&seed_1, &seed_2, &NATIVE_TOKEN_ID])
        };

        assert_eq!(native_holding, zero_token_holding);
        assert_ne!(native_holding, legacy_string_holding);
        assert_eq!(
            derive_vault_account_ids_for_token(&program_id, owner, vault_id, NATIVE_TOKEN_ID).1,
            native_holding
        );
    }
}
