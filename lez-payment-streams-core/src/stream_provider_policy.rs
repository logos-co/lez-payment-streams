//! [`StreamProviderPolicy`], [`StreamParams`], and [`PolicyRejectReason`].
//!
//! These types mirror LIP-155 payment streams (see `rfc-index/docs/ift-ts/raw/payment-streams.md`).
//! Step 3a keeps them free of protobuf and wire parsing; higher layers supply parsed values.

use nssa_core::account::AccountId;

use crate::{Timestamp, TokensPerSecond};

/// Native token amount scale (matches on-chain stream `allocation` and vault accounting).
pub use nssa_core::account::Balance;

/// Rules a provider advertises and that clients and on-chain terms must satisfy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamProviderPolicy {
    /// Minimum accepted stream rate (`TokensPerSecond` scale).
    pub min_stream_rate: TokensPerSecond,
    /// Minimum accepted stream allocation (`Balance` scale).
    pub min_stream_allocation: Balance,
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
        min_stream_rate: TokensPerSecond,
        min_stream_allocation: Balance,
        max_create_stream_deadline_delay: Timestamp,
        vault_proof_max_response_bytes: u64,
    ) -> Self {
        Self {
            min_stream_rate,
            min_stream_allocation,
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
    pub stream_rate: TokensPerSecond,
    pub stream_allocation: Balance,
    /// Latest ledger time by which the payer must land `create_stream` on-chain (signed proposal field).
    pub create_stream_deadline: Timestamp,
    pub service_id: Vec<u8>,
}

impl StreamParams {
    #[must_use]
    pub fn new(
        stream_rate: TokensPerSecond,
        stream_allocation: Balance,
        create_stream_deadline: Timestamp,
        service_id: Vec<u8>,
    ) -> Self {
        Self {
            stream_rate,
            stream_allocation,
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

/// Reasons a pure-policy check can reject before signature / protobuf failures (handled in Step 4).
///
/// FFI Step 3b maps these variants to eligibility codes such as `PARAMS_REJECTED` and
/// `STREAM_NOT_ACTIVE`; keep discriminant churn small once `repr(C)` lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyRejectReason {
    /// Proposed rate is below the provider minimum.
    RateBelowPolicyMin,
    /// Proposed allocation is below the provider minimum.
    AllocationBelowPolicyMin,
    /// [`StreamParams::create_stream_deadline`] is invalid for the proposal-clock `now`:
    /// not strictly in the future, or later than `now + StreamProviderPolicy::max_create_stream_deadline_delay`.
    CreateStreamDeadlineInvalid,
    /// `vault_total_allocated + stream_allocation` would exceed holdings (unallocated holding balance).
    UnallocatedInsufficient,
    /// Established on-chain rate is weaker than accepted proposal terms.
    RateBelowAcceptedParams,
    /// Established on-chain allocation is weaker than accepted proposal terms.
    AllocationBelowAcceptedParams,
    /// Folded [`crate::StreamConfig::provider`] or proof payee mismatch against the acceptance binding.
    ProviderMismatch,
    /// Only [`crate::StreamState::Active`] streams may serve proofs.
    StreamNotActive,
    /// Outbound vault-proof response exceeds `vault_proof_max_response_bytes`.
    ResponseTooLarge,
}
