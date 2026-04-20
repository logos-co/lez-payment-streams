#![no_main]

use spel_framework::prelude::*;

use lez_payment_streams_core::{
    checked_total_allocated_after_add,
    MockTimestamp,
    StreamConfig,
    StreamId,
    StreamState,
    Timestamp,
    TokensPerSecond,
    VaultConfig,
    VaultHolding,
    VaultId,
    ERR_ARITHMETIC_OVERFLOW,
    ERR_CLOSE_UNAUTHORIZED,
    ERR_CLAIM_UNAUTHORIZED,
    ERR_INSUFFICIENT_FUNDS,
    ERR_INVALID_MOCK_TIMESTAMP,
    ERR_NEXT_STREAM_ID_OVERFLOW,
    ERR_RESUME_ZERO_UNACCRUED,
    ERR_STREAM_CLOSED,
    ERR_STREAM_EXCEEDS_ALLOCATION,
    ERR_STREAM_ID_MISMATCH,
    ERR_STREAM_NOT_ACTIVE,
    ERR_STREAM_NOT_PAUSED,
    ERR_ZERO_DEPOSIT_AMOUNT,
    ERR_ZERO_STREAM_ALLOCATION,
    ERR_ZERO_STREAM_RATE,
    ERR_ZERO_TOP_UP_AMOUNT,
    ERR_ZERO_WITHDRAW_AMOUNT,
    ERR_VERSION_MISMATCH,
    ERR_VAULT_ID_MISMATCH,
    ERR_VAULT_OWNER_MISMATCH,
};
use nssa_core::account::{AccountId, Balance};
use nssa_core::program::ProgramId;

#[cfg(target_arch = "riscv32")]
risc0_zkvm::guest::entry!(main);

#[lez_program(instruction = "lez_payment_streams_core::Instruction")]
mod lez_payment_streams {
    #![cfg_attr(not(target_arch = "riscv32"), allow(dead_code))]

    #[allow(unused_imports)]
    use super::*;

    // SPEL error mapping, mock clock parsing, vault and stream validation, shared loaders.

    fn spel_custom(code: u32, message: &'static str) -> SpelError {
        SpelError::Custom {
            code,
            message: message.into(),
        }
    }

    #[derive(Clone, Copy)]
    enum ResumeFromPausedInstruction {
        ResumeStream,
        TopUpStream,
    }

    fn spel_resume_from_paused_at_err(code: u32, ix: ResumeFromPausedInstruction) -> SpelError {
        let message = match (code, ix) {
            (ERR_STREAM_NOT_PAUSED, ResumeFromPausedInstruction::ResumeStream) => {
                "stream is not paused after accrual fold"
            }
            (ERR_STREAM_NOT_PAUSED, ResumeFromPausedInstruction::TopUpStream) => {
                "stream is not paused after top-up"
            }
            (
                ERR_RESUME_ZERO_UNACCRUED,
                ResumeFromPausedInstruction::ResumeStream,
            ) => "unaccrued is zero",
            (
                ERR_RESUME_ZERO_UNACCRUED,
                ResumeFromPausedInstruction::TopUpStream,
            ) => "unaccrued is zero after top-up",
            _ => "resume_from_paused_at failed",
        };
        spel_custom(code, message)
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

    /// Read vault config and holding from the usual two-account layout for vault instructions.
    /// SPEL order: config, then holding.
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

    /// Check vault config, holding, and instruction `vault_id` agree (no signer check).
    fn validate_vault_structural(
        vault_config_state: &VaultConfig,
        vault_holding_state: &VaultHolding,
        vault_id: VaultId,
    ) -> Result<(), SpelError> {
        if vault_config_state.version != vault_holding_state.version {
            return Err(spel_custom(ERR_VERSION_MISMATCH, "version mismatch"));
        }

        if vault_config_state.vault_id != vault_id {
            return Err(spel_custom(ERR_VAULT_ID_MISMATCH, "incorrect vault id"));
        }

        Ok(())
    }

    fn validate_vault_owner_signer(
        vault_config_state: &VaultConfig,
        owner_account_id: AccountId,
    ) -> Result<(), SpelError> {
        if vault_config_state.owner != owner_account_id {
            return Err(spel_custom(ERR_VAULT_OWNER_MISMATCH, "owner mismatch"));
        }

        Ok(())
    }

    /// Structural validation plus vault owner as signer
    /// (deposit, withdraw, `create_stream`, `top_up`, owner-only stream paths).
    fn validate_vault_config(
        vault_config_state: &VaultConfig,
        vault_holding_state: &VaultHolding,
        vault_id: VaultId,
        owner_account_id: AccountId,
    ) -> Result<(), SpelError> {
        validate_vault_structural(vault_config_state, vault_holding_state, vault_id)?;
        validate_vault_owner_signer(vault_config_state, owner_account_id)?;
        Ok(())
    }

    /// Validate a deserialized [`StreamConfig`]:
    /// versions match vault accounts, `stream_id` fits the vault,
    /// PDA arg matches, invariants hold.
    /// Call after [`validate_vault_config`] for owner-signed instructions.
    /// Call after [`validate_vault_structural`] for `close_stream` when the provider signs.
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

    /// Load vault config, holding, stream, and mock clock `now` for owner stream instructions with the `sync_stream` five-account layout.
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

    // Serialize the debit amount for `authenticated_transfer` inside `ChainedCall::instruction_data` (`deposit` only today).
    fn serialize_transfer_amount(amount: Balance) -> Result<Vec<u32>, SpelError> {
        risc0_zkvm::serde::to_vec(&amount).map_err(|_| SpelError::SerializationError {
            message: "failed to serialize transfer amount".into(),
        })
    }

    // Instructions follow `lez_payment_streams_core::Instruction`.
    // Vault: `initialize_vault`, `deposit`, `withdraw`.
    // Streams: `create_stream`, `sync_stream`, `pause_stream`, `resume_stream`, `top_up_stream`.
    // `close_stream`, `claim`.

    /// Create vault config and holding PDAs for `vault_id`.
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

        // New stream allocation and the vault `total_allocated` increase use the same amount.
        let next_vault_total_allocated = checked_total_allocated_after_add(
            vault_holding_with_meta.account.balance,
            vault_config_state.total_allocated,
            allocation,
        )
        .map_err(|code| spel_custom(code, "vault total_allocated increase failed"))?;

        let accrued_as_of = parse_mock_timestamp(&mock_timestamp_with_meta)?.timestamp;

        let stream_config_state =
            StreamConfig::new(stream_id, provider, rate, allocation, accrued_as_of);

        let next_stream_id = stream_id
            .checked_add(1)
            .ok_or_else(|| spel_custom(ERR_NEXT_STREAM_ID_OVERFLOW, "next_stream_id overflow"))?;

        vault_config_state.next_stream_id = next_stream_id;
        vault_config_state.total_allocated = next_vault_total_allocated;

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

    /// Run [`StreamConfig::at_time`] with mock clock `now` and write the stream (owner).
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

        stream_config_state = stream_config_state
            .resume_from_paused_at(now)
            .map_err(|code| spel_resume_from_paused_at_err(code, ResumeFromPausedInstruction::ResumeStream))?;

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

    /// Add vault-funded allocation to the stream.
    /// For Paused streams, resume with `accrued_as_of = now`
    /// like [`StreamConfig::resume_from_paused_at`] in `resume_stream`.
    #[instruction]
    pub fn top_up_stream(
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
        vault_total_allocated_increase: Balance,
    ) -> SpelResult {
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

        let mut stream_config_state =
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

        let now = parse_mock_timestamp(&mock_timestamp_with_meta)?.timestamp;

        stream_config_state = stream_config_state
            .at_time(now)
            .map_err(|code| spel_custom(code, "at_time failed"))?;

        if stream_config_state.state == StreamState::Closed {
            return Err(spel_custom(ERR_STREAM_CLOSED, "stream is closed"));
        }

        if vault_total_allocated_increase == 0 {
            return Err(spel_custom(ERR_ZERO_TOP_UP_AMOUNT, "zero top-up amount"));
        }

        let next_vault_total_allocated = checked_total_allocated_after_add(
            vault_holding_with_meta.account.balance,
            vault_config_state.total_allocated,
            vault_total_allocated_increase,
        )
        .map_err(|code| spel_custom(code, "vault total_allocated increase failed"))?;

        stream_config_state.allocation = stream_config_state
            .allocation
            .checked_add(vault_total_allocated_increase)
            .ok_or_else(|| spel_custom(ERR_ARITHMETIC_OVERFLOW, "stream allocation overflow"))?;

        vault_config_state.total_allocated = next_vault_total_allocated;

        if stream_config_state.state == StreamState::Paused {
            // Same `accrued_as_of = now` anchor as `resume_stream`; see `resume_from_paused_at`.
            stream_config_state = stream_config_state
                .resume_from_paused_at(now)
                .map_err(|code| spel_resume_from_paused_at_err(code, ResumeFromPausedInstruction::TopUpStream))?;
        }

        let mut vault_config_account = vault_config_with_meta.account;
        vault_config_account.data = vault_config_state.to_bytes().try_into().unwrap();

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

    /// Close at mock clock time after accrual fold.
    /// Require vault owner or stream provider as signer.
    #[instruction]
    pub fn close_stream(
        #[account(mut, pda = [literal("vault_config"), account("owner"), arg("vault_id")])]
        vault_config_with_meta: AccountWithMetadata,
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), literal("native")])]
        vault_holding_with_meta: AccountWithMetadata,
        #[account(mut, pda = [literal("stream_config"), account("vault_config"), arg("stream_id")])]
        stream_config_with_meta: AccountWithMetadata,
        #[account(mut)]
        owner_with_meta: AccountWithMetadata,
        #[account(signer)]
        authority_with_meta: AccountWithMetadata,
        mock_timestamp_with_meta: AccountWithMetadata,
        vault_id: VaultId,
        stream_id: StreamId,
    ) -> SpelResult {
        let (mut vault_config_state, vault_holding_state) = parse_vault_config_and_holding(
            &vault_config_with_meta,
            &vault_holding_with_meta,
        )?;

        validate_vault_structural(
            &vault_config_state,
            &vault_holding_state,
            vault_id,
        )?;

        if owner_with_meta.account_id != vault_config_state.owner {
            return Err(spel_custom(ERR_VAULT_OWNER_MISMATCH, "owner account does not match vault"));
        }

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

        let authority = authority_with_meta.account_id;
        if authority != vault_config_state.owner && authority != stream_config_state.provider {
            return Err(spel_custom(ERR_CLOSE_UNAUTHORIZED, "not vault owner or stream provider"));
        }

        let now = parse_mock_timestamp(&mock_timestamp_with_meta)?.timestamp;

        let (next_vault_total_allocated, stream_after_close) = stream_config_state
            .close_at_time(now, vault_config_state.total_allocated)
            .map_err(|code| spel_custom(code, "close_at_time failed"))?;

        vault_config_state.total_allocated = next_vault_total_allocated;

        let mut vault_config_account = vault_config_with_meta.account;
        vault_config_account.data = vault_config_state.to_bytes().try_into().unwrap();

        let mut stream_account = stream_config_with_meta.account;
        stream_account.data = stream_after_close.to_bytes().try_into().unwrap();

        Ok(SpelOutput::states_only(vec![
            AccountPostState::new(vault_config_account),
            AccountPostState::new(vault_holding_with_meta.account),
            AccountPostState::new(stream_account),
            AccountPostState::new(owner_with_meta.account),
            AccountPostState::new(authority_with_meta.account),
            AccountPostState::new(mock_timestamp_with_meta.account),
        ]))
    }

    /// Pay accrued liquidity to the stream provider.
    /// Delegate payout math to [`StreamConfig::claim_at_time`] with mock clock `now`.
    #[instruction]
    pub fn claim(
        #[account(mut, pda = [literal("vault_config"), account("owner"), arg("vault_id")])]
        vault_config_with_meta: AccountWithMetadata,
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), literal("native")])]
        vault_holding_with_meta: AccountWithMetadata,
        #[account(mut, pda = [literal("stream_config"), account("vault_config"), arg("stream_id")])]
        stream_config_with_meta: AccountWithMetadata,
        #[account(mut)]
        owner_with_meta: AccountWithMetadata,
        #[account(mut, signer)]
        provider_with_meta: AccountWithMetadata,
        mock_timestamp_with_meta: AccountWithMetadata,
        vault_id: VaultId,
        stream_id: StreamId,
    ) -> SpelResult {
        let (mut vault_config_state, vault_holding_state) = parse_vault_config_and_holding(
            &vault_config_with_meta,
            &vault_holding_with_meta,
        )?;

        validate_vault_structural(
            &vault_config_state,
            &vault_holding_state,
            vault_id,
        )?;

        if owner_with_meta.account_id != vault_config_state.owner {
            return Err(spel_custom(ERR_VAULT_OWNER_MISMATCH, "owner account does not match vault"));
        }

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

        if provider_with_meta.account_id != stream_config_state.provider {
            return Err(spel_custom(ERR_CLAIM_UNAUTHORIZED, "not stream provider"));
        }

        let now = parse_mock_timestamp(&mock_timestamp_with_meta)?.timestamp;

        let (next_vault_total_allocated, payout, stream_after_claim) = stream_config_state
            .claim_at_time(now, vault_config_state.total_allocated)
            .map_err(|code| spel_custom(code, "claim_at_time failed"))?;

        vault_config_state.total_allocated = next_vault_total_allocated;

        let mut holding_account = vault_holding_with_meta.account;
        holding_account.balance = holding_account.balance.checked_sub(payout).ok_or_else(|| {
            spel_custom(ERR_INSUFFICIENT_FUNDS, "vault holding balance underflow")
        })?;

        let mut provider_account = provider_with_meta.account;
        provider_account.balance = provider_account.balance.checked_add(payout).ok_or_else(|| {
            spel_custom(ERR_ARITHMETIC_OVERFLOW, "provider balance overflow")
        })?;

        let mut vault_config_account = vault_config_with_meta.account;
        vault_config_account.data = vault_config_state.to_bytes().try_into().unwrap();

        let mut stream_account = stream_config_with_meta.account;
        stream_account.data = stream_after_claim.to_bytes().try_into().unwrap();

        Ok(SpelOutput::states_only(vec![
            AccountPostState::new(vault_config_account),
            AccountPostState::new(holding_account),
            AccountPostState::new(stream_account),
            AccountPostState::new(owner_with_meta.account),
            AccountPostState::new(provider_account),
            AccountPostState::new(mock_timestamp_with_meta.account),
        ]))
    }
}
