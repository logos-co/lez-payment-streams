#![no_main]
// Stock spel v0.6.0 macros expand to nssa_core::. Guest source keeps lee_core::
// via this alias (Cargo cannot name the same package twice).
extern crate nssa_core as lee_core;

use spel_framework::prelude::*;

use authenticated_transfer_core::Instruction as AuthenticatedTransferInstruction;
use lee_core::account::{Account, AccountId, Balance};
use lee_core::program::ProgramId;
use lez_payment_streams_core::{
    chain_timestamp_to_fold_seconds, checked_total_allocated_after_add,
    checked_total_allocated_after_release, require_native_token_id, ClockAccountData, ErrorCode,
    StreamConfig, StreamId, StreamState, Timestamp, TokensPerSecond, VaultConfig, VaultHolding,
    VaultId, VaultPrivacyTier, VersionId, CLOCK_PROGRAM_ACCOUNT_IDS,
};

#[cfg(target_arch = "riscv32")]
risc0_zkvm::guest::entry!(main);

#[lez_program(instruction = "lez_payment_streams_core::Instruction")]
mod lez_payment_streams {
    #![cfg_attr(not(target_arch = "riscv32"), allow(dead_code))]

    #[allow(
        unused_imports,
        reason = "imports used by guest-only code paths under cfg"
    )]
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
        let vault_config_state = borsh::from_slice::<VaultConfig>(&vault_config.account.data)
            .map_err(|_| SpelError::DeserializationError {
                account_index: VAULT_CONFIG_ACCOUNT_INDEX,
                message: "invalid vault config data".into(),
            })?;

        require_native_token_id(&vault_config_state.token_id)
            .map_err(|_| spel_err(ErrorCode::UnsupportedTokenId, "unsupported token_id"))?;

        let vault_holding_state = borsh::from_slice::<VaultHolding>(&vault_holding.account.data)
            .map_err(|_| SpelError::DeserializationError {
                account_index: VAULT_HOLDING_ACCOUNT_INDEX,
                message: "invalid vault holding data".into(),
            })?;

        Ok((vault_config_state, vault_holding_state))
    }

    fn parse_clock_timestamp(meta: &AccountWithMetadata) -> Result<Timestamp, SpelError> {
        // Allowlist check against the three system clock account ids.
        // Any other account id (including a caller-supplied fake) is rejected.
        if !CLOCK_PROGRAM_ACCOUNT_IDS.contains(&meta.account_id) {
            return Err(spel_err(
                ErrorCode::InvalidClockAccount,
                "not a system clock account",
            ));
        }
        // `block_id` is validated structurally as part of the Borsh parse but is not used for
        // stream math. Unknown or future clock payload extensions fail here intentionally.
        let parsed: ClockAccountData = borsh::from_slice(meta.account.data.as_ref())
            .map_err(|_| spel_err(ErrorCode::InvalidClockAccount, "invalid clock account data"))?;
        // LEZ clock wire is milliseconds; rates and at_time use fold seconds.
        Ok(chain_timestamp_to_fold_seconds(parsed.timestamp))
    }

    fn parse_stream_account(
        stream_config: &AccountWithMetadata,
    ) -> Result<StreamConfig, SpelError> {
        let mut stream =
            borsh::from_slice::<StreamConfig>(&stream_config.account.data).map_err(|_| {
                SpelError::DeserializationError {
                    account_index: STREAM_CONFIG_ACCOUNT_INDEX,
                    message: "invalid stream config data".into(),
                }
            })?;
        // Normalize legacy ms checkpoints so at_time matches module status folds.
        stream.accrued_as_of = chain_timestamp_to_fold_seconds(stream.accrued_as_of);
        Ok(stream)
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

    /// Load and validate vault, stream, and clock for instructions authorized by the vault owner
    /// (pause, resume, top-up, close_stream_by_owner).
    /// The `owner_account_id` parameter is the account id used for owner authorization.
    fn load_owner_stream_context(
        vault_config: &AccountWithMetadata,
        vault_holding: &AccountWithMetadata,
        stream_config: &AccountWithMetadata,
        clock_account: &AccountWithMetadata,
        vault_id: VaultId,
        stream_id: StreamId,
        owner_account_id: AccountId,
    ) -> Result<(VaultConfig, VaultHolding, StreamConfig, Timestamp), SpelError> {
        let (vault_config_state, vault_holding_state) =
            parse_vault_accounts(vault_config, vault_holding)?;

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

        Ok((
            vault_config_state,
            vault_holding_state,
            stream_config_state,
            now,
        ))
    }

    /// Load and validate vault, stream, and clock for instructions where the owner account is
    /// present for identity binding but authorization comes from a different account
    /// (`close_stream_by_provider`, `claim`).
    /// `owner_account_id` is still checked against `VaultConfig.owner` as defense in depth
    /// alongside the PDA binding.
    fn load_stream_context_with_owner_binding(
        vault_config: &AccountWithMetadata,
        vault_holding: &AccountWithMetadata,
        stream_config: &AccountWithMetadata,
        clock_account: &AccountWithMetadata,
        vault_id: VaultId,
        stream_id: StreamId,
        owner_account_id: AccountId,
    ) -> Result<(VaultConfig, VaultHolding, StreamConfig, Timestamp), SpelError> {
        let (vault_config_state, vault_holding_state) =
            parse_vault_accounts(vault_config, vault_holding)?;

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

        Ok((
            vault_config_state,
            vault_holding_state,
            stream_config_state,
            now,
        ))
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

    /// Shared account order for instructions with owner binding and separate provider authorization:
    /// `[vault_config, vault_holding, stream_config, owner, provider, clock_account]`.
    fn execute_stream_instruction_with_explicit_owner(
        vault_config_account: Account,
        vault_holding_account: Account,
        stream_account: Account,
        owner_account: Account,
        provider_account: Account,
        clock_account: Account,
    ) -> SpelOutput {
        SpelOutput::execute(
            vec![
                vault_config_account,
                vault_holding_account,
                stream_account,
                owner_account,
                provider_account,
                clock_account,
            ],
            vec![],
        )
    }

    /// Shared close accounting: fold/close stream, release unaccrued into vault total_allocated.
    /// Callers write both account datas and choose the path-specific SpelOutput builder.
    fn apply_close_accounting(
        vault_config_state: &mut VaultConfig,
        stream_config_state: StreamConfig,
        now: Timestamp,
    ) -> Result<StreamConfig, SpelError> {
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

        Ok(stream_after_close)
    }

    fn serialize_transfer_amount(amount: Balance) -> Result<Vec<u32>, SpelError> {
        let instruction = AuthenticatedTransferInstruction::Transfer { amount };
        risc0_zkvm::serde::to_vec(&instruction).map_err(|_| SpelError::SerializationError {
            message: "failed to serialize authenticated_transfer instruction".into(),
        })
    }

    fn write_account_data(account: &mut Account, state: &impl borsh::BorshSerialize) {
        let data =
            borsh::to_vec(state).expect("borsh serialization of a known account state cannot fail");
        account.data = data
            .try_into()
            .expect("serialized account state fits in the account data buffer");
    }

    // ---- Vault instructions ---- //

    #[instruction]
    pub fn initialize_vault(
        #[account(init, pda = [literal("vault_config"), account("owner"), arg("vault_id")])]
        vault_config: AccountWithMetadata,
        // LIP-155 holding seed 3 is the 32-byte token_id. Native is all-zeroes.
        #[account(init, pda = [literal("vault_holding"), account("vault_config"), arg("token_id")])]
        vault_holding: AccountWithMetadata,
        #[account(signer)] owner: AccountWithMetadata,
        vault_id: VaultId,
        privacy_tier: VaultPrivacyTier,
        token_id: [u8; 32],
    ) -> SpelResult {
        require_native_token_id(&token_id)
            .map_err(|_| spel_err(ErrorCode::UnsupportedTokenId, "unsupported token_id"))?;
        let mut vault_config_state = VaultConfig::new(
            owner.account_id,
            vault_id,
            None::<VersionId>,
            Some(privacy_tier),
        );
        vault_config_state.token_id = token_id;
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
        // const("") is seed_from_str empty = LIP-155 native token_id (32 zeroes).
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), const("")])]
        vault_holding: AccountWithMetadata,
        #[account(mut, signer)] owner: AccountWithMetadata,
        vault_id: VaultId,
        amount: Balance,
        authenticated_transfer_program_id: ProgramId,
    ) -> SpelResult {
        if amount == 0 {
            return Err(spel_err(
                ErrorCode::ZeroDepositAmount,
                "zero deposit amount",
            ));
        }

        let (vault_config_state, vault_holding_state) =
            parse_vault_accounts(&vault_config, &vault_holding)?;

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
            pre_states: vec![owner.clone(), vault_holding.clone()],
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
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), const("")])]
        vault_holding: AccountWithMetadata,
        #[account(mut, signer)] owner: AccountWithMetadata,
        #[account(mut)] withdraw_to: AccountWithMetadata,
        vault_id: VaultId,
        amount: Balance,
    ) -> SpelResult {
        if amount == 0 {
            return Err(spel_err(
                ErrorCode::ZeroWithdrawAmount,
                "zero withdraw amount",
            ));
        }

        let (vault_config_state, vault_holding_state) =
            parse_vault_accounts(&vault_config, &vault_holding)?;

        validate_vault_structure(&vault_config_state, &vault_holding_state, vault_id)?;
        validate_vault_owner(&vault_config_state, owner.account_id)?;

        let unallocated = vault_holding
            .account
            .balance
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
            .ok_or_else(|| {
                spel_err(
                    ErrorCode::InsufficientFunds,
                    "vault holding balance underflow",
                )
            })?;

        withdraw_to.account.balance = withdraw_to
            .account
            .balance
            .checked_add(amount)
            .ok_or_else(|| spel_err(ErrorCode::ArithmeticOverflow, "recipient balance overflow"))?;

        // The PP circuit requires that any account modified during execution
        // carries an ownership claim if it was default-owned (Account::default()) in pre-state.
        // A default-owned recipient is a new private commitment;
        // claiming it here lets the circuit set `program_owner` correctly
        // before its "modified but not claimed" invariant check.
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
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), const("")])]
        vault_holding: AccountWithMetadata,
        #[account(init, pda = [literal("stream_config"), account("vault_config"), arg("stream_id")])]
        stream_config: AccountWithMetadata,
        #[account(signer)] owner: AccountWithMetadata,
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
            return Err(spel_err(
                ErrorCode::ZeroStreamAllocation,
                "zero stream allocation",
            ));
        }

        let (mut vault_config_state, vault_holding_state) =
            parse_vault_accounts(&vault_config, &vault_holding)?;

        validate_vault_structure(&vault_config_state, &vault_holding_state, vault_id)?;
        validate_vault_owner(&vault_config_state, owner.account_id)?;

        if provider == owner.account_id {
            return Err(spel_err(
                ErrorCode::ProviderEqualsOwner,
                "provider must not equal vault owner",
            ));
        }

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
            vec![
                vault_config,
                vault_holding,
                stream_config,
                owner,
                clock_account,
            ],
            vec![],
        ))
    }

    #[instruction]
    pub fn pause_stream(
        #[account(mut, pda = [literal("vault_config"), account("owner"), arg("vault_id")])]
        vault_config: AccountWithMetadata,
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), const("")])]
        vault_holding: AccountWithMetadata,
        #[account(mut, pda = [literal("stream_config"), account("vault_config"), arg("stream_id")])]
        stream_config: AccountWithMetadata,
        #[account(signer)] owner: AccountWithMetadata,
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
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), const("")])]
        vault_holding: AccountWithMetadata,
        #[account(mut, pda = [literal("stream_config"), account("vault_config"), arg("stream_id")])]
        stream_config: AccountWithMetadata,
        #[account(signer)] owner: AccountWithMetadata,
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
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), const("")])]
        vault_holding: AccountWithMetadata,
        #[account(mut, pda = [literal("stream_config"), account("vault_config"), arg("stream_id")])]
        stream_config: AccountWithMetadata,
        #[account(signer)] owner: AccountWithMetadata,
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
    pub fn close_stream_by_owner(
        #[account(mut, pda = [literal("vault_config"), account("owner"), arg("vault_id")])]
        vault_config: AccountWithMetadata,
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), const("")])]
        vault_holding: AccountWithMetadata,
        #[account(mut, pda = [literal("stream_config"), account("vault_config"), arg("stream_id")])]
        stream_config: AccountWithMetadata,
        #[account(signer)] owner: AccountWithMetadata,
        clock_account: AccountWithMetadata,
        vault_id: VaultId,
        stream_id: StreamId,
    ) -> SpelResult {
        let (mut vault_config_state, _, stream_config_state, now) = load_owner_stream_context(
            &vault_config,
            &vault_holding,
            &stream_config,
            &clock_account,
            vault_id,
            stream_id,
            owner.account_id,
        )?;

        let stream_after_close =
            apply_close_accounting(&mut vault_config_state, stream_config_state, now)?;

        let mut vault_config_account = vault_config.account;
        write_account_data(&mut vault_config_account, &vault_config_state);

        let mut stream_account = stream_config.account;
        write_account_data(&mut stream_account, &stream_after_close);

        Ok(execute_owner_stream_instruction(
            vault_config_account,
            vault_holding.account,
            stream_account,
            owner.account,
            clock_account.account,
        ))
    }

    #[instruction]
    pub fn claim(
        #[account(mut, pda = [literal("vault_config"), account("owner"), arg("vault_id")])]
        vault_config: AccountWithMetadata,
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), const("")])]
        vault_holding: AccountWithMetadata,
        #[account(mut, pda = [literal("stream_config"), account("vault_config"), arg("stream_id")])]
        stream_config: AccountWithMetadata,
        // `owner` is present for identity binding.
        // Authorization comes from the provider account.
        // The owner id is verified against `VaultConfig.owner` for defense in depth alongside
        // the PDA seed binding (same pattern as `close_stream_by_provider`).
        #[account(mut)] owner: AccountWithMetadata,
        #[account(mut, signer)] provider: AccountWithMetadata,
        clock_account: AccountWithMetadata,
        vault_id: VaultId,
        stream_id: StreamId,
    ) -> SpelResult {
        let (mut vault_config_state, _, stream_config_state, now) =
            load_stream_context_with_owner_binding(
                &vault_config,
                &vault_holding,
                &stream_config,
                &clock_account,
                vault_id,
                stream_id,
                owner.account_id,
            )?;

        if provider.account_id != stream_config_state.provider {
            return Err(spel_err(
                ErrorCode::ClaimUnauthorized,
                "not stream provider",
            ));
        }

        let (payout, stream_after_claim) = stream_config_state
            .claim_at_time(now)
            .map_err(|e| spel_err(e, "claim_at_time failed"))?;

        // `claim_at_time` reduces stream allocation by exactly `payout`, so the vault-side
        // `total_allocated` must release the same amount to preserve allocation conservation.
        vault_config_state.total_allocated =
            checked_total_allocated_after_release(vault_config_state.total_allocated, payout)
                .map_err(|e| spel_err(e, "total_allocated release failed"))?;

        let mut vault_config = vault_config;
        let mut vault_holding = vault_holding;
        let mut stream_config = stream_config;
        let mut provider = provider;

        vault_holding.account.balance = vault_holding
            .account
            .balance
            .checked_sub(payout)
            .ok_or_else(|| {
                spel_err(
                    ErrorCode::InsufficientFunds,
                    "vault holding balance underflow",
                )
            })?;

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

    #[instruction]
    pub fn close_stream_by_provider(
        #[account(mut, pda = [literal("vault_config"), account("owner"), arg("vault_id")])]
        vault_config: AccountWithMetadata,
        #[account(mut, pda = [literal("vault_holding"), account("vault_config"), const("")])]
        vault_holding: AccountWithMetadata,
        #[account(mut, pda = [literal("stream_config"), account("vault_config"), arg("stream_id")])]
        stream_config: AccountWithMetadata,
        // `owner` is present for identity binding.
        // Authorization comes from the stream provider.
        // The owner id is verified against `VaultConfig.owner` in `load_stream_context_with_owner_binding`
        // as defense in depth alongside the PDA seed binding.
        #[account(mut)] owner: AccountWithMetadata,
        #[account(signer)] provider: AccountWithMetadata,
        clock_account: AccountWithMetadata,
        vault_id: VaultId,
        stream_id: StreamId,
    ) -> SpelResult {
        let (mut vault_config_state, _, stream_config_state, now) =
            load_stream_context_with_owner_binding(
                &vault_config,
                &vault_holding,
                &stream_config,
                &clock_account,
                vault_id,
                stream_id,
                owner.account_id,
            )?;

        if provider.account_id != stream_config_state.provider {
            return Err(spel_err(
                ErrorCode::CloseUnauthorized,
                "not stream provider",
            ));
        }

        let stream_after_close =
            apply_close_accounting(&mut vault_config_state, stream_config_state, now)?;

        let mut vault_config = vault_config;
        let mut stream_config = stream_config;

        write_account_data(&mut vault_config.account, &vault_config_state);
        write_account_data(&mut stream_config.account, &stream_after_close);

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
