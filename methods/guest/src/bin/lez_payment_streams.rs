#![no_main]

use spel_framework::prelude::*;

use lez_payment_streams_core::{VaultConfig, VaultHolding, VaultId};

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


}
