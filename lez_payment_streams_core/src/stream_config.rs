//! [`StreamState`], [`StreamConfig`], and lazy accrual math.

use borsh::{BorshDeserialize, BorshSerialize};
use nssa_core::account::{AccountId, Balance};

use crate::error_codes::ErrorCode;
use crate::{StreamId, Timestamp, TokensPerSecond, VersionId, DEFAULT_VERSION};

/// Stream lifecycle. One byte on the wire (ordinal).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[borsh(use_discriminant = true)]
pub enum StreamState {
    Active = 0,
    Paused = 1,
    Closed = 2,
}

/// Stream PDA account body.
/// Vault identity comes from the stream PDA seeds at derivation time, not from this struct.
#[spel_framework_macros::account_type]
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct StreamConfig {
    pub version: VersionId,
    /// Match the `stream_id` seed in the stream PDA derivation.
    pub stream_id: StreamId,
    pub provider: AccountId,
    pub rate: TokensPerSecond,
    pub allocation: Balance,
    pub accrued: Balance,
    pub state: StreamState,
    /// Latest chain time folded into `accrued`.
    /// When not depleted: equals `t` after each fold.
    /// When depleted: equals the depletion instant
    /// (`⌈unaccrued/rate⌉` seconds after the prior `accrued_as_of`),
    /// which may precede `t` if depletion occurred within the accrual interval.
    pub accrued_as_of: Timestamp,
}

impl StreamConfig {
    /// Use `None` for `version` to pick [`DEFAULT_VERSION`].
    pub fn new(
        stream_id: StreamId,
        provider: AccountId,
        rate: TokensPerSecond,
        allocation: Balance,
        accrued_as_of: Timestamp,
        version: Option<VersionId>,
    ) -> Self {
        Self {
            version: version.unwrap_or(DEFAULT_VERSION),
            stream_id,
            provider,
            rate,
            allocation,
            accrued: Balance::MIN,
            state: StreamState::Active,
            accrued_as_of,
        }
    }

    /// Remaining unaccrued funds: `allocation - accrued` floored at zero.
    pub fn unaccrued(&self) -> Balance {
        self.allocation.saturating_sub(self.accrued)
    }

    /// Fold lazy accrual to chain time `t`.
    ///
    /// For Paused or Closed, return the input unchanged.
    /// If `t` equals [`StreamConfig::accrued_as_of`], return a copy.
    ///
    /// For Active streams with `t` after [`StreamConfig::accrued_as_of`],
    /// add `rate * Δt` up to `allocation`.
    /// When [`StreamConfig::unaccrued`] reaches zero after that step,
    /// set state to [`StreamState::Paused`].
    ///
    /// Returns [`ErrorCode::TimeRegression`] when `t` precedes [`StreamConfig::accrued_as_of`].
    pub fn at_time(&self, t: Timestamp) -> Result<Self, ErrorCode> {
        self.validate_invariants()?;

        match self.state {
            StreamState::Paused | StreamState::Closed => return Ok(self.clone()),
            StreamState::Active => {}
        }

        if t < self.accrued_as_of {
            return Err(ErrorCode::TimeRegression);
        }

        if t == self.accrued_as_of {
            return Ok(self.clone());
        }

        let allocation = self.allocation;
        let base_as_of = self.accrued_as_of;
        let base_accrued = self.accrued;
        let rate = self.rate;

        // `rate * elapsed` is computed as u128 because for high rates and long intervals
        // the product can exceed u64::MAX even though neither operand does.
        // Widening stays local here; stored fields remain u64/u128 as defined.
        let elapsed_seconds = t - base_as_of;
        let accrued_over_elapsed_seconds =
            u128::from(rate).saturating_mul(u128::from(elapsed_seconds));
        let new_accrued = base_accrued
            .saturating_add(accrued_over_elapsed_seconds)
            .min(allocation);

        let mut stream_after_accrual = self.clone();
        stream_after_accrual.accrued = new_accrued;

        if stream_after_accrual.unaccrued() == (0 as Balance) {
            stream_after_accrual.state = StreamState::Paused;
            let unaccrued_before_accrual_interval = self.unaccrued();
            // Ceiling division: the stream depleted partway through the last second.
            // Rounding up places `accrued_as_of` at the first second when `accrued == allocation`,
            // which is the earliest time a fold from `base_as_of` could reach depletion.
            let time_to_depletion =
                u64::try_from(unaccrued_before_accrual_interval.div_ceil(u128::from(rate)))
                    .map_err(|_| ErrorCode::ArithmeticOverflow)?;
            let depleted_at = base_as_of
                .checked_add(time_to_depletion)
                .ok_or(ErrorCode::ArithmeticOverflow)?;
            stream_after_accrual.accrued_as_of = depleted_at;
            Ok(stream_after_accrual)
        } else {
            stream_after_accrual.accrued_as_of = t;
            Ok(stream_after_accrual)
        }
    }

    /// Check `accrued <= allocation`, rate rules, and zero-allocation cases (`Paused` or `Closed` with zero accrued).
    pub fn validate_invariants(&self) -> Result<(), ErrorCode> {
        if self.accrued > self.allocation {
            return Err(ErrorCode::StreamExceedsAllocation);
        }
        if self.allocation == 0 {
            if self.accrued != 0 {
                return Err(ErrorCode::StreamExceedsAllocation);
            }
            return match self.state {
                StreamState::Active => Err(ErrorCode::ZeroStreamAllocation),
                StreamState::Paused | StreamState::Closed => Ok(()),
            };
        }
        if self.rate == 0 {
            return Err(ErrorCode::ZeroStreamRate);
        }
        Ok(())
    }

    /// Move a paused stream to [`StreamState::Active`] at `now`.
    ///
    /// Set [`StreamConfig::accrued_as_of`] to `now`
    /// so the next [`StreamConfig::at_time`] counts from the resume point.
    /// Leave [`StreamConfig::accrued`] unchanged: wall time spent in the paused state
    /// must not retroactively accrue on the next fold.
    ///
    /// Returns [`ErrorCode::ResumeZeroUnaccrued`] when unaccrued is zero; depleted streams cannot resume.
    pub fn resume_from_paused_at_time(self, now: Timestamp) -> Result<Self, ErrorCode> {
        if self.state != StreamState::Paused {
            return Err(ErrorCode::StreamNotPaused);
        }
        if self.unaccrued() == (0 as Balance) {
            return Err(ErrorCode::ResumeZeroUnaccrued);
        }
        let mut stream_after_resume = self;
        stream_after_resume.state = StreamState::Active;
        stream_after_resume.accrued_as_of = now;
        Ok(stream_after_resume)
    }

    /// Close at chain time `now`: run [`StreamConfig::at_time`],
    /// set [`StreamState::Closed`], trim `allocation` to accrued.
    ///
    /// Returns the unaccrued amount released and the closed [`StreamConfig`].
    /// The caller is responsible for applying the released amount to `VaultConfig.total_allocated`.
    /// Closed streams may still retain accrued funds after this step: `allocation` is trimmed to
    /// `accrued`, not to zero, so the provider can still claim the residual later.
    ///
    /// Returns [`ErrorCode::StreamClosed`] if the stream is already closed.
    pub fn close_at_time(self, now: Timestamp) -> Result<(Balance, Self), ErrorCode> {
        let stream_config_now = self.at_time(now)?;
        if stream_config_now.state == StreamState::Closed {
            return Err(ErrorCode::StreamClosed);
        }
        let unaccrued_released = stream_config_now.unaccrued();
        let accrued = stream_config_now.accrued;
        let mut stream_after_close = stream_config_now;
        stream_after_close.state = StreamState::Closed;
        stream_after_close.allocation = accrued;
        Ok((unaccrued_released, stream_after_close))
    }

    /// Pay out post-`at_time` [`StreamConfig::accrued`] at `now`:
    /// run [`StreamConfig::at_time`], shrink `allocation`, clear `accrued`, keep [`StreamState`].
    ///
    /// Returns the payout amount and the post-claim [`StreamConfig`].
    /// The caller is responsible for applying the payout to `VaultConfig.total_allocated`.
    /// This keeps vault and stream accounting aligned even for claims from closed streams with
    /// residual accrued balance: both the stream's `allocation` and the vault's
    /// `total_allocated` decrease by the same payout.
    ///
    /// Returns [`ErrorCode::ZeroClaimAmount`] when accrued is zero after the fold.
    pub fn claim_at_time(self, now: Timestamp) -> Result<(Balance, Self), ErrorCode> {
        let stream_config_now = self.at_time(now)?;
        let payout = stream_config_now.accrued;
        if payout == (0 as Balance) {
            return Err(ErrorCode::ZeroClaimAmount);
        }
        let mut stream_after_claim = stream_config_now;
        stream_after_claim.allocation = stream_after_claim
            .allocation
            .checked_sub(payout)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        stream_after_claim.accrued = 0 as Balance;
        stream_after_claim.validate_invariants()?;
        Ok((payout, stream_after_claim))
    }
}

#[cfg(test)]
mod stream_test_fixtures {
    use super::{StreamConfig, StreamState};
    use crate::{Timestamp, TokensPerSecond, DEFAULT_VERSION};
    use nssa_core::account::{AccountId, Balance};

    fn account(n: u8) -> AccountId {
        AccountId::new([n; 32])
    }

    pub(super) fn stream_active(
        accrued: Balance,
        allocation: Balance,
        rate: TokensPerSecond,
        accrued_as_of: Timestamp,
    ) -> StreamConfig {
        StreamConfig {
            version: DEFAULT_VERSION,
            stream_id: 0,
            provider: account(2),
            rate,
            allocation,
            accrued,
            state: StreamState::Active,
            accrued_as_of,
        }
    }
}

#[cfg(test)]
mod unaccrued_tests {
    use super::stream_test_fixtures::stream_active;
    use nssa_core::account::Balance;

    #[test]
    fn unaccrued_saturating_sub() {
        let s_active = stream_active(30, 100, 1, 0);
        assert_eq!(s_active.unaccrued(), 70 as Balance);
        let s_accrued_past_cap = stream_active(150, 100, 1, 0);
        assert_eq!(s_accrued_past_cap.unaccrued(), 0 as Balance);
    }
}

#[cfg(test)]
mod validate_invariants_tests {
    use super::stream_test_fixtures::stream_active;
    use super::StreamState;

    #[test]
    fn validate_invariants_closed_zero_allocation_zero_accrued_succeeds() {
        let mut s = stream_active(0, 0, 0, 0);
        s.state = StreamState::Closed;
        assert!(s.validate_invariants().is_ok());
    }
}

#[cfg(test)]
mod at_time_tests {
    use super::stream_test_fixtures::stream_active;
    use super::StreamState;
    use crate::error_codes::ErrorCode;

    #[test]
    fn at_time_time_regression_fails() {
        let s_active = stream_active(0, 1000, 10, 100);
        assert_eq!(s_active.at_time(99), Err(ErrorCode::TimeRegression));
    }

    #[test]
    fn at_time_accrued_above_allocation_fails() {
        let s_invalid = stream_active(500, 100, 10, 100);
        assert_eq!(
            s_invalid.at_time(100),
            Err(ErrorCode::StreamExceedsAllocation)
        );
    }

    #[test]
    fn at_time_zero_rate_fails() {
        let s_zero_rate = stream_active(0, 100, 0, 0);
        assert_eq!(s_zero_rate.at_time(0), Err(ErrorCode::ZeroStreamRate));
    }

    #[test]
    fn at_time_zero_allocation_when_active_fails() {
        let s_zero_allocation = stream_active(0, 0, 10, 0);
        assert_eq!(
            s_zero_allocation.at_time(0),
            Err(ErrorCode::ZeroStreamAllocation)
        );
    }

    #[test]
    fn at_time_paused_zero_allocation_unchanged_succeeds() {
        let mut s = stream_active(0, 0, 10, 100);
        s.state = StreamState::Paused;
        assert!(s.validate_invariants().is_ok());
        let s_after = s.at_time(200).unwrap();
        assert_eq!(s_after.accrued, 0);
        assert_eq!(s_after.allocation, 0);
        assert_eq!(s_after.state, StreamState::Paused);
        assert_eq!(s_after.accrued_as_of, 100);
    }

    #[test]
    fn at_time_when_t_equals_accrued_as_of_unchanged_succeeds() {
        let s_active = stream_active(50, 1000, 10, 100);
        let s_at_same_clock = s_active.at_time(100).unwrap();
        assert_eq!(s_at_same_clock.accrued, 50);
        assert_eq!(s_at_same_clock.accrued_as_of, 100);
        assert_eq!(s_at_same_clock.state, StreamState::Active);
    }

    #[test]
    fn at_time_linear_accrual_succeeds() {
        let s_active = stream_active(0, 1000, 10, 100);
        let s_after = s_active.at_time(105).unwrap();
        assert_eq!(s_after.accrued, 50);
        assert_eq!(s_after.accrued_as_of, 105);
        assert_eq!(s_after.state, StreamState::Active);
    }

    #[test]
    fn at_time_paused_no_accrual_succeeds() {
        let mut s_paused = stream_active(100, 1000, 10, 100);
        s_paused.state = StreamState::Paused;
        let s_unchanged = s_paused.at_time(200).unwrap();
        assert_eq!(s_unchanged.accrued, 100);
        assert_eq!(s_unchanged.accrued_as_of, 100);
    }

    #[test]
    fn at_time_depletion_sets_accrued_as_of_to_depletion_instant_succeeds() {
        // allocation 100, rate 10/s, t0=0, accrued 0 -> deplete at t=10
        let s_active = stream_active(0, 100, 10, 0);
        let s_depleted = s_active.at_time(50).unwrap();
        assert_eq!(s_depleted.accrued, 100);
        assert_eq!(s_depleted.state, StreamState::Paused);
        assert_eq!(s_depleted.accrued_as_of, 10);
    }

    #[test]
    fn at_time_depletion_instant_not_clock_t_when_t_past_instant_succeeds() {
        let s_active = stream_active(0, 100, 10, 0);
        let s_depleted = s_active.at_time(100).unwrap();
        assert_eq!(s_depleted.accrued_as_of, 10);
        assert_eq!(s_depleted.accrued, 100);
        assert_eq!(s_depleted.state, StreamState::Paused);
    }
}

#[cfg(test)]
mod resume_from_paused_at_time_tests {
    use super::stream_test_fixtures::stream_active;
    use super::StreamState;
    use crate::error_codes::ErrorCode;
    use crate::Timestamp;
    use nssa_core::account::Balance;

    #[test]
    fn resume_from_paused_at_time_succeeds() {
        let mut s_paused = stream_active(10, 100, 5, 50);
        s_paused.state = StreamState::Paused;
        let now: Timestamp = 200;
        let s_resumed = s_paused.resume_from_paused_at_time(now).unwrap();
        assert_eq!(s_resumed.state, StreamState::Active);
        assert_eq!(s_resumed.accrued_as_of, now);
        assert_eq!(s_resumed.accrued, 10 as Balance);
        assert_eq!(s_resumed.allocation, 100 as Balance);
    }

    #[test]
    fn resume_from_paused_at_time_when_active_fails() {
        let s_active = stream_active(0, 100, 5, 0);
        assert_eq!(
            s_active.resume_from_paused_at_time(1),
            Err(ErrorCode::StreamNotPaused)
        );
    }

    #[test]
    fn resume_from_paused_at_time_when_closed_fails() {
        let mut s_closed = stream_active(0, 100, 5, 0);
        s_closed.state = StreamState::Closed;
        assert_eq!(
            s_closed.resume_from_paused_at_time(1),
            Err(ErrorCode::StreamNotPaused)
        );
    }

    #[test]
    fn resume_from_paused_at_time_zero_unaccrued_fails() {
        let mut s_paused_fully_accrued = stream_active(100, 100, 5, 10);
        s_paused_fully_accrued.state = StreamState::Paused;
        assert_eq!(
            s_paused_fully_accrued.resume_from_paused_at_time(20),
            Err(ErrorCode::ResumeZeroUnaccrued)
        );
    }
}

#[cfg(test)]
mod close_at_time_tests {
    use super::stream_test_fixtures::stream_active;
    use super::StreamState;
    use crate::error_codes::ErrorCode;
    use crate::Timestamp;
    use nssa_core::account::Balance;

    #[test]
    fn close_at_time_folds_accrual_before_releasing_succeeds() {
        let s = stream_active(0, 100, 10, 0);
        let now: Timestamp = 5;
        let (released, closed) = s.close_at_time(now).unwrap();
        assert_eq!(released, 50 as Balance);
        assert_eq!(closed.state, StreamState::Closed);
        assert_eq!(closed.allocation, 50 as Balance);
        assert_eq!(closed.accrued, 50 as Balance);
    }

    #[test]
    fn close_at_time_releases_unaccrued_succeeds() {
        let s = stream_active(30, 100, 1, 0);
        let now: Timestamp = 0;
        let (released, closed) = s.close_at_time(now).unwrap();
        assert_eq!(released, 70 as Balance);
        assert_eq!(closed.state, StreamState::Closed);
        assert_eq!(closed.allocation, 30 as Balance);
        assert_eq!(closed.accrued, 30 as Balance);
    }

    #[test]
    fn close_at_time_zero_unaccrued_releases_nothing_succeeds() {
        let mut s = stream_active(100, 100, 1, 0);
        s.state = StreamState::Paused;
        let now: Timestamp = 0;
        let (released, closed) = s.close_at_time(now).unwrap();
        assert_eq!(released, 0 as Balance);
        assert_eq!(closed.state, StreamState::Closed);
        assert_eq!(closed.allocation, 100 as Balance);
    }

    #[test]
    fn close_at_time_when_already_closed_fails() {
        let mut s = stream_active(0, 100, 1, 0);
        s.state = StreamState::Closed;
        assert_eq!(s.close_at_time(100), Err(ErrorCode::StreamClosed));
    }
}

#[cfg(test)]
mod claim_at_time_tests {
    use super::stream_test_fixtures::stream_active;
    use super::StreamState;
    use crate::error_codes::ErrorCode;
    use crate::Timestamp;
    use nssa_core::account::Balance;

    #[test]
    fn claim_at_time_zero_accrued_fails() {
        let s = stream_active(0, 100, 10, 0);
        assert_eq!(s.claim_at_time(0), Err(ErrorCode::ZeroClaimAmount));
    }

    #[test]
    fn claim_at_time_active_partial_payout_succeeds() {
        let s = stream_active(0, 100, 10, 0);
        let now: Timestamp = 5;
        let (payout, stream_after_claim) = s.claim_at_time(now).unwrap();
        assert_eq!(payout, 50 as Balance);
        assert_eq!(stream_after_claim.accrued, 0 as Balance);
        assert_eq!(stream_after_claim.allocation, 50 as Balance);
        assert_eq!(stream_after_claim.state, StreamState::Active);
    }

    #[test]
    fn claim_at_time_paused_drains_to_zero_succeeds() {
        let mut s = stream_active(80, 80, 1, 0);
        s.state = StreamState::Paused;
        let (payout, stream_after_claim) = s.claim_at_time(0).unwrap();
        assert_eq!(payout, 80 as Balance);
        assert_eq!(stream_after_claim.allocation, 0 as Balance);
        assert_eq!(stream_after_claim.accrued, 0 as Balance);
        assert_eq!(stream_after_claim.state, StreamState::Paused);
    }

    #[test]
    fn claim_at_time_closed_residual_succeeds() {
        let mut s = stream_active(30, 30, 1, 0);
        s.state = StreamState::Closed;
        let (payout, stream_after_claim) = s.claim_at_time(0).unwrap();
        assert_eq!(payout, 30 as Balance);
        assert_eq!(stream_after_claim.allocation, 0 as Balance);
        assert_eq!(stream_after_claim.accrued, 0 as Balance);
        assert_eq!(stream_after_claim.state, StreamState::Closed);
    }

    #[test]
    fn claim_at_time_time_regression_fails() {
        let s = stream_active(0, 1000, 10, 100);
        assert_eq!(s.claim_at_time(99), Err(ErrorCode::TimeRegression));
    }
}
