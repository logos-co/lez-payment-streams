//! [`StreamProviderPolicy`], [`StreamParams`], and [`PolicyRejectReason`].
//!
//! These types mirror LIP-155 payment streams (see `rfc-index/docs/ift-ts/raw/payment-streams.md`).
//! Step 3a keeps them free of protobuf and wire parsing; higher layers supply parsed values.

use nssa_core::account::AccountId;

use crate::{Timestamp, TokensPerSecond};

/// Native token amount scale (matches on-chain stream `allocation` and vault accounting).
pub use nssa_core::account::Balance;

/// Max length for [`StreamParams::service_id`] bytes (LIP-155 LEZ integration).
///
/// Core does not reject longer `Vec`s here; callers (module / Step 4) should enforce before signing.
pub const MAX_SERVICE_ID_LEN: usize = 128;

/// Rules a provider advertises and that clients and on-chain terms must satisfy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamProviderPolicy {
    /// Minimum accepted rate (`TokensPerSecond` scale).
    pub min_rate: TokensPerSecond,
    /// Minimum accepted allocation (`Balance` scale).
    pub min_allocation: Balance,
    /// Upper bound on how far in the future `create_stream_deadline` may be
    /// relative to the LEZ clock-account timestamp `now` at proposal verification.
    pub max_create_stream_deadline_delay: Timestamp,
    /// Hard cap on outbound vault-proof response payload size enforced by demo providers.
    pub vault_proof_max_response_bytes: u64,
}

impl StreamProviderPolicy {
    /// Convenience constructor used in tests (not an on-chain discriminator).
    #[must_use]
    pub const fn new(
        min_rate: TokensPerSecond,
        min_allocation: Balance,
        max_create_stream_deadline_delay: Timestamp,
        vault_proof_max_response_bytes: u64,
    ) -> Self {
        Self {
            min_rate,
            min_allocation,
            max_create_stream_deadline_delay,
            vault_proof_max_response_bytes,
        }
    }
}

/// Off-chain proposal / session terms (`StreamProposal` semantics), already decoded upstream.
///
/// `service_id` is carried for module-level checks against `/vac/waku/store-query/3.0.0` —
/// predicates in this crate deliberately do NOT inspect it (`stream_satisfies_policy` excludes it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamParams {
    /// Proposed accrual rate; same scale as on-chain [`crate::StreamConfig::rate`].
    pub rate: TokensPerSecond,
    /// Proposed allocation cap; same scale as on-chain [`crate::StreamConfig::allocation`].
    pub allocation: Balance,
    /// Latest ledger time by which the payer must land `create_stream` on-chain (signed proposal field).
    pub create_stream_deadline: Timestamp,
    /// Opaque service identifier; length SHOULD be at most [`MAX_SERVICE_ID_LEN`].
    pub service_id: Vec<u8>,
}

impl StreamParams {
    #[must_use]
    pub fn new(
        rate: TokensPerSecond,
        allocation: Balance,
        create_stream_deadline: Timestamp,
        service_id: Vec<u8>,
    ) -> Self {
        Self {
            rate,
            allocation,
            create_stream_deadline,
            service_id,
        }
    }
}

/// Session view pinned at acceptance: proposal params, payee identity, and provider policy snapshot.
///
/// LIP-155 requires ongoing verification to use `policy_at_acceptance`, not the latest advertised policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptedStreamTerms {
    pub params: StreamParams,
    pub provider_id: AccountId,
    pub policy_at_acceptance: StreamProviderPolicy,
}

/// Inputs required to validate an off-chain `StreamProposal` against a pinned provider policy row.
///
/// Field order matches [`crate::policy::proposal_satisfies_policy`]: negotiated [`StreamParams`] (proposal)
/// first, then advertised [`StreamProviderPolicy`], then vault snapshot and evaluation clock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalCheckInputs<'a> {
    pub params: &'a StreamParams,
    pub policy: &'a StreamProviderPolicy,
    pub vault_holding_balance: Balance,
    pub vault_total_allocated: Balance,
    /// LEZ clock-account timestamp when this proposal is evaluated (same domain as [`crate::StreamConfig::at_time`]).
    /// This is not the on-chain stream start time; that exists only after `create_stream` (see [`crate::StreamConfig::accrued_as_of`]).
    pub now: Timestamp,
}

impl<'a> ProposalCheckInputs<'a> {
    #[must_use]
    pub const fn new(
        params: &'a StreamParams,
        policy: &'a StreamProviderPolicy,
        vault_holding_balance: Balance,
        vault_total_allocated: Balance,
        now: Timestamp,
    ) -> Self {
        Self {
            params,
            policy,
            vault_holding_balance,
            vault_total_allocated,
            now,
        }
    }
}

/// Reasons a pure-policy check can reject before signature / protobuf failures (handled in Step 4).
///
/// FFI Step 3b maps these variants to eligibility codes such as `PARAMS_REJECTED` and
/// `STREAM_NOT_ACTIVE`. Discriminants are stable for `#[repr(u32)]` ABI; extend only by appending.
#[non_exhaustive]
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyRejectReason {
    /// Proposed rate is below the provider minimum.
    RateBelowPolicyMin = 0,
    /// Proposed allocation is below the provider minimum.
    AllocationBelowPolicyMin = 1,
    /// [`StreamParams::create_stream_deadline`] is invalid for the proposal-clock `now`:
    /// not strictly in the future, or later than `now + StreamProviderPolicy::max_create_stream_deadline_delay`.
    CreateStreamDeadlineInvalid = 2,
    /// `vault_total_allocated +` [`StreamParams::allocation`] would exceed holdings (unallocated holding balance).
    UnallocatedInsufficient = 3,
    /// Established on-chain rate is weaker than accepted proposal terms.
    RateBelowAcceptedParams = 4,
    /// Established on-chain allocation is weaker than accepted proposal terms.
    AllocationBelowAcceptedParams = 5,
    /// Folded [`crate::StreamConfig::provider`] or proof payee mismatch against the acceptance binding.
    ProviderMismatch = 6,
    /// Only [`crate::StreamState::Active`] streams may serve proofs.
    StreamNotActive = 7,
    /// Outbound vault-proof response exceeds `vault_proof_max_response_bytes`.
    ResponseTooLarge = 8,
}
