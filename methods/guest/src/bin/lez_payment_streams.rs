#![no_main]

use spel_framework::prelude::*;

use lez_payment_streams_core::{
    MockTimestamp,
    StreamConfig,
    StreamId,
    TokensPerSecond,
    VaultConfig,
    VaultHolding,
    VaultId,
    ERR_ALLOCATION_EXCEEDS_UNALLOCATED,
    ERR_BALANCE_OVERFLOW,
    ERR_INSUFFICIENT_FUNDS,
    ERR_INVALID_MOCK_TIMESTAMP,
    ERR_NEXT_STREAM_ID_OVERFLOW,
    ERR_STREAM_ID_MISMATCH,
    ERR_TOTAL_ALLOCATED_OVERFLOW,
    ERR_ZERO_DEPOSIT_AMOUNT,
    ERR_ZERO_STREAM_ALLOCATION,
    ERR_ZERO_STREAM_RATE,
    ERR_ZERO_WITHDRAW_AMOUNT,
    ERR_VERSION_MISMATCH,
    ERR_VAULT_ID_MISMATCH,
};
use nssa_core::account::{AccountId, Balance};
use nssa_core::program::ProgramId;

#[cfg(target_arch = "riscv32")]
risc0_zkvm::guest::entry!(main);

#[lez_program]
mod lez_payment_streams {
    #![cfg_attr(not(target_arch = "riscv32"), allow(dead_code))]

    #[allow(unused_imports)]
    use super::*;

    /// Parse vault config and holding from the standard two-account layout used by vault mutating
    /// instructions. Account indices match SPEL `#[account]` order: config first, holding second.
    fn parse_vault_config_and_holding(
        vault_config_with_meta: &AccountWithMetadata,
        vault_holding_with_meta: &AccountWithMetadata,
    ) -> Result<(VaultConfig, VaultHolding), SpelError> {
        let vault_config_state =
            VaultConfig::from_bytes(&vault_config_with_meta.account.data).ok_or_else(|| {
                SpelError::DeserializationError {
                    account_index: 0,
                    message: "invalid vault config data".into(),
                }
            })?;

        let vault_holding_state =
            VaultHolding::from_bytes(&vault_holding_with_meta.account.data).ok_or_else(|| {
                SpelError::DeserializationError {
                    account_index: 1,
                    message: "invalid vault holding data".into(),
                }
            })?;

        Ok((vault_config_state, vault_holding_state))
    }

    /// Shared checks for operations that require a vault owner signer and a matching `vault_id`
    /// argument (deposit, withdraw, and later stream instructions that touch both vault accounts).
    fn validate_vault_owner_invariants(
        vault_config_state: &VaultConfig,
        vault_holding_state: &VaultHolding,
        vault_id: VaultId,
        owner_account_id: AccountId,
    ) -> Result<(), SpelError> {
        if vault_config_state.version != vault_holding_state.version {
            return Err(SpelError::Custom {
                code: ERR_VERSION_MISMATCH,
                message: "version mismatch".into(),
            });
        }

        if vault_config_state.vault_id != vault_id {
            return Err(SpelError::Custom {
                code: ERR_VAULT_ID_MISMATCH,
                message: "incorrect vault id".into(),
            });
        }

        if vault_config_state.owner != owner_account_id {
            return Err(SpelError::Unauthorized {
                message: "owner mismatch".into(),
            });
        }

        Ok(())
    }

    // How `authenticated_transfer` encodes the debit amount in `ChainedCall::instruction_data`.
    // Kept as a named step for clarity, not because we expect reuse: deposit is the only
    // path that moves funds from an external signer balance into this program.
    fn serialize_transfer_amount(amount: Balance) -> Result<Vec<u32>, SpelError> {
        risc0_zkvm::serde::to_vec(&amount).map_err(|_| SpelError::SerializationError {
            message: "failed to serialize transfer amount".into(),
        })
    }

    /// Initialize a vault.
    #[instruction]
    pub fn initialize_vault(
        #[account(init, pda = [literal("vault_config"), account("owner"), arg("vault_id")])]
        vault_config_with_meta: AccountWithMetadata,
        #[account(init, pda = [literal("vault_holding"), account("vault_config"), literal("native")])]
        vault_holding_with_meta: AccountWithMetadata,
        #[account(signer)]
        owner_with_meta: AccountWithMetadata,
        vault_id: VaultId,
    ) -> SpelResult {
        let vault_config_state = VaultConfig::new(owner_with_meta.account_id, vault_id);
        let vault_holding_state = VaultHolding::new();

        let mut vault_config_account = vault_config_with_meta.account;
        let mut vault_holding_account = vault_holding_with_meta.account;

        vault_config_account.data = vault_config_state.to_bytes().try_into().unwrap();
        vault_holding_account.data = vault_holding_state.to_bytes().try_into().unwrap();

        Ok(SpelOutput::states_only(vec![
            AccountPostState::new_claimed(vault_config_account),
            AccountPostState::new_claimed(vault_holding_account),
            AccountPostState::new(owner_with_meta.account),
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
            return Err(SpelError::Custom {
                code: ERR_ZERO_DEPOSIT_AMOUNT,
                message: "zero deposit amount".into(),
            });
        }

        let (vault_config_state, vault_holding_state) = parse_vault_config_and_holding(
            &vault_config_with_meta,
            &vault_holding_with_meta,
        )?;

        validate_vault_owner_invariants(
            &vault_config_state,
            &vault_holding_state,
            vault_id,
            owner_with_meta.account_id,
        )?;

        // `pre_states` order matches authenticated-transfer: signer (source) → vault holding.
        let instruction_data = serialize_transfer_amount(amount)?;
        let transfer_call = ChainedCall {
            program_id: authenticated_transfer_program_id,
            instruction_data,
            pre_states: vec![
                owner_with_meta.clone(),
                vault_holding_with_meta.clone(),
            ],
            pda_seeds: vec![],
        };

        Ok(SpelOutput::with_chained_calls(
            vec![
                AccountPostState::new(vault_config_with_meta.account),
                AccountPostState::new(vault_holding_with_meta.account),
                AccountPostState::new(owner_with_meta.account),
            ],
            vec![transfer_call],
        ))

    }

    #[instruction]
    pub fn withdraw(
        #[account(mut, pda = [literal("vault_config"), account("owner"), arg("vault_id")])]
        vault_config_with_meta: AccountWithMetadata,
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), literal("native")])]
        vault_holding_with_meta: AccountWithMetadata,
        #[account(mut, signer)]
        owner_with_meta: AccountWithMetadata,
        #[account(mut)]
        withdraw_to: AccountWithMetadata,
        vault_id: VaultId,
        amount: Balance,
    ) -> SpelResult {
        if amount == 0 {
            return Err(SpelError::Custom {
                code: ERR_ZERO_WITHDRAW_AMOUNT,
                message: "zero withdraw amount".into(),
            });
        }

        let (vault_config_state, vault_holding_state) = parse_vault_config_and_holding(
            &vault_config_with_meta,
            &vault_holding_with_meta,
        )?;

        validate_vault_owner_invariants(
            &vault_config_state,
            &vault_holding_state,
            vault_id,
            owner_with_meta.account_id,
        )?;

        let unallocated = vault_holding_with_meta.account.balance
            .saturating_sub(vault_config_state.total_allocated);
        if amount > unallocated {
            return Err(SpelError::Custom {
                code: ERR_INSUFFICIENT_FUNDS,
                message: "withdraw exceeds unallocated vault balance".into(),
            });
        }

        // Debit vault holding and credit `withdraw_to` inside this program.
        // Chained `authenticated_transfer` cannot debit the vault PDA
        // (it is owned by this program, not the auth-transfer program).
        let mut holding_account = vault_holding_with_meta.account;
        let mut recipient_account = withdraw_to.account;

        holding_account.balance = holding_account.balance.checked_sub(amount).ok_or_else(|| {
            SpelError::Custom {
                code: ERR_INSUFFICIENT_FUNDS,
                message: "vault holding balance underflow".into(),
            }
        })?;

        recipient_account.balance = recipient_account.balance.checked_add(amount).ok_or_else(|| {
            SpelError::Custom {
                code: ERR_BALANCE_OVERFLOW,
                message: "recipient balance overflow".into(),
            }
        })?;

        Ok(SpelOutput::states_only(vec![
            AccountPostState::new(vault_config_with_meta.account),
            AccountPostState::new(holding_account),
            AccountPostState::new(owner_with_meta.account),
            AccountPostState::new(recipient_account),
        ]))
    }

    // Vault config and holding are both `mut` like deposit/withdraw
    // so every vault-scoped instruction shares the same account shape.
    // Holding is unchanged here (echoed in post state).
    #[instruction]
    pub fn create_stream(
        #[account(mut, pda = [literal("vault_config"), account("owner"), arg("vault_id")])]
        vault_config_with_meta: AccountWithMetadata,
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), literal("native")])]
        vault_holding_with_meta: AccountWithMetadata,
        #[account(init, pda = [literal("stream_config"), account("vault_config"), arg("stream_id")])]
        stream_config_with_meta: AccountWithMetadata,
        #[account(signer)]
        owner_with_meta: AccountWithMetadata,
        mock_timestamp_with_meta: AccountWithMetadata,
        vault_id: VaultId,
        stream_id: StreamId,
        provider: AccountId,
        rate: TokensPerSecond,
        allocation: Balance,
    ) -> SpelResult {
        if rate == 0 {
            return Err(SpelError::Custom {
                code: ERR_ZERO_STREAM_RATE,
                message: "zero stream rate".into(),
            });
        }
        if allocation == 0 {
            return Err(SpelError::Custom {
                code: ERR_ZERO_STREAM_ALLOCATION,
                message: "zero stream allocation".into(),
            });
        }

        let (mut vault_config_state, vault_holding_state) = parse_vault_config_and_holding(
            &vault_config_with_meta,
            &vault_holding_with_meta,
        )?;

        validate_vault_owner_invariants(
            &vault_config_state,
            &vault_holding_state,
            vault_id,
            owner_with_meta.account_id,
        )?;

        if stream_id != vault_config_state.next_stream_id {
            return Err(SpelError::Custom {
                code: ERR_STREAM_ID_MISMATCH,
                message: "stream id does not match vault next_stream_id".into(),
            });
        }

        let unallocated = vault_holding_with_meta.account.balance
            .saturating_sub(vault_config_state.total_allocated);
        if allocation > unallocated {
            return Err(SpelError::Custom {
                code: ERR_ALLOCATION_EXCEEDS_UNALLOCATED,
                message: "stream allocation exceeds unallocated vault balance".into(),
            });
        }

        let new_total_allocated = vault_config_state.total_allocated
            .checked_add(allocation).ok_or_else(|| {
                SpelError::Custom {
                    code: ERR_TOTAL_ALLOCATED_OVERFLOW,
                    message: "total allocated overflow".into(),
                }
            })?;

        let clock_state = MockTimestamp::from_bytes(&mock_timestamp_with_meta.account.data).ok_or_else(|| {
            SpelError::Custom {
                code: ERR_INVALID_MOCK_TIMESTAMP,
                message: "invalid mock timestamp account data".into(),
            }
        })?;
        let last_accrued_at = clock_state.timestamp;

        let stream_config_state =
            StreamConfig::new(stream_id, provider, rate, allocation, last_accrued_at);

        let next_stream_id = stream_id.checked_add(1).ok_or_else(|| {
            SpelError::Custom {
                code: ERR_NEXT_STREAM_ID_OVERFLOW,
                message: "next_stream_id overflow".into(),
            }
        })?;

        vault_config_state.next_stream_id = next_stream_id;
        vault_config_state.total_allocated = new_total_allocated;

        let mut vault_config_account = vault_config_with_meta.account;
        vault_config_account.data = vault_config_state.to_bytes().try_into().unwrap();

        let mut stream_account = stream_config_with_meta.account;
        stream_account.data = stream_config_state.to_bytes().try_into().unwrap();

        Ok(SpelOutput::states_only(vec![
            AccountPostState::new(vault_config_account),
            AccountPostState::new(vault_holding_with_meta.account),
            AccountPostState::new_claimed(stream_account),
            AccountPostState::new(owner_with_meta.account),
            AccountPostState::new(mock_timestamp_with_meta.account),
        ]))
    }
}
