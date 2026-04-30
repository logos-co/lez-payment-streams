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

    // Helpers are grouped in the same order they are typically used by an instruction handler:
    // parse accounts, validate relationships, load a full context, write account data, then execute
    // the instruction-specific state transition.

    // ---- Account role conventions ---- //

    // These indices match the account order declared by each `#[instruction]` signature.
    const VAULT_CONFIG_ACCOUNT_INDEX: usize = 0;
    const VAULT_HOLDING_ACCOUNT_INDEX: usize = 1;
    const STREAM_CONFIG_ACCOUNT_INDEX: usize = 2;

    // ---- Error helpers ---- //

    fn spel_err(code: ErrorCode, message: &'static str) -> SpelError {
        SpelError::Custom {
            code: code as u32,
            message: message.into(),
        }
    }

    #[derive(Clone, Copy)]
    enum ResumeFromPausedInstruction {
        ResumeStream,
        TopUpStream,
    }

    fn spel_map_resume_from_paused_error(
        code: ErrorCode,
        ix: ResumeFromPausedInstruction,
    ) -> SpelError {
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

    // ---- Parsing helpers ---- //

    fn parse_vault_accounts(
        vault_config: &AccountWithMetadata,
        vault_holding: &AccountWithMetadata,
    ) -> Result<(VaultConfig, VaultHolding), SpelError> {
        let vault_config_state =
            borsh::from_slice::<VaultConfig>(&vault_config.account.data).map_err(|_| {
                SpelError::DeserializationError {
                    account_index: VAULT_CONFIG_ACCOUNT_INDEX,
                    message: "invalid vault config data".into(),
                }
            })?;

        let vault_holding_state =
            borsh::from_slice::<VaultHolding>(&vault_holding.account.data).map_err(|_| {
                SpelError::DeserializationError {
                    account_index: VAULT_HOLDING_ACCOUNT_INDEX,
                    message: "invalid vault holding data".into(),
                }
            })?;

        Ok((vault_config_state, vault_holding_state))
    }

    fn parse_clock_timestamp(meta: &AccountWithMetadata) -> Result<Timestamp, SpelError> {
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

    fn parse_stream_account(stream_config: &AccountWithMetadata) -> Result<StreamConfig, SpelError> {
        borsh::from_slice::<StreamConfig>(&stream_config.account.data).map_err(|_| {
            SpelError::DeserializationError {
                account_index: STREAM_CONFIG_ACCOUNT_INDEX,
                message: "invalid stream config data".into(),
            }
        })
    }

    // ---- Validation helpers ---- //

    fn validate_vault_structure(
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

    fn validate_vault_owner(
        vault_config_state: &VaultConfig,
        owner_account_id: AccountId,
    ) -> Result<(), SpelError> {
        if vault_config_state.owner != owner_account_id {
            return Err(spel_err(ErrorCode::VaultOwnerMismatch, "owner mismatch"));
        }

        Ok(())
    }

    fn validate_stream_binding_against_vault(
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
        Ok(())
    }

    fn validate_stream_local_invariants(stream_config: &StreamConfig) -> Result<(), SpelError> {
        stream_config.validate_invariants().map_err(|code| {
            let message = match code {
                ErrorCode::ZeroStreamRate => "zero stream rate",
                ErrorCode::ZeroStreamAllocation => "zero stream allocation",
                ErrorCode::StreamExceedsAllocation => "accrued exceeds allocation",
                _ => "invalid stream config",
            };
            spel_err(code, message)
        })
    }

    // ---- Shared account loaders ---- //

    /// Load and validate vault, stream, and clock for instructions where the **vault owner is
    /// the transaction signer** (pause, resume, top-up).
    /// The `owner_account_id` parameter is the account id of the signing owner account.
    fn load_owner_stream_context(
        vault_config: &AccountWithMetadata,
        vault_holding: &AccountWithMetadata,
        stream_config: &AccountWithMetadata,
        clock_account: &AccountWithMetadata,
        vault_id: VaultId,
        stream_id: StreamId,
        owner_account_id: AccountId,
    ) -> Result<(VaultConfig, VaultHolding, StreamConfig, Timestamp), SpelError> {
        let (vault_config_state, vault_holding_state) = parse_vault_accounts(
            vault_config,
            vault_holding,
        )?;

        validate_vault_structure(&vault_config_state, &vault_holding_state, vault_id)?;
        validate_vault_owner(&vault_config_state, owner_account_id)?;

        let stream_config_state = parse_stream_account(stream_config)?;

        validate_stream_binding_against_vault(
            &stream_config_state,
            &vault_config_state,
            &vault_holding_state,
            stream_id,
        )?;
        validate_stream_local_invariants(&stream_config_state)?;

        let now = parse_clock_timestamp(clock_account)?;

        Ok((vault_config_state, vault_holding_state, stream_config_state, now))
    }

    /// Load and validate vault, stream, and clock for instructions where the **owner is an
    /// explicit non-signing account** and the actual signer is a different authority (close)
    /// or the stream provider (claim).
    /// `owner_account_id` is still checked against `VaultConfig.owner` as defense in depth
    /// alongside the PDA binding; the owner account does not need to sign.
    fn load_stream_context_with_explicit_owner(
        vault_config: &AccountWithMetadata,
        vault_holding: &AccountWithMetadata,
        stream_config: &AccountWithMetadata,
        clock_account: &AccountWithMetadata,
        vault_id: VaultId,
        stream_id: StreamId,
        owner_account_id: AccountId,
    ) -> Result<(VaultConfig, VaultHolding, StreamConfig, Timestamp), SpelError> {
        let (vault_config_state, vault_holding_state) = parse_vault_accounts(
            vault_config,
            vault_holding,
        )?;

        validate_vault_structure(
            &vault_config_state,
            &vault_holding_state,
            vault_id,
        )?;

        validate_vault_owner(&vault_config_state, owner_account_id)?;

        let stream_config_state = parse_stream_account(stream_config)?;

        validate_stream_binding_against_vault(
            &stream_config_state,
            &vault_config_state,
            &vault_holding_state,
            stream_id,
        )?;
        validate_stream_local_invariants(&stream_config_state)?;

        let now = parse_clock_timestamp(clock_account)?;

        Ok((vault_config_state, vault_holding_state, stream_config_state, now))
    }

    // ---- Shared output helpers ---- //

    /// Shared account order for owner-authorized stream instructions:
    /// `[vault_config, vault_holding, stream_config, owner, clock_account]`.
    fn execute_owner_stream_instruction(
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

    /// Shared account order for instructions with an explicit non-signing owner:
    /// `[vault_config, vault_holding, stream_config, owner, signer, clock_account]`.
    fn execute_stream_instruction_with_explicit_owner(
        vault_config_account: Account,
        vault_holding_account: Account,
        stream_account: Account,
        owner_account: Account,
        signer_account: Account,
        clock_account: Account,
    ) -> SpelOutput {
        SpelOutput::execute(
            vec![
                vault_config_account,
                vault_holding_account,
                stream_account,
                owner_account,
                signer_account,
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

    fn write_account_data(account: &mut Account, state: &impl borsh::BorshSerialize) {
        account.data = borsh::to_vec(state).unwrap().try_into().unwrap();
    }

    // ---- Vault instructions ---- //

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

        write_account_data(&mut vault_config.account, &vault_config_state);
        write_account_data(&mut vault_holding.account, &vault_holding_state);

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

        let (vault_config_state, vault_holding_state) = parse_vault_accounts(
            &vault_config,
            &vault_holding,
        )?;

        validate_vault_structure(&vault_config_state, &vault_holding_state, vault_id)?;
        validate_vault_owner(&vault_config_state, owner.account_id)?;

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

        let (vault_config_state, vault_holding_state) = parse_vault_accounts(
            &vault_config,
            &vault_holding,
        )?;

        validate_vault_structure(&vault_config_state, &vault_holding_state, vault_id)?;
        validate_vault_owner(&vault_config_state, owner.account_id)?;

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

        vault_holding.account.balance = vault_holding
            .account
            .balance
            .checked_sub(amount)
            .ok_or_else(|| spel_err(ErrorCode::InsufficientFunds, "vault holding balance underflow"))?;

        withdraw_to.account.balance = withdraw_to
            .account
            .balance
            .checked_add(amount)
            .ok_or_else(|| spel_err(ErrorCode::ArithmeticOverflow, "recipient balance overflow"))?;

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

    // ---- Stream instructions ---- //

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

        let (mut vault_config_state, vault_holding_state) = parse_vault_accounts(
            &vault_config,
            &vault_holding,
        )?;

        validate_vault_structure(&vault_config_state, &vault_holding_state, vault_id)?;
        validate_vault_owner(&vault_config_state, owner.account_id)?;

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

        let accrued_as_of = parse_clock_timestamp(&clock_account)?;

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

        write_account_data(&mut vault_config.account, &vault_config_state);
        write_account_data(&mut stream_config.account, &stream_config_state);

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
        let (_, _, mut stream_config_state, now) = load_owner_stream_context(
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
        write_account_data(&mut stream_account, &stream_config_state);

        Ok(execute_owner_stream_instruction(
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
        let (_, _, mut stream_config_state, now) = load_owner_stream_context(
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
            .map_err(|e| {
                spel_map_resume_from_paused_error(e, ResumeFromPausedInstruction::ResumeStream)
            })?;

        let vault_config_account = vault_config.account;
        let mut stream_account = stream_config.account;
        write_account_data(&mut stream_account, &stream_config_state);

        Ok(execute_owner_stream_instruction(
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
        let (mut vault_config_state, _, mut stream_config_state, now) = load_owner_stream_context(
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
                .map_err(|e| {
                    spel_map_resume_from_paused_error(e, ResumeFromPausedInstruction::TopUpStream)
                })?;
        }

        let mut vault_config_account = vault_config.account;
        write_account_data(&mut vault_config_account, &vault_config_state);

        let mut stream_account = stream_config.account;
        write_account_data(&mut stream_account, &stream_config_state);

        Ok(execute_owner_stream_instruction(
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
        // The owner id is still verified against `VaultConfig.owner` in `load_stream_context_with_explicit_owner`
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
            load_stream_context_with_explicit_owner(
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

        // `close_at_time` shrinks stream allocation only by the unaccrued remainder returned to
        // the vault. Any accrued residual stays allocated on the closed stream until a later claim.
        vault_config_state.total_allocated = checked_total_allocated_after_release(
            vault_config_state.total_allocated,
            unaccrued_released,
        )
        .map_err(|e| spel_err(e, "total_allocated release failed"))?;

        let mut vault_config = vault_config;
        let mut stream_config = stream_config;

        write_account_data(&mut vault_config.account, &vault_config_state);
        write_account_data(&mut stream_config.account, &stream_after_close);

        Ok(execute_stream_instruction_with_explicit_owner(
            vault_config.account,
            vault_holding.account,
            stream_config.account,
            owner.account,
            authority.account,
            clock_account.account,
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
            load_stream_context_with_explicit_owner(
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

        // `claim_at_time` reduces stream allocation by exactly `payout`, so the vault-side
        // `total_allocated` must release the same amount to preserve allocation conservation.
        vault_config_state.total_allocated = checked_total_allocated_after_release(
            vault_config_state.total_allocated,
            payout,
        )
        .map_err(|e| spel_err(e, "total_allocated release failed"))?;

        let mut vault_config = vault_config;
        let mut vault_holding = vault_holding;
        let mut stream_config = stream_config;
        let mut provider = provider;

        vault_holding.account.balance = vault_holding
            .account
            .balance
            .checked_sub(payout)
            .ok_or_else(|| spel_err(ErrorCode::InsufficientFunds, "vault holding balance underflow"))?;

        provider.account.balance = provider
            .account
            .balance
            .checked_add(payout)
            .ok_or_else(|| spel_err(ErrorCode::ArithmeticOverflow, "provider balance overflow"))?;

        write_account_data(&mut vault_config.account, &vault_config_state);
        write_account_data(&mut stream_config.account, &stream_after_claim);

        Ok(execute_stream_instruction_with_explicit_owner(
            vault_config.account,
            vault_holding.account,
            stream_config.account,
            owner.account,
            provider.account,
            clock_account.account,
        ))
    }
}
