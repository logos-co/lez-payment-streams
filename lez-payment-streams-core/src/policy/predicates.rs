//! Predicate implementations backing [`super`].

use nssa_core::account::AccountId;

use crate::error_codes::ErrorCode;
use crate::stream_config::{StreamConfig, StreamState};
use crate::stream_provider_policy::{
    AcceptedStreamTerms, Balance, PolicyRejectReason, ProposalCheckInputs, StreamParams,
    StreamProviderPolicy,
};
use crate::Timestamp;

/// Stream state after folding lazy accrual through [`fold_stream`] (one [`StreamConfig::at_time`]).
///
/// `as_of` is the ledger / clock timestamp passed into folding (historic, preflight, or head).
/// It may differ from [`StreamConfig::accrued_as_of`] after mid-interval depletion.
///
/// `accrued` and `unaccrued` use LEZ `Balance` (`u128`); the C FFI represents wide amounts as
/// little-endian `lo` / `hi` `u64` halves (same pattern as decoded vault/stream configs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamFoldedAtTime {
    pub stream_config: StreamConfig,
    /// Accrued amount at `as_of` (LEZ `Balance` / `u128`).
    pub accrued: Balance,
    /// Remaining principal cap not yet accrued at `as_of` (same width as `accrued`).
    pub unaccrued: Balance,
    pub as_of: Timestamp,
}

/// Apply [`StreamConfig::at_time`] and surface accrued / unaccrued balances for tooling + tests.
///
/// Downstream FFI can forward the struct fields verbatim without redoing arithmetic.
pub fn fold_stream(
    stream: &StreamConfig,
    as_of: Timestamp,
) -> Result<StreamFoldedAtTime, ErrorCode> {
    let stream_config = stream.at_time(as_of)?;
    let accrued = stream_config.accrued;
    let unaccrued = stream_config.unaccrued();

    Ok(StreamFoldedAtTime {
        stream_config,
        accrued,
        unaccrued,
        as_of,
    })
}

/// Portion of the vault holding balance not yet committed in `total_allocated`
/// (`holding_balance - total_allocated`, saturating at zero).
#[must_use]
pub fn unallocated_balance(
    vault_holding_balance: Balance,
    vault_total_allocated: Balance,
) -> Balance {
    vault_holding_balance.saturating_sub(vault_total_allocated)
}

/// LIP-155 deadline band: `params_create_stream_deadline` must lie in
/// `(check_time, check_time + policy_max_create_stream_deadline_delay]`.
/// `check_time` matches [`ProposalCheckInputs::now`] (not a signed proposal field).
///
/// When `check_time + policy_max_create_stream_deadline_delay` overflows `u64`, the inclusive upper
/// bound is pinned to [`Timestamp::MAX`]. Any finite deadline is then below that ceiling, so the
/// check stays well-defined without wrapping. For example, `checked_add(delay)` overflowing still
/// leaves `Timestamp::MAX` as the upper bound rather than wrapping a smaller timestamp.
pub fn create_stream_deadline_satisfies_policy_as_of(
    params_create_stream_deadline: Timestamp,
    policy_max_create_stream_deadline_delay: Timestamp,
    check_time: Timestamp,
) -> Result<(), PolicyRejectReason> {
    if check_time >= params_create_stream_deadline {
        return Err(PolicyRejectReason::CreateStreamDeadlineInvalid);
    }

    // Overflow pins to MAX: if the delay sum does not fit in `Timestamp`, the admissible window
    // upper bound is unbounded for all practical deadlines, so treat the ceiling as infinite.
    let max_allowed_create_stream_deadline =
        match check_time.checked_add(policy_max_create_stream_deadline_delay) {
            Some(upper) => upper,
            None => Timestamp::MAX,
        };
    if params_create_stream_deadline > max_allowed_create_stream_deadline {
        return Err(PolicyRejectReason::CreateStreamDeadlineInvalid);
    }
    Ok(())
}

/// Proposal-time policy check executed on both peers (preflight symmetry).
///
/// Validates minima, bounded `create_stream_deadline`, and vault solvency for the proposed allocation.
///
/// Unallocated-balance invariant (see also [`crate::checked_total_allocated_after_add`]):
///
/// holding balance -
/// vault total_allocated >= proposed [`crate::StreamParams::allocation`]
pub fn proposal_satisfies_policy(
    ProposalCheckInputs {
        params,
        policy,
        vault_holding_balance,
        vault_total_allocated,
        now,
    }: &ProposalCheckInputs<'_>,
) -> Result<(), PolicyRejectReason> {
    if params.rate < policy.min_rate {
        return Err(PolicyRejectReason::RateBelowPolicyMin);
    }
    if params.allocation < policy.min_allocation {
        return Err(PolicyRejectReason::AllocationBelowPolicyMin);
    }

    create_stream_deadline_satisfies_policy_as_of(
        params.create_stream_deadline,
        policy.max_create_stream_deadline_delay,
        *now,
    )?;

    let unallocated = unallocated_balance(*vault_holding_balance, *vault_total_allocated);
    if params.allocation > unallocated {
        return Err(PolicyRejectReason::UnallocatedInsufficient);
    }
    Ok(())
}

/// First `StreamProof` in a session: reconcile folded on-chain snapshot against accepted proposal.
///
/// Establishes parity between `StreamParams` negotiated off-chain (Step 4 will sign them)
/// and the payer's eventual on-chain [`StreamConfig`]:
///
/// - Compare persisted `allocation` and `rate` fields, not instantaneous unaccrued balance.
/// - Bind `provider` pubkey bytes to LEZ `/ VaultProof.provider_id`.
pub fn new_stream_satisfies_proposal(
    folded_stream: &StreamConfig,
    proposal_params: &StreamParams,
    proposal_provider_id: AccountId,
) -> Result<(), PolicyRejectReason> {
    stream_provider_binding_satisfies_expected_payee(folded_stream, proposal_provider_id)?;

    if folded_stream.rate < proposal_params.rate {
        return Err(PolicyRejectReason::RateBelowAcceptedParams);
    }

    // Compare stored principal cap, ignoring accrual drawdown tracked in `unaccrued()`.
    if folded_stream.allocation < proposal_params.allocation {
        return Err(PolicyRejectReason::AllocationBelowAcceptedParams);
    }
    Ok(())
}

/// Subsequent / ongoing proof validation using the policy snapshot taken at acceptance.
///
/// Keeps rejects aligned between payer preflight and provider verification.
///
/// - Stream must remain [`StreamState::Active`].
/// - Payee pubkey must remain the accepted provider binding.
/// - On-chain [`StreamConfig::rate`] must be greater than or equal to the accepted proposal rate
///   and remain at or above pinned policy minima. The payment-streams spec requires this on every
///   `StreamProof`, not only when reconciling the first proof with [`new_stream_satisfies_proposal`],
///   so the provider rejects service if folded on-chain state violates those floors. This LEZ guest
///   sets `rate` only at `create_stream`; it does not later lower `rate` (for example
///   [`Instruction::TopUpStream`] only increases allocation).
///
/// `service_id` is intentionally untouched here (module compares against `/vac/waku/store-query/3.0.0`).
pub fn stream_satisfies_policy(
    folded_stream: &StreamConfig,
    accepted_terms: &AcceptedStreamTerms,
) -> Result<(), PolicyRejectReason> {
    if folded_stream.state != StreamState::Active {
        return Err(PolicyRejectReason::StreamNotActive);
    }

    stream_provider_binding_satisfies_expected_payee(folded_stream, accepted_terms.provider_id)?;

    if folded_stream.rate < accepted_terms.policy_at_acceptance.min_rate {
        return Err(PolicyRejectReason::RateBelowPolicyMin);
    }
    if folded_stream.rate < accepted_terms.params.rate {
        return Err(PolicyRejectReason::RateBelowAcceptedParams);
    }
    Ok(())
}

/// Prevent oversized vault proofs on the outbound path (demo MUST enforce once per session).
pub fn response_within_policy(
    response_payload_byte_len: u64,
    policy: &StreamProviderPolicy,
) -> Result<(), PolicyRejectReason> {
    if response_payload_byte_len > policy.vault_proof_max_response_bytes {
        return Err(PolicyRejectReason::ResponseTooLarge);
    }
    Ok(())
}

/// Returns [`Ok(())`] when [`StreamConfig::provider`] equals the expected vault payee
/// [`AccountId`] (vault proof binds this binding).
#[inline]
fn stream_provider_binding_satisfies_expected_payee(
    stream_account: &StreamConfig,
    expected_provider_id: AccountId,
) -> Result<(), PolicyRejectReason> {
    if stream_account.provider != expected_provider_id {
        return Err(PolicyRejectReason::ProviderMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod predicates_unit_tests {
    //! Documented vectors called out under `step3a-implementor-notes.md` (reuse verbatim from Step 3b).

    use nssa_core::account::AccountId;

    use super::{
        create_stream_deadline_satisfies_policy_as_of, fold_stream, new_stream_satisfies_proposal,
        proposal_satisfies_policy, response_within_policy, stream_satisfies_policy,
        unallocated_balance, ProposalCheckInputs, StreamFoldedAtTime,
    };
    use crate::error_codes::ErrorCode;
    use crate::stream_provider_policy::{
        AcceptedStreamTerms, Balance, PolicyRejectReason, StreamParams, StreamProviderPolicy,
    };
    use crate::{StreamConfig, StreamId, StreamState, Timestamp, TokensPerSecond, DEFAULT_VERSION};

    fn account_marker(byte: u8) -> AccountId {
        AccountId::new([byte; 32])
    }

    /// Shared fixture mirroring [`crate::stream_config::stream_test_fixtures`] but with visible provider IDs.
    fn stream_fixture(
        accrued: Balance,
        allocation: Balance,
        rate: TokensPerSecond,
        accrued_as_of: Timestamp,
        state: StreamState,
        provider: AccountId,
    ) -> StreamConfig {
        StreamConfig {
            version: DEFAULT_VERSION,
            stream_id: StreamId::MIN,
            provider,
            rate,
            allocation,
            accrued,
            state,
            accrued_as_of,
        }
    }

    #[test]
    fn unallocated_balance_matches_saturating_sub() {
        assert_eq!(
            unallocated_balance(600, 400),
            200,
            "vault holding minus total_allocated uses saturating subtraction semantics"
        );
    }

    #[test]
    fn unallocated_balance_saturates_at_zero_when_overallocated() {
        assert_eq!(unallocated_balance(100, 200), 0);
    }

    #[test]
    fn fold_stream_active_accrual_follows_lazy_linear_model() {
        let provider = account_marker(9);
        let stream = stream_fixture(0, 1_000, 10, 100, StreamState::Active, provider);

        let folded = fold_stream(&stream, 105).expect("linear accrual fold");
        assert_eq!(folded.accrued, 50);
        assert_eq!(folded.unaccrued, 950);
        assert_eq!(folded.stream_config.accrued_as_of, 105);
        assert_eq!(folded.stream_config.state, StreamState::Active);
        assert_eq!(folded.as_of, 105);
    }

    #[test]
    fn fold_stream_depletion_transitions_into_paused_state() {
        let provider = account_marker(2);
        let stream = stream_fixture(0, 100, 10, 0, StreamState::Active, provider);

        let folded = fold_stream(&stream, 50).expect("depletion folds must succeed");
        assert_eq!(folded.accrued, 100);
        assert_eq!(folded.unaccrued, 0);
        assert_eq!(folded.stream_config.state, StreamState::Paused);
        assert_eq!(folded.stream_config.accrued_as_of, 10);
    }

    #[test]
    fn fold_stream_paused_preserves_prior_accrual_state() {
        let provider = account_marker(7);
        let mut stream = stream_fixture(100, 1_000, 10, 100, StreamState::Active, provider);
        stream.state = StreamState::Paused;

        let folded = fold_stream(&stream, 200).expect("paused streams short-circuit accrual");
        assert_eq!(
            folded,
            StreamFoldedAtTime {
                stream_config: stream.clone(),
                accrued: 100,
                unaccrued: stream.unaccrued(),
                as_of: 200,
            },
        );
    }

    #[test]
    fn fold_stream_surfaces_time_regression_from_stream_config_at_time() {
        let provider = account_marker(8);
        let stream = stream_fixture(
            0,
            1_000,
            10,
            /* accrued_as_of */ 100,
            StreamState::Active,
            provider,
        );

        assert_eq!(
            fold_stream(&stream, 99),
            Err(ErrorCode::TimeRegression),
            "fold_stream must not mask clock regression errors from folding"
        );
    }

    #[test]
    fn proposal_rejects_when_rate_or_allocation_below_advertised_floors() {
        let policy = StreamProviderPolicy::new(
            /* min_rate */ 20, /* min_allocation */ 500,
            /* max_deadline_delay */ 1_000, /* vault response cap */ 65_536,
        );

        assert_eq!(
            proposal_satisfies_policy(&ProposalCheckInputs {
                params: &StreamParams::new(10, 600, 200, vec![]),
                policy: &policy,
                vault_holding_balance: 10_000,
                vault_total_allocated: 100,
                now: 100,
            }),
            Err(PolicyRejectReason::RateBelowPolicyMin)
        );

        assert_eq!(
            proposal_satisfies_policy(&ProposalCheckInputs {
                params: &StreamParams::new(30, 400, 400, vec![]),
                policy: &policy,
                vault_holding_balance: 10_000,
                vault_total_allocated: 100,
                now: 350,
            }),
            Err(PolicyRejectReason::AllocationBelowPolicyMin)
        );
    }

    #[test]
    fn proposal_deadline_requires_strict_future_and_band_limited_horizon() {
        let policy = StreamProviderPolicy::new(1, 1, /* max_deadline_delay */ 10, 65_536);

        proposal_satisfies_policy(&ProposalCheckInputs {
            params: &StreamParams::new(1, 1, /* deadline exactly +10s */ 110, vec![]),
            policy: &policy,
            vault_holding_balance: 1_000,
            vault_total_allocated: 0,
            now: 100,
        })
        .expect("Δ = 10 is inclusive at the provider window");

        assert!(proposal_satisfies_policy(&ProposalCheckInputs {
            params: &StreamParams::new(1, 1, /* not strictly ahead of now */ 100, vec![]),
            policy: &policy,
            vault_holding_balance: 1_000,
            vault_total_allocated: 0,
            now: 100,
        })
        .is_err());

        assert_eq!(
            proposal_satisfies_policy(&ProposalCheckInputs {
                params: &StreamParams::new(1, 1, /* exceeds +10s horizon */ 200, vec![]),
                policy: &policy,
                vault_holding_balance: 1_000,
                vault_total_allocated: 0,
                now: 100,
            }),
            Err(PolicyRejectReason::CreateStreamDeadlineInvalid)
        );
    }

    #[test]
    fn proposal_deadline_overflow_pins_upper_bound_to_max() {
        let policy = StreamProviderPolicy::new(1, 1, Timestamp::MAX, 65_536);

        create_stream_deadline_satisfies_policy_as_of(Timestamp::MAX - 1, Timestamp::MAX, 100)
            .expect("overflow handling should pin upper bound to MAX");

        proposal_satisfies_policy(&ProposalCheckInputs {
            params: &StreamParams::new(1, 1, Timestamp::MAX - 1, vec![]),
            policy: &policy,
            vault_holding_balance: 1_000,
            vault_total_allocated: 0,
            now: 100,
        })
        .expect("finite deadline within overflow-pinned window");
    }

    #[test]
    fn proposal_checks_unallocated_balance_after_policy_minima() {
        let policy = StreamProviderPolicy::new(1, 1, 1_000, 65_536);

        assert_eq!(
            proposal_satisfies_policy(&ProposalCheckInputs {
                params: &StreamParams::new(5, /* alloc */ 250, /* deadline within window */ 100, vec![]),
                policy: &policy,
                vault_holding_balance: 600,
                vault_total_allocated: /* leaves 189 unallocated */ 411,
                now: 50,
            }),
            Err(PolicyRejectReason::UnallocatedInsufficient)
        );

        proposal_satisfies_policy(&ProposalCheckInputs {
            params: &StreamParams::new(5, /* alloc respects solvency */ 189, 100, vec![]),
            policy: &policy,
            vault_holding_balance: 600,
            vault_total_allocated: 411,
            now: 50,
        })
        .expect("exact unallocated parity should authorize the proposal floor");
    }

    #[test]
    fn new_stream_accepts_matching_or_stricter_on_chain_terms() {
        let provider = account_marker(3);
        let accepted = StreamParams::new(
            /* rate floor */ 10,
            /* allocation floor */ 200,
            999,
            vec![],
        );
        let on_chain_exact = stream_fixture(0, 200, 10, 50, StreamState::Active, provider);

        assert!(new_stream_satisfies_proposal(&on_chain_exact, &accepted, provider).is_ok());

        let on_chain_stricter = stream_fixture(0, 250, 15, 50, StreamState::Active, provider);
        assert!(new_stream_satisfies_proposal(&on_chain_stricter, &accepted, provider).is_ok());
    }

    #[test]
    fn new_stream_rejects_weaker_than_accepted_snapshots_or_mismatched_provider() {
        let provider = account_marker(4);
        let impostor = account_marker(5);
        let accepted = StreamParams::new(50, 200, 0, vec![]);

        assert_eq!(
            new_stream_satisfies_proposal(
                &stream_fixture(0, 199, 50, 0, StreamState::Active, provider),
                &accepted,
                provider,
            ),
            Err(PolicyRejectReason::AllocationBelowAcceptedParams)
        );

        assert_eq!(
            new_stream_satisfies_proposal(
                &stream_fixture(
                    0,
                    200,
                    /* weaker rate */ 40,
                    0,
                    StreamState::Active,
                    provider
                ),
                &accepted,
                provider,
            ),
            Err(PolicyRejectReason::RateBelowAcceptedParams)
        );

        assert_eq!(
            new_stream_satisfies_proposal(
                &stream_fixture(0, 200, 50, 0, StreamState::Active, provider),
                &accepted,
                impostor,
            ),
            Err(PolicyRejectReason::ProviderMismatch)
        );

        let mut misbound = stream_fixture(0, 200, 50, 0, StreamState::Active, provider);
        misbound.provider = impostor;
        assert_eq!(
            new_stream_satisfies_proposal(&misbound, &accepted, provider),
            Err(PolicyRejectReason::ProviderMismatch)
        );
    }

    fn accepted_terms_fixture(
        params: StreamParams,
        policy: StreamProviderPolicy,
        provider: AccountId,
    ) -> AcceptedStreamTerms {
        AcceptedStreamTerms {
            params,
            provider_id: provider,
            policy_at_acceptance: policy,
        }
    }

    #[test]
    fn stream_policy_accepts_when_active_rates_meet_pins() {
        let provider = account_marker(11);
        let policy = StreamProviderPolicy::new(
            /* min_rate */ 10, /* min_allocation */ 1, 1_000, 65_536,
        );
        let params = StreamParams::new(12, 500, 0, vec![]);
        let terms = accepted_terms_fixture(params, policy, provider);
        let folded = stream_fixture(25, 500, 15, 200, StreamState::Active, provider);

        assert!(stream_satisfies_policy(&folded, &terms).is_ok());
    }

    #[test]
    fn stream_policy_rejects_paused_streams_even_if_rates_would_pass() {
        let provider = account_marker(12);
        let policy = StreamProviderPolicy::new(1, 1, 1_000, 65_536);
        let params = StreamParams::new(5, 100, 0, vec![]);
        let terms = accepted_terms_fixture(params, policy, provider);
        let folded = stream_fixture(100, 100, 10, 10, StreamState::Paused, provider);

        assert_eq!(
            stream_satisfies_policy(&folded, &terms),
            Err(PolicyRejectReason::StreamNotActive)
        );
    }

    #[test]
    fn stream_policy_rejects_closed_streams() {
        let provider = account_marker(14);
        let policy = StreamProviderPolicy::new(1, 1, 1_000, 65_536);
        let params = StreamParams::new(5, 100, 0, vec![]);
        let terms = accepted_terms_fixture(params, policy, provider);
        let folded = stream_fixture(10, 10, 5, 0, StreamState::Closed, provider);

        assert_eq!(
            stream_satisfies_policy(&folded, &terms),
            Err(PolicyRejectReason::StreamNotActive)
        );
    }

    #[test]
    fn stream_policy_compares_on_chain_rate_against_pinned_policy_and_accepted_floors() {
        let provider = account_marker(13);
        let policy = StreamProviderPolicy::new(20, 1, 1_000, 65_536);
        let params = StreamParams::new(30, 100, 0, vec![]);
        let terms = accepted_terms_fixture(params, policy, provider);
        let folded = stream_fixture(
            0,
            100,
            /* below policy min */ 15,
            0,
            StreamState::Active,
            provider,
        );

        assert_eq!(
            stream_satisfies_policy(&folded, &terms),
            Err(PolicyRejectReason::RateBelowPolicyMin)
        );

        let folded_ok_policy_bad_accepted = stream_fixture(
            0,
            100,
            /* meets policy but not accepted rate */ 25,
            0,
            StreamState::Active,
            provider,
        );
        assert_eq!(
            stream_satisfies_policy(&folded_ok_policy_bad_accepted, &terms),
            Err(PolicyRejectReason::RateBelowAcceptedParams)
        );
    }

    #[test]
    fn stream_policy_rejects_mismatched_provider() {
        let bound = account_marker(20);
        let wrong = account_marker(21);
        let policy = StreamProviderPolicy::new(1, 1, 1_000, 65_536);
        let params = StreamParams::new(10, 100, 0, vec![]);
        let terms = accepted_terms_fixture(params, policy, bound);
        let folded = stream_fixture(0, 100, 10, 0, StreamState::Active, wrong);

        assert_eq!(
            stream_satisfies_policy(&folded, &terms),
            Err(PolicyRejectReason::ProviderMismatch)
        );
    }

    #[test]
    fn response_cap_allows_exact_limit_and_rejects_overages() {
        let policy = StreamProviderPolicy::new(1, 1, 1, /* cap */ 128);

        assert!(response_within_policy(128, &policy).is_ok());
        assert_eq!(
            response_within_policy(129, &policy),
            Err(PolicyRejectReason::ResponseTooLarge)
        );
    }
}
