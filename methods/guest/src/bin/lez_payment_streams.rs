#![no_main]

use spel_framework::prelude::*;

use lez_payment_streams_core::{
    MockTimestamp,
    StreamConfig,
    StreamId,
    StreamState,
    Timestamp,
    TokensPerSecond,
    VaultConfig,
    VaultHolding,
    VaultId,
    ERR_ALLOCATION_EXCEEDS_UNALLOCATED,
    ERR_ARITHMETIC_OVERFLOW,
    ERR_INSUFFICIENT_FUNDS,
    ERR_INVALID_MOCK_TIMESTAMP,
    ERR_NEXT_STREAM_ID_OVERFLOW,
    ERR_RESUME_ZERO_REMAINING_ALLOCATION,
    ERR_STREAM_EXCEEDS_ALLOCATION,
    ERR_STREAM_ID_MISMATCH,
    ERR_STREAM_NOT_ACTIVE,
    ERR_STREAM_NOT_PAUSED,
    ERR_TOTAL_ALLOCATED_OVERFLOW,
    ERR_ZERO_DEPOSIT_AMOUNT,
    ERR_ZERO_STREAM_ALLOCATION,
    ERR_ZERO_STREAM_RATE,
    ERR_ZERO_WITHDRAW_AMOUNT,
    ERR_VERSION_MISMATCH,
    ERR_VAULT_ID_MISMATCH,
    ERR_VAULT_OWNER_MISMATCH,
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

    fn spel_custom(code: u32, message: &'static str) -> SpelError {
        SpelError::Custom {
            code,
            message: message.into(),
        }
    }

    fn parse_mock_timestamp(meta: &AccountWithMetadata) -> Result<MockTimestamp, SpelError> {
        MockTimestamp::from_bytes(&meta.account.data).ok_or_else(|| {
            spel_custom(ERR_INVALID_MOCK_TIMESTAMP, "invalid mock timestamp account data")
        })
    }

    fn stream_invariant_err(code: u32) -> SpelError {
        let message = match code {
            ERR_ZERO_STREAM_RATE => "zero stream rate",
            ERR_ZERO_STREAM_ALLOCATION => "zero stream allocation",
            ERR_STREAM_EXCEEDS_ALLOCATION => "accrued exceeds allocation",
            _ => "invalid stream config",
        };
        SpelError::Custom {
            code,
            message: message.into(),
        }
    }

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
    fn validate_vault_config(
        vault_config_state: &VaultConfig,
        vault_holding_state: &VaultHolding,
        vault_id: VaultId,
        owner_account_id: AccountId,
    ) -> Result<(), SpelError> {
        if vault_config_state.version != vault_holding_state.version {
            return Err(spel_custom(ERR_VERSION_MISMATCH, "version mismatch"));
        }

        if vault_config_state.vault_id != vault_id {
            return Err(spel_custom(ERR_VAULT_ID_MISMATCH, "incorrect vault id"));
        }

        if vault_config_state.owner != owner_account_id {
            return Err(spel_custom(ERR_VAULT_OWNER_MISMATCH, "owner mismatch"));
        }

        Ok(())
    }

    /// After deserializing a [`StreamConfig`], check version alignment with vault accounts,
    /// `stream_id` vs PDA argument, vault existence bound, and core stream invariants (rate,
    /// allocation, accrued cap). Call after [`validate_vault_config`].
    fn validate_stream_config_for_vault(
        stream_config: &StreamConfig,
        vault_config_state: &VaultConfig,
        vault_holding_state: &VaultHolding,
        stream_id: StreamId,
    ) -> Result<(), SpelError> {
        if stream_config.version != vault_config_state.version {
            return Err(spel_custom(
                ERR_VERSION_MISMATCH,
                "stream version does not match vault config",
            ));
        }
        if stream_config.version != vault_holding_state.version {
            return Err(spel_custom(
                ERR_VERSION_MISMATCH,
                "stream version does not match vault holding",
            ));
        }
        if stream_id >= vault_config_state.next_stream_id {
            return Err(spel_custom(
                ERR_STREAM_ID_MISMATCH,
                "stream does not exist for this vault",
            ));
        }
        if stream_config.stream_id != stream_id {
            return Err(spel_custom(
                ERR_STREAM_ID_MISMATCH,
                "stream id does not match account",
            ));
        }
        stream_config
            .validate_invariants()
            .map_err(stream_invariant_err)
    }

    /// Vault config and holding, deserialized stream, and mock clock `now` for owner stream
    /// instructions that share the `sync_stream` account layout (indices 0–4).
    fn load_vault_stream_and_clock(
        vault_config_with_meta: &AccountWithMetadata,
        vault_holding_with_meta: &AccountWithMetadata,
        stream_config_with_meta: &AccountWithMetadata,
        mock_timestamp_with_meta: &AccountWithMetadata,
        vault_id: VaultId,
        stream_id: StreamId,
        owner_account_id: AccountId,
    ) -> Result<(VaultConfig, VaultHolding, StreamConfig, Timestamp), SpelError> {
        let (vault_config_state, vault_holding_state) = parse_vault_config_and_holding(
            vault_config_with_meta,
            vault_holding_with_meta,
        )?;

        validate_vault_config(
            &vault_config_state,
            &vault_holding_state,
            vault_id,
            owner_account_id,
        )?;

        let stream_config_state =
            StreamConfig::from_bytes(&stream_config_with_meta.account.data).ok_or_else(|| {
                SpelError::DeserializationError {
                    account_index: 2,
                    message: "invalid stream config data".into(),
                }
            })?;

        validate_stream_config_for_vault(
            &stream_config_state,
            &vault_config_state,
            &vault_holding_state,
            stream_id,
        )?;

        let now = parse_mock_timestamp(mock_timestamp_with_meta)?.timestamp;

        Ok((vault_config_state, vault_holding_state, stream_config_state, now))
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
            return Err(spel_custom(ERR_ZERO_DEPOSIT_AMOUNT, "zero deposit amount"));
        }

        let (vault_config_state, vault_holding_state) = parse_vault_config_and_holding(
            &vault_config_with_meta,
            &vault_holding_with_meta,
        )?;

        validate_vault_config(
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
            return Err(spel_custom(ERR_ZERO_WITHDRAW_AMOUNT, "zero withdraw amount"));
        }

        let (vault_config_state, vault_holding_state) = parse_vault_config_and_holding(
            &vault_config_with_meta,
            &vault_holding_with_meta,
        )?;

        validate_vault_config(
            &vault_config_state,
            &vault_holding_state,
            vault_id,
            owner_with_meta.account_id,
        )?;

        let unallocated = vault_holding_with_meta.account.balance
            .saturating_sub(vault_config_state.total_allocated);
        if amount > unallocated {
            return Err(spel_custom(
                ERR_INSUFFICIENT_FUNDS,
                "withdraw exceeds unallocated vault balance",
            ));
        }

        // Debit vault holding and credit `withdraw_to` inside this program.
        // Chained `authenticated_transfer` cannot debit the vault PDA
        // (it is owned by this program, not the auth-transfer program).
        let mut holding_account = vault_holding_with_meta.account;
        let mut recipient_account = withdraw_to.account;

        holding_account.balance = holding_account.balance.checked_sub(amount).ok_or_else(|| {
            spel_custom(ERR_INSUFFICIENT_FUNDS, "vault holding balance underflow")
        })?;

        recipient_account.balance = recipient_account.balance.checked_add(amount).ok_or_else(|| {
            spel_custom(ERR_ARITHMETIC_OVERFLOW, "recipient balance overflow")
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
            return Err(spel_custom(ERR_ZERO_STREAM_RATE, "zero stream rate"));
        }
        if allocation == 0 {
            return Err(spel_custom(ERR_ZERO_STREAM_ALLOCATION, "zero stream allocation"));
        }

        let (mut vault_config_state, vault_holding_state) = parse_vault_config_and_holding(
            &vault_config_with_meta,
            &vault_holding_with_meta,
        )?;

        validate_vault_config(
            &vault_config_state,
            &vault_holding_state,
            vault_id,
            owner_with_meta.account_id,
        )?;

        if stream_id != vault_config_state.next_stream_id {
            return Err(spel_custom(
                ERR_STREAM_ID_MISMATCH,
                "stream id does not match vault next_stream_id",
            ));
        }

        let unallocated = vault_holding_with_meta.account.balance
            .saturating_sub(vault_config_state.total_allocated);
        if allocation > unallocated {
            return Err(spel_custom(
                ERR_ALLOCATION_EXCEEDS_UNALLOCATED,
                "stream allocation exceeds unallocated vault balance",
            ));
        }

        let new_total_allocated = vault_config_state.total_allocated
            .checked_add(allocation)
            .ok_or_else(|| spel_custom(ERR_TOTAL_ALLOCATED_OVERFLOW, "total allocated overflow"))?;

        let accrued_as_of = parse_mock_timestamp(&mock_timestamp_with_meta)?.timestamp;

        let stream_config_state =
            StreamConfig::new(stream_id, provider, rate, allocation, accrued_as_of);

        let next_stream_id = stream_id
            .checked_add(1)
            .ok_or_else(|| spel_custom(ERR_NEXT_STREAM_ID_OVERFLOW, "next_stream_id overflow"))?;

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

    /// Apply lazy accrual through the mock clock `now` and persist stream state (owner only).
    #[instruction]
    pub fn sync_stream(
        #[account(mut, pda = [literal("vault_config"), account("owner"), arg("vault_id")])]
        vault_config_with_meta: AccountWithMetadata,
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), literal("native")])]
        vault_holding_with_meta: AccountWithMetadata,
        #[account(mut, pda = [literal("stream_config"), account("vault_config"), arg("stream_id")])]
        stream_config_with_meta: AccountWithMetadata,
        #[account(signer)]
        owner_with_meta: AccountWithMetadata,
        mock_timestamp_with_meta: AccountWithMetadata,
        vault_id: VaultId,
        stream_id: StreamId,
    ) -> SpelResult {
        let (_, _, mut stream_config_state, now) = load_vault_stream_and_clock(
            &vault_config_with_meta,
            &vault_holding_with_meta,
            &stream_config_with_meta,
            &mock_timestamp_with_meta,
            vault_id,
            stream_id,
            owner_with_meta.account_id,
        )?;

        stream_config_state = stream_config_state
            .at_time(now)
            .map_err(|code| spel_custom(code, "at_time failed"))?;

        let vault_config_account = vault_config_with_meta.account;
        let mut stream_account = stream_config_with_meta.account;
        stream_account.data = stream_config_state.to_bytes().try_into().unwrap();

        Ok(SpelOutput::states_only(vec![
            AccountPostState::new(vault_config_account),
            AccountPostState::new(vault_holding_with_meta.account),
            AccountPostState::new(stream_account),
            AccountPostState::new(owner_with_meta.account),
            AccountPostState::new(mock_timestamp_with_meta.account),
        ]))
    }

    #[instruction]
    pub fn pause_stream(
        #[account(mut, pda = [literal("vault_config"), account("owner"), arg("vault_id")])]
        vault_config_with_meta: AccountWithMetadata,
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), literal("native")])]
        vault_holding_with_meta: AccountWithMetadata,
        #[account(mut, pda = [literal("stream_config"), account("vault_config"), arg("stream_id")])]
        stream_config_with_meta: AccountWithMetadata,
        #[account(signer)]
        owner_with_meta: AccountWithMetadata,
        mock_timestamp_with_meta: AccountWithMetadata,
        vault_id: VaultId,
        stream_id: StreamId,
    ) -> SpelResult {
        let (_, _, mut stream_config_state, now) = load_vault_stream_and_clock(
            &vault_config_with_meta,
            &vault_holding_with_meta,
            &stream_config_with_meta,
            &mock_timestamp_with_meta,
            vault_id,
            stream_id,
            owner_with_meta.account_id,
        )?;

        stream_config_state = stream_config_state
            .at_time(now)
            .map_err(|code| spel_custom(code, "at_time failed"))?;

        if stream_config_state.state != StreamState::Active {
            return Err(spel_custom(
                ERR_STREAM_NOT_ACTIVE,
                "stream is not active after accrual fold",
            ));
        }

        stream_config_state.state = StreamState::Paused;

        let vault_config_account = vault_config_with_meta.account;
        let mut stream_account = stream_config_with_meta.account;
        stream_account.data = stream_config_state.to_bytes().try_into().unwrap();

        Ok(SpelOutput::states_only(vec![
            AccountPostState::new(vault_config_account),
            AccountPostState::new(vault_holding_with_meta.account),
            AccountPostState::new(stream_account),
            AccountPostState::new(owner_with_meta.account),
            AccountPostState::new(mock_timestamp_with_meta.account),
        ]))
    }

    #[instruction]
    pub fn resume_stream(
        #[account(mut, pda = [literal("vault_config"), account("owner"), arg("vault_id")])]
        vault_config_with_meta: AccountWithMetadata,
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), literal("native")])]
        vault_holding_with_meta: AccountWithMetadata,
        #[account(mut, pda = [literal("stream_config"), account("vault_config"), arg("stream_id")])]
        stream_config_with_meta: AccountWithMetadata,
        #[account(signer)]
        owner_with_meta: AccountWithMetadata,
        mock_timestamp_with_meta: AccountWithMetadata,
        vault_id: VaultId,
        stream_id: StreamId,
    ) -> SpelResult {
        let (_, _, mut stream_config_state, now) = load_vault_stream_and_clock(
            &vault_config_with_meta,
            &vault_holding_with_meta,
            &stream_config_with_meta,
            &mock_timestamp_with_meta,
            vault_id,
            stream_id,
            owner_with_meta.account_id,
        )?;

        stream_config_state = stream_config_state
            .at_time(now)
            .map_err(|code| spel_custom(code, "at_time failed"))?;

        if stream_config_state.state != StreamState::Paused {
            return Err(spel_custom(
                ERR_STREAM_NOT_PAUSED,
                "stream is not paused after accrual fold",
            ));
        }

        if stream_config_state.remaining_allocation() == (0 as Balance) {
            return Err(spel_custom(
                ERR_RESUME_ZERO_REMAINING_ALLOCATION,
                "remaining allocation is zero",
            ));
        }

        stream_config_state.state = StreamState::Active;
        stream_config_state.accrued_as_of = now;

        let vault_config_account = vault_config_with_meta.account;
        let mut stream_account = stream_config_with_meta.account;
        stream_account.data = stream_config_state.to_bytes().try_into().unwrap();

        Ok(SpelOutput::states_only(vec![
            AccountPostState::new(vault_config_account),
            AccountPostState::new(vault_holding_with_meta.account),
            AccountPostState::new(stream_account),
            AccountPostState::new(owner_with_meta.account),
            AccountPostState::new(mock_timestamp_with_meta.account),
        ]))
    }
}
