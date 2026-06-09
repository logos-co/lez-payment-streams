//! LEZ payment streams.
//!
//! Vault PDAs, stream accrual, [`Instruction`] types for the guest and tests, the [`policy`]
//! module (Step 3a: `fold_stream`, `proposal_satisfies_policy`, `stream_satisfies_policy`, ...),
//! plus public-transaction helpers ([`instruction_wire`], [`instruction_accounts`]; Step 5).

#[cfg(test)]
mod harness_seeds;

#[cfg(test)]
mod test_helpers;

#[cfg(test)]
mod program_tests;

mod error_codes;
mod instruction;
mod instruction_accounts;
#[cfg(feature = "host")]
mod instruction_wire;
#[cfg(feature = "host")]
mod off_chain;
mod pda;
mod policy;
mod stream_config;
mod stream_provider_policy;
mod vault;

pub use clock_core::{
    ClockAccountData, CLOCK_01_PROGRAM_ACCOUNT_ID, CLOCK_10_PROGRAM_ACCOUNT_ID,
    CLOCK_50_PROGRAM_ACCOUNT_ID, CLOCK_PROGRAM_ACCOUNT_IDS,
};
pub use error_codes::*;
pub use instruction::Instruction;
pub use instruction_accounts::{
    claim_instruction_accounts, close_stream_instruction_accounts,
    create_stream_instruction_accounts, deposit_instruction_accounts,
    initialize_vault_instruction_accounts, pause_stream_instruction_accounts,
    resume_stream_instruction_accounts, top_up_stream_instruction_accounts,
    withdraw_instruction_accounts, ClaimStreamInstructionAccounts, DepositInstructionAccounts,
    InitializeVaultInstructionAccounts, StreamAuthorityInstructionAccounts,
    StreamOwnerInstructionAccounts, WithdrawInstructionAccounts,
};
#[cfg(feature = "host")]
pub use instruction_wire::{
    instruction_bytes_for_public_transaction, instruction_bytes_le_from_words,
    instruction_try_from_instruction_words, instruction_words_for_public_transaction,
    instruction_words_from_bytes_le,
};
#[cfg(feature = "host")]
pub use off_chain::*;
pub use pda::{derive_stream_config_account_id, derive_vault_account_ids};
pub use policy::{
    create_stream_deadline_satisfies_policy_as_of, fold_stream, new_stream_satisfies_proposal,
    proposal_satisfies_policy, response_within_policy, stream_satisfies_policy,
    unallocated_balance, StreamFoldedAtTime,
};
pub use stream_config::{StreamConfig, StreamState};
pub use stream_provider_policy::{
    AcceptedStreamTerms, Balance, PolicyRejectReason, ProposalCheckInputs, StreamParams,
    StreamProviderPolicy, MAX_SERVICE_ID_LEN,
};
pub use vault::{
    checked_total_allocated_after_add, checked_total_allocated_after_release, VaultConfig,
    VaultHolding, VaultPrivacyTier,
};

// ---- Type aliases ---- //

pub type VersionId = u8;
pub type VaultId = u64;
pub type StreamId = u64;
pub type TokensPerSecond = u64;
pub type Timestamp = u64;

// ---- Version ---- //

pub const DEFAULT_VERSION: VersionId = 1;
