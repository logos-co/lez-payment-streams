#![no_main]

use spel_framework::prelude::*;

use lez_payment_streams_core::{
    checked_total_allocated_after_add,
    checked_total_allocated_after_release,
    ClockAccountData,
    CLOCK_PROGRAM_ACCOUNT_IDS,
    ErrorCode,
    StreamConfig,
    StreamId,
    StreamState,
    Timestamp,
    TokensPerSecond,
    VersionId,
    VaultConfig,
    VaultHolding,
    VaultId,
    VaultPrivacyTier,
};
use nssa_core::account::{Account, AccountId, Balance};
use nssa_core::program::ProgramId;

#[cfg(target_arch = "riscv32")]
risc0_zkvm::guest::entry!(main);

#[lez_program(instruction = "lez_payment_streams_core::Instruction")]
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

    fn spel_err(code: ErrorCode, message: &'static str) -> SpelError {
        spel_custom(code as u32, message)
    }

    #[derive(Clone, Copy)]
    enum ResumeFromPausedInstruction {
        ResumeStream,
        TopUpStream,
    }

    fn spel_resume_from_paused_at_err(code: ErrorCode, ix: ResumeFromPausedInstruction) -> SpelError {
        let message = match (code, ix) {
            (ErrorCode::StreamNotPaused, ResumeFromPausedInstruction::ResumeStream) => {
                "stream is not paused after accrual fold"
            }
            (ErrorCode::StreamNotPaused, ResumeFromPausedInstruction::TopUpStream) => {
                "stream is not paused after top-up"
            }
            (ErrorCode::ResumeZeroUnaccrued, ResumeFromPausedInstruction::ResumeStream) => {
                "unaccrued is zero"
            }
            (ErrorCode::ResumeZeroUnaccrued, ResumeFromPausedInstruction::TopUpStream) => {
                "unaccrued is zero after top-up"
            }
            _ => "resume_from_paused_at_time failed",
        };
        spel_err(code, message)
    }

    fn parse_clock_account(meta: &AccountWithMetadata) -> Result<Timestamp, SpelError> {
        // Allowlist check against the three system clock account ids.
        // Any other account id (including a caller-supplied fake) is rejected.
        if !CLOCK_PROGRAM_ACCOUNT_IDS
            .iter()
            .any(|id| *id == meta.account_id)
        {
            return Err(spel_err(
                ErrorCode::InvalidClockAccount,
                "not a system clock account",
            ));
        }
        // `block_id` is validated structurally as part of the Borsh parse but is not used for
        // stream math. Unknown or future clock payload extensions fail here intentionally.
        let parsed: ClockAccountData =
            borsh::from_slice(meta.account.data.as_ref()).map_err(|_| {
                spel_err(ErrorCode::InvalidClockAccount, "invalid clock account data")
            })?;
        Ok(parsed.timestamp)
    }

    fn stream_invariant_err(code: ErrorCode) -> SpelError {
        let message = match code {
            ErrorCode::ZeroStreamRate => "zero stream rate",
            ErrorCode::ZeroStreamAllocation => "zero stream allocation",
            ErrorCode::StreamExceedsAllocation => "accrued exceeds allocation",
            _ => "invalid stream config",
        };
        spel_err(code, message)
    }

    fn parse_vault_config_and_holding(
        vault_config: &AccountWithMetadata,
        vault_holding: &AccountWithMetadata,
    ) -> Result<(VaultConfig, VaultHolding), SpelError> {
        let vault_config_state =
            VaultConfig::from_bytes(&vault_config.account.data).ok_or_else(|| {
                SpelError::DeserializationError {
                    account_index: 0,
                    message: "invalid vault config data".into(),
                }
            })?;

        let vault_holding_state =
            VaultHolding::from_bytes(&vault_holding.account.data).ok_or_else(|| {
                SpelError::DeserializationError {
                    account_index: 1,
                    message: "invalid vault holding data".into(),
                }
            })?;

        Ok((vault_config_state, vault_holding_state))
    }

    fn validate_vault_structural(
        vault_config_state: &VaultConfig,
        vault_holding_state: &VaultHolding,
        vault_id: VaultId,
    ) -> Result<(), SpelError> {
        if vault_config_state.version != vault_holding_state.version {
            return Err(spel_err(ErrorCode::VersionMismatch, "version mismatch"));
        }

        if vault_config_state.vault_id != vault_id {
            return Err(spel_err(ErrorCode::VaultIdMismatch, "incorrect vault id"));
        }

        Ok(())
    }

    fn validate_vault_owner_signer(
        vault_config_state: &VaultConfig,
        owner_account_id: AccountId,
    ) -> Result<(), SpelError> {
        if vault_config_state.owner != owner_account_id {
            return Err(spel_err(ErrorCode::VaultOwnerMismatch, "owner mismatch"));
        }

        Ok(())
    }

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

    fn validate_stream_config_for_vault(
        stream_config: &StreamConfig,
        vault_config_state: &VaultConfig,
        vault_holding_state: &VaultHolding,
        stream_id: StreamId,
    ) -> Result<(), SpelError> {
        if stream_config.version != vault_config_state.version {
            return Err(spel_err(
                ErrorCode::VersionMismatch,
                "stream version does not match vault config",
            ));
        }
        if stream_config.version != vault_holding_state.version {
            return Err(spel_err(
                ErrorCode::VersionMismatch,
                "stream version does not match vault holding",
            ));
        }
        if stream_id >= vault_config_state.next_stream_id {
            return Err(spel_err(
                ErrorCode::StreamIdMismatch,
                "stream does not exist for this vault",
            ));
        }
        if stream_config.stream_id != stream_id {
            return Err(spel_err(
                ErrorCode::StreamIdMismatch,
                "stream id does not match account",
            ));
        }
        stream_config
            .validate_invariants()
            .map_err(stream_invariant_err)
    }

    /// Load and validate vault, stream, and clock for instructions where the **vault owner is
    /// the transaction signer** (pause, resume, top-up).
    /// The `owner_account_id` parameter is the account id of the signing owner account.
    fn load_vault_stream_and_clock(
        vault_config: &AccountWithMetadata,
        vault_holding: &AccountWithMetadata,
        stream_config: &AccountWithMetadata,
        clock_account: &AccountWithMetadata,
        vault_id: VaultId,
        stream_id: StreamId,
        owner_account_id: AccountId,
    ) -> Result<(VaultConfig, VaultHolding, StreamConfig, Timestamp), SpelError> {
        let (vault_config_state, vault_holding_state) = parse_vault_config_and_holding(
            vault_config,
            vault_holding,
        )?;

        validate_vault_config(
            &vault_config_state,
            &vault_holding_state,
            vault_id,
            owner_account_id,
        )?;

        let stream_config_state =
            StreamConfig::from_bytes(&stream_config.account.data).ok_or_else(|| {
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

        let now = parse_clock_account(clock_account)?;

        Ok((vault_config_state, vault_holding_state, stream_config_state, now))
    }

    /// Load and validate vault, stream, and clock for instructions where the **owner is an
    /// explicit non-signing account** and the actual signer is a different authority (close)
    /// or the stream provider (claim).
    /// `owner_account_id` is still checked against `VaultConfig.owner` as defense in depth
    /// alongside the PDA binding; the owner account does not need to sign.
    fn load_vault_stream_and_clock_with_explicit_owner(
        vault_config: &AccountWithMetadata,
        vault_holding: &AccountWithMetadata,
        stream_config: &AccountWithMetadata,
        clock_account: &AccountWithMetadata,
        vault_id: VaultId,
        stream_id: StreamId,
        owner_account_id: AccountId,
    ) -> Result<(VaultConfig, VaultHolding, StreamConfig, Timestamp), SpelError> {
        let (vault_config_state, vault_holding_state) = parse_vault_config_and_holding(
            vault_config,
            vault_holding,
        )?;

        validate_vault_structural(
            &vault_config_state,
            &vault_holding_state,
            vault_id,
        )?;

        validate_vault_owner_signer(&vault_config_state, owner_account_id)?;

        let stream_config_state =
            StreamConfig::from_bytes(&stream_config.account.data).ok_or_else(|| {
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

        let now = parse_clock_account(clock_account)?;

        Ok((vault_config_state, vault_holding_state, stream_config_state, now))
    }

    fn execute_five_owner_stream_accounts(
        vault_config_account: Account,
        vault_holding_account: Account,
        stream_account: Account,
        owner_account: Account,
        clock_account: Account,
    ) -> SpelOutput {
        SpelOutput::execute(
            vec![
                vault_config_account,
                vault_holding_account,
                stream_account,
                owner_account,
                clock_account,
            ],
            vec![],
        )
    }

    fn serialize_transfer_amount(amount: Balance) -> Result<Vec<u32>, SpelError> {
        risc0_zkvm::serde::to_vec(&amount).map_err(|_| SpelError::SerializationError {
            message: "failed to serialize transfer amount".into(),
        })
    }

    #[instruction]
    pub fn initialize_vault(
        #[account(init, pda = [literal("vault_config"), account("owner"), arg("vault_id")])]
        vault_config: AccountWithMetadata,
        // The "native" seed reserves a path for future per-token vaults: changing this literal
        // to a token mint id will produce a distinct address without altering other seeds.
        #[account(init, pda = [literal("vault_holding"), account("vault_config"), literal("native")])]
        vault_holding: AccountWithMetadata,
        #[account(signer)]
        owner: AccountWithMetadata,
        vault_id: VaultId,
        privacy_tier: VaultPrivacyTier,
    ) -> SpelResult {
        let vault_config_state = VaultConfig::new(
            owner.account_id,
            vault_id,
            None::<VersionId>,
            Some(privacy_tier),
        );
        let vault_holding_state = VaultHolding::new(None::<VersionId>);

        let mut vault_config = vault_config;
        let mut vault_holding = vault_holding;

        vault_config.account.data = vault_config_state.to_bytes().try_into().unwrap();
        vault_holding.account.data = vault_holding_state.to_bytes().try_into().unwrap();

        Ok(SpelOutput::execute(
            vec![vault_config, vault_holding, owner],
            vec![],
        ))
    }

    #[instruction]
    pub fn deposit(
        #[account(mut, pda = [literal("vault_config"), account("owner"), arg("vault_id")])]
        vault_config: AccountWithMetadata,
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), literal("native")])]
        vault_holding: AccountWithMetadata,
        #[account(mut, signer)]
        owner: AccountWithMetadata,
        vault_id: VaultId,
        amount: Balance,
        authenticated_transfer_program_id: ProgramId,
    ) -> SpelResult {
        if amount == 0 {
            return Err(spel_err(ErrorCode::ZeroDepositAmount, "zero deposit amount"));
        }

        let (vault_config_state, vault_holding_state) = parse_vault_config_and_holding(
            &vault_config,
            &vault_holding,
        )?;

        validate_vault_config(
            &vault_config_state,
            &vault_holding_state,
            vault_id,
            owner.account_id,
        )?;

        // The native balance decrease on the owner's account is executed by
        // `authenticated_transfer_program`, not by this guest, because `validate_execution`
        // only allows a program to decrease balances on accounts it owns.
        // For PP deposit the caller must load this program as a `ProgramWithDependencies`
        // dependency so the PP circuit can prove the full chained call in one proof.
        // Consequently the deposit amount is always publicly visible: `vault_holding` is a
        // public PDA and its balance change appears in the public post-states.
        let instruction_data = serialize_transfer_amount(amount)?;
        let transfer_call = ChainedCall {
            program_id: authenticated_transfer_program_id,
            instruction_data,
            pre_states: vec![
                owner.clone(),
                vault_holding.clone(),
            ],
            pda_seeds: vec![],
        };

        Ok(SpelOutput::execute(
            vec![vault_config, vault_holding, owner],
            vec![transfer_call],
        ))
    }

    #[instruction]
    pub fn withdraw(
        #[account(mut, pda = [literal("vault_config"), account("owner"), arg("vault_id")])]
        vault_config: AccountWithMetadata,
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), literal("native")])]
        vault_holding: AccountWithMetadata,
        #[account(mut, signer)]
        owner: AccountWithMetadata,
        #[account(mut)]
        withdraw_to: AccountWithMetadata,
        vault_id: VaultId,
        amount: Balance,
    ) -> SpelResult {
        if amount == 0 {
            return Err(spel_err(ErrorCode::ZeroWithdrawAmount, "zero withdraw amount"));
        }

        let (vault_config_state, vault_holding_state) = parse_vault_config_and_holding(
            &vault_config,
            &vault_holding,
        )?;

        validate_vault_config(
            &vault_config_state,
            &vault_holding_state,
            vault_id,
            owner.account_id,
        )?;

        let unallocated = vault_holding.account.balance
            .saturating_sub(vault_config_state.total_allocated);
        if amount > unallocated {
            return Err(spel_err(
                ErrorCode::InsufficientFunds,
                "withdraw exceeds unallocated vault balance",
            ));
        }

        let mut vault_holding = vault_holding;
        let mut withdraw_to = withdraw_to;

        let recipient_was_default = withdraw_to.account == Account::default();

        vault_holding.account.balance = vault_holding.account.balance.checked_sub(amount).ok_or_else(|| {
            spel_err(ErrorCode::InsufficientFunds, "vault holding balance underflow")
        })?;

        withdraw_to.account.balance = withdraw_to.account.balance.checked_add(amount).ok_or_else(|| {
            spel_err(ErrorCode::ArithmeticOverflow, "recipient balance overflow")
        })?;

        // The PP circuit requires that any account modified during execution carries an ownership
        // claim if it was default-owned (Account::default()) in pre-state.  A default-owned
        // recipient is a new private commitment; claiming it here lets the circuit set
        // `program_owner` correctly before its "modified but not claimed" invariant check.
        // Public withdrawals to existing accounts are unaffected: `AutoClaim::None` is a no-op.
        let withdraw_to_claim = if recipient_was_default {
            AutoClaim::Claimed(Claim::Authorized)
        } else {
            AutoClaim::None
        };

        let vault_config_account = vault_config.account;
        let vault_holding_account = vault_holding.account;
        let owner_account = owner.account;
        let withdraw_to_account = withdraw_to.account;

        Ok(SpelOutput::execute_with_claims(
            &[
                vault_config_account,
                vault_holding_account,
                owner_account,
                withdraw_to_account,
            ],
            &[
                AutoClaim::None,
                AutoClaim::None,
                AutoClaim::None,
                withdraw_to_claim,
            ],
            vec![],
        ))
    }

    #[instruction]
    pub fn create_stream(
        #[account(mut, pda = [literal("vault_config"), account("owner"), arg("vault_id")])]
        vault_config: AccountWithMetadata,
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), literal("native")])]
        vault_holding: AccountWithMetadata,
        #[account(init, pda = [literal("stream_config"), account("vault_config"), arg("stream_id")])]
        stream_config: AccountWithMetadata,
        #[account(signer)]
        owner: AccountWithMetadata,
        clock_account: AccountWithMetadata,
        vault_id: VaultId,
        stream_id: StreamId,
        provider: AccountId,
        rate: TokensPerSecond,
        allocation: Balance,
    ) -> SpelResult {
        if rate == 0 {
            return Err(spel_err(ErrorCode::ZeroStreamRate, "zero stream rate"));
        }
        if allocation == 0 {
            return Err(spel_err(ErrorCode::ZeroStreamAllocation, "zero stream allocation"));
        }

        let (mut vault_config_state, vault_holding_state) = parse_vault_config_and_holding(
            &vault_config,
            &vault_holding,
        )?;

        validate_vault_config(
            &vault_config_state,
            &vault_holding_state,
            vault_id,
            owner.account_id,
        )?;

        if stream_id != vault_config_state.next_stream_id {
            return Err(spel_err(
                ErrorCode::StreamIdMismatch,
                "stream id does not match vault next_stream_id",
            ));
        }

        let next_vault_total_allocated = checked_total_allocated_after_add(
            vault_holding.account.balance,
            vault_config_state.total_allocated,
            allocation,
        )
        .map_err(|e| spel_err(e, "vault total_allocated increase failed"))?;

        let accrued_as_of = parse_clock_account(&clock_account)?;

        let stream_config_state = StreamConfig::new(
            stream_id,
            provider,
            rate,
            allocation,
            accrued_as_of,
            None::<VersionId>,
        );

        let next_stream_id = stream_id
            .checked_add(1)
            .ok_or_else(|| spel_err(ErrorCode::NextStreamIdOverflow, "next_stream_id overflow"))?;

        vault_config_state.next_stream_id = next_stream_id;
        vault_config_state.total_allocated = next_vault_total_allocated;

        let mut vault_config = vault_config;
        let mut stream_config = stream_config;

        vault_config.account.data = vault_config_state.to_bytes().try_into().unwrap();
        stream_config.account.data = stream_config_state.to_bytes().try_into().unwrap();

        Ok(SpelOutput::execute(
            vec![vault_config, vault_holding, stream_config, owner, clock_account],
            vec![],
        ))
    }

    #[instruction]
    pub fn pause_stream(
        #[account(mut, pda = [literal("vault_config"), account("owner"), arg("vault_id")])]
        vault_config: AccountWithMetadata,
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), literal("native")])]
        vault_holding: AccountWithMetadata,
        #[account(mut, pda = [literal("stream_config"), account("vault_config"), arg("stream_id")])]
        stream_config: AccountWithMetadata,
        #[account(signer)]
        owner: AccountWithMetadata,
        clock_account: AccountWithMetadata,
        vault_id: VaultId,
        stream_id: StreamId,
    ) -> SpelResult {
        let (_, _, mut stream_config_state, now) = load_vault_stream_and_clock(
            &vault_config,
            &vault_holding,
            &stream_config,
            &clock_account,
            vault_id,
            stream_id,
            owner.account_id,
        )?;

        stream_config_state = stream_config_state
            .at_time(now)
            .map_err(|e| spel_err(e, "at_time failed"))?;

        if stream_config_state.state != StreamState::Active {
            return Err(spel_err(
                ErrorCode::StreamNotActive,
                "stream is not active after accrual fold",
            ));
        }

        stream_config_state.state = StreamState::Paused;

        let vault_config_account = vault_config.account;
        let mut stream_account = stream_config.account;
        stream_account.data = stream_config_state.to_bytes().try_into().unwrap();

        Ok(execute_five_owner_stream_accounts(
            vault_config_account,
            vault_holding.account,
            stream_account,
            owner.account,
            clock_account.account,
        ))
    }

    #[instruction]
    pub fn resume_stream(
        #[account(mut, pda = [literal("vault_config"), account("owner"), arg("vault_id")])]
        vault_config: AccountWithMetadata,
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), literal("native")])]
        vault_holding: AccountWithMetadata,
        #[account(mut, pda = [literal("stream_config"), account("vault_config"), arg("stream_id")])]
        stream_config: AccountWithMetadata,
        #[account(signer)]
        owner: AccountWithMetadata,
        clock_account: AccountWithMetadata,
        vault_id: VaultId,
        stream_id: StreamId,
    ) -> SpelResult {
        let (_, _, mut stream_config_state, now) = load_vault_stream_and_clock(
            &vault_config,
            &vault_holding,
            &stream_config,
            &clock_account,
            vault_id,
            stream_id,
            owner.account_id,
        )?;

        stream_config_state = stream_config_state
            .at_time(now)
            .map_err(|e| spel_err(e, "at_time failed"))?;

        stream_config_state = stream_config_state
            .resume_from_paused_at_time(now)
            .map_err(|e| spel_resume_from_paused_at_err(e, ResumeFromPausedInstruction::ResumeStream))?;

        let vault_config_account = vault_config.account;
        let mut stream_account = stream_config.account;
        stream_account.data = stream_config_state.to_bytes().try_into().unwrap();

        Ok(execute_five_owner_stream_accounts(
            vault_config_account,
            vault_holding.account,
            stream_account,
            owner.account,
            clock_account.account,
        ))
    }

    #[instruction]
    pub fn top_up_stream(
        #[account(mut, pda = [literal("vault_config"), account("owner"), arg("vault_id")])]
        vault_config: AccountWithMetadata,
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), literal("native")])]
        vault_holding: AccountWithMetadata,
        #[account(mut, pda = [literal("stream_config"), account("vault_config"), arg("stream_id")])]
        stream_config: AccountWithMetadata,
        #[account(signer)]
        owner: AccountWithMetadata,
        clock_account: AccountWithMetadata,
        vault_id: VaultId,
        stream_id: StreamId,
        vault_total_allocated_increase: Balance,
    ) -> SpelResult {
        let (mut vault_config_state, _, mut stream_config_state, now) = load_vault_stream_and_clock(
            &vault_config,
            &vault_holding,
            &stream_config,
            &clock_account,
            vault_id,
            stream_id,
            owner.account_id,
        )?;

        stream_config_state = stream_config_state
            .at_time(now)
            .map_err(|e| spel_err(e, "at_time failed"))?;

        if stream_config_state.state == StreamState::Closed {
            return Err(spel_err(ErrorCode::StreamClosed, "stream is closed"));
        }

        if vault_total_allocated_increase == 0 {
            return Err(spel_err(ErrorCode::ZeroTopUpAmount, "zero top-up amount"));
        }

        let next_vault_total_allocated = checked_total_allocated_after_add(
            vault_holding.account.balance,
            vault_config_state.total_allocated,
            vault_total_allocated_increase,
        )
        .map_err(|e| spel_err(e, "vault total_allocated increase failed"))?;

        stream_config_state.allocation = stream_config_state
            .allocation
            .checked_add(vault_total_allocated_increase)
            .ok_or_else(|| spel_err(ErrorCode::ArithmeticOverflow, "stream allocation overflow"))?;

        vault_config_state.total_allocated = next_vault_total_allocated;

        if stream_config_state.state == StreamState::Paused {
            stream_config_state = stream_config_state
                .resume_from_paused_at_time(now)
                .map_err(|e| spel_resume_from_paused_at_err(e, ResumeFromPausedInstruction::TopUpStream))?;
        }

        let mut vault_config_account = vault_config.account;
        vault_config_account.data = vault_config_state.to_bytes().try_into().unwrap();

        let mut stream_account = stream_config.account;
        stream_account.data = stream_config_state.to_bytes().try_into().unwrap();

        Ok(execute_five_owner_stream_accounts(
            vault_config_account,
            vault_holding.account,
            stream_account,
            owner.account,
            clock_account.account,
        ))
    }

    #[instruction]
    pub fn close_stream(
        #[account(mut, pda = [literal("vault_config"), account("owner"), arg("vault_id")])]
        vault_config: AccountWithMetadata,
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), literal("native")])]
        vault_holding: AccountWithMetadata,
        #[account(mut, pda = [literal("stream_config"), account("vault_config"), arg("stream_id")])]
        stream_config: AccountWithMetadata,
        // `owner` is an explicit non-signing account: either the vault owner or the stream
        // provider may be `authority`, so we cannot require the owner to sign.
        // The owner id is still verified against `VaultConfig.owner` in `load_vault_stream_and_clock_with_explicit_owner`
        // as defense in depth alongside the PDA seed binding.
        #[account(mut)]
        owner: AccountWithMetadata,
        #[account(signer)]
        authority: AccountWithMetadata,
        clock_account: AccountWithMetadata,
        vault_id: VaultId,
        stream_id: StreamId,
    ) -> SpelResult {
        let (mut vault_config_state, _, stream_config_state, now) =
            load_vault_stream_and_clock_with_explicit_owner(
                &vault_config,
                &vault_holding,
                &stream_config,
                &clock_account,
                vault_id,
                stream_id,
                owner.account_id,
            )?;

        let authority_id = authority.account_id;
        if authority_id != vault_config_state.owner && authority_id != stream_config_state.provider {
            return Err(spel_err(ErrorCode::CloseUnauthorized, "not vault owner or stream provider"));
        }

        let (unaccrued_released, stream_after_close) = stream_config_state
            .close_at_time(now)
            .map_err(|e| spel_err(e, "close_at_time failed"))?;

        vault_config_state.total_allocated = checked_total_allocated_after_release(
            vault_config_state.total_allocated,
            unaccrued_released,
        )
        .map_err(|e| spel_err(e, "total_allocated release failed"))?;

        let mut vault_config = vault_config;
        let mut stream_config = stream_config;

        vault_config.account.data = vault_config_state.to_bytes().try_into().unwrap();
        stream_config.account.data = stream_after_close.to_bytes().try_into().unwrap();

        Ok(SpelOutput::execute(
            vec![vault_config, vault_holding, stream_config, owner, authority, clock_account],
            vec![],
        ))
    }

    #[instruction]
    pub fn claim(
        #[account(mut, pda = [literal("vault_config"), account("owner"), arg("vault_id")])]
        vault_config: AccountWithMetadata,
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), literal("native")])]
        vault_holding: AccountWithMetadata,
        #[account(mut, pda = [literal("stream_config"), account("vault_config"), arg("stream_id")])]
        stream_config: AccountWithMetadata,
        // `owner` is an explicit non-signing account: the provider signs, not the vault owner.
        // The owner id is verified against `VaultConfig.owner` for defense in depth alongside
        // the PDA seed binding (same pattern as `close_stream`).
        #[account(mut)]
        owner: AccountWithMetadata,
        #[account(mut, signer)]
        provider: AccountWithMetadata,
        clock_account: AccountWithMetadata,
        vault_id: VaultId,
        stream_id: StreamId,
    ) -> SpelResult {
        let (mut vault_config_state, _, stream_config_state, now) =
            load_vault_stream_and_clock_with_explicit_owner(
                &vault_config,
                &vault_holding,
                &stream_config,
                &clock_account,
                vault_id,
                stream_id,
                owner.account_id,
            )?;

        if provider.account_id != stream_config_state.provider {
            return Err(spel_err(ErrorCode::ClaimUnauthorized, "not stream provider"));
        }

        let (payout, stream_after_claim) = stream_config_state
            .claim_at_time(now)
            .map_err(|e| spel_err(e, "claim_at_time failed"))?;

        vault_config_state.total_allocated = checked_total_allocated_after_release(
            vault_config_state.total_allocated,
            payout,
        )
        .map_err(|e| spel_err(e, "total_allocated release failed"))?;

        let mut vault_config = vault_config;
        let mut vault_holding = vault_holding;
        let mut stream_config = stream_config;
        let mut provider = provider;

        vault_holding.account.balance = vault_holding.account.balance.checked_sub(payout).ok_or_else(|| {
            spel_err(ErrorCode::InsufficientFunds, "vault holding balance underflow")
        })?;

        provider.account.balance = provider.account.balance.checked_add(payout).ok_or_else(|| {
            spel_err(ErrorCode::ArithmeticOverflow, "provider balance overflow")
        })?;

        vault_config.account.data = vault_config_state.to_bytes().try_into().unwrap();
        stream_config.account.data = stream_after_claim.to_bytes().try_into().unwrap();

        Ok(SpelOutput::execute(
            vec![vault_config, vault_holding, stream_config, owner, provider, clock_account],
            vec![],
        ))
    }
}
