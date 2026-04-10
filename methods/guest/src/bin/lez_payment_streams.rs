#![no_main]

use spel_framework::prelude::*;

use lez_payment_streams_core::{
    VaultConfig,
    VaultHolding,
    VaultId,
    ERR_ZERO_DEPOSIT_AMOUNT,
    ERR_VERSION_MISMATCH,
    ERR_VAULT_ID_MISMATCH,
};
use nssa_core::account::Balance;
use nssa_core::program::ProgramId;

risc0_zkvm::guest::entry!(main);

#[lez_program]
mod lez_payment_streams {
    #[allow(unused_imports)]
    use super::*;

    /// Initialize a vault.
    #[instruction]
    pub fn initialize_vault(
        #[account(init, pda = [literal("vault_config"), account("owner"), arg("vault_id")])]
        vault_config: AccountWithMetadata,
        #[account(init, pda = [literal("vault_holding"), account("vault_config"), literal("native")])]
        vault_holding: AccountWithMetadata,
        #[account(signer)]
        owner: AccountWithMetadata,
        vault_id: VaultId,
    ) -> SpelResult {
        let vault_config_state = VaultConfig::new(owner.account_id, vault_id);
        let vault_holding_state = VaultHolding::new();

        let mut vault_config_account = vault_config.account.clone();
        let mut vault_holding_account = vault_holding.account.clone();

        vault_config_account.data = vault_config_state.to_bytes().try_into().unwrap();
        vault_holding_account.data = vault_holding_state.to_bytes().try_into().unwrap();

        Ok(SpelOutput::states_only(vec![
            AccountPostState::new_claimed(vault_config_account),
            AccountPostState::new_claimed(vault_holding_account),
            AccountPostState::new(owner.account.clone()),
        ]))
    }

    #[instruction]
    pub fn deposit(
        #[account(mut, pda = [literal("vault_config"), account("owner"), arg("vault_id")])]
        vault_config_with_meta: AccountWithMetadata,
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), literal("native")])]
        vault_holding_with_meta: AccountWithMetadata,
        #[account(mut, signer)]
        owner_with_meta: AccountWithMetadata,
        vault_id: VaultId,
        amount: Balance,
        authenticated_transfer_program_id: ProgramId,
    ) -> SpelResult {
        if amount == 0 {
            return Err(SpelError::Custom{
                code: ERR_ZERO_DEPOSIT_AMOUNT,
                message: "zero deposit amount".into(),
            });
        }
        // Parse vault state first, then apply invariant checks.
        let vault_config_account = &vault_config_with_meta.account;
        let vault_config_state = VaultConfig::from_bytes(&vault_config_account.data).ok_or_else(|| {
            SpelError::DeserializationError {
                account_index: 0,
                message: "invalid vault config data".into(),
            }
        })?;

        let vault_holding_state = VaultHolding::from_bytes(&vault_holding_with_meta.account.data).ok_or_else(|| {
            SpelError::DeserializationError {
                account_index: 1,
                message: "invalid vault holding data".into(),
            }
        })?;

        if vault_config_state.version != vault_holding_state.version {
            return Err(SpelError::Custom{
                code: ERR_VERSION_MISMATCH,
                message: "version mismatch".into(),
            });
        }

        if vault_config_state.vault_id != vault_id {
            return Err(SpelError::Custom{
                code: ERR_VAULT_ID_MISMATCH,
                message: "incorrect vault id".into(),
            });
        }

        // optional defense-in-depth (PDA should link vault to owner already)
        if vault_config_state.owner != owner_with_meta.account_id {
            return Err(SpelError::Unauthorized {
                message: "owner mismatch".into(),
            });
        }

        let transfer_instruction_data = risc0_zkvm::serde::to_vec(&amount)
            .map_err(|_| {
                SpelError::SerializationError {
                    message: "failed to serialize transfer amount".into(),
                }
            }
        )?;

        let transfer_call = ChainedCall {
            program_id: authenticated_transfer_program_id,
            instruction_data: transfer_instruction_data,
            pre_states: vec![owner_with_meta.clone(), vault_holding_with_meta.clone()],
            pda_seeds: vec![],
        };

        Ok(SpelOutput::with_chained_calls(
            vec![
                AccountPostState::new(vault_config_with_meta.account.clone()),
                AccountPostState::new(vault_holding_with_meta.account.clone()),
                AccountPostState::new(owner_with_meta.account.clone()),
            ],
            vec![transfer_call],
        ))

    }


}
