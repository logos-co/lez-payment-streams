//! [`StreamState`], [`StreamConfig`], and lazy accrual math.

use core::mem::size_of;

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use nssa_core::account::{AccountId, Balance};

use crate::error_codes::ErrorCode;
use crate::vault::checked_total_allocated_after_release;
use crate::{StreamId, Timestamp, TokensPerSecond, VersionId, DEFAULT_VERSION};

/// Stream lifecycle. One byte on the wire (ordinal).
#[repr(u8)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[borsh(use_discriminant = true)]
pub enum StreamState {
    Active = 0,
    Paused = 1,
    Closed = 2,
}

impl StreamState {
    pub fn from_discriminant(d: u8) -> Option<Self> {
        match d {
            0 => Some(Self::Active),
            1 => Some(Self::Paused),
            2 => Some(Self::Closed),
            _ => None,
        }
    }
}

/// Stream PDA account body. Vault identity comes from the stream PDA seeds at derivation time, not from this struct.
#[spel_framework_macros::account_type]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct StreamConfig {
    pub version: VersionId,
    /// Match the `stream_id` seed in the stream PDA derivation.
    pub stream_id: StreamId,
    pub provider: AccountId,
    /// Tokens per second (LEZ MVP uses one-second steps).
    pub rate: TokensPerSecond,
    pub allocation: Balance,
    pub accrued: Balance,
    pub state: StreamState,
    /// Latest chain time folded into `accrued`.
    /// When not depleted: equals `t` after the most recent `at_time` call.
    /// When depleted: equals the depletion instant `⌈unaccrued/rate⌉` seconds after the prior
    /// snapshot, which may be before `t` when the stream exhausted mid-interval.
    pub accrued_as_of: Timestamp,
}

impl StreamConfig {
    pub const SIZE: usize = size_of::<VersionId>()
        + size_of::<StreamId>()
        + size_of::<AccountId>()
        + size_of::<TokensPerSecond>()
        + size_of::<Balance>()
        + size_of::<Balance>()
        + size_of::<StreamState>()
        + size_of::<Timestamp>();

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(Self::SIZE);
        buf.extend_from_slice(&self.version.to_le_bytes());
        buf.extend_from_slice(&self.stream_id.to_le_bytes());
        buf.extend_from_slice(self.provider.value());
        buf.extend_from_slice(&self.rate.to_le_bytes());
        buf.extend_from_slice(&self.allocation.to_le_bytes());
        buf.extend_from_slice(&self.accrued.to_le_bytes());
        buf.push(self.state as u8);
        buf.extend_from_slice(&self.accrued_as_of.to_le_bytes());
        buf
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() != Self::SIZE {
            return None;
        }
        let mut offset = 0;

        let size = size_of::<VersionId>();
        let version = VersionId::from_le_bytes(data[offset..offset + size].try_into().ok()?);
        offset += size;

        let size = size_of::<StreamId>();
        let stream_id = StreamId::from_le_bytes(data[offset..offset + size].try_into().ok()?);
        offset += size;

        let size = size_of::<AccountId>();
        let provider = AccountId::new(data[offset..offset + size].try_into().ok()?);
        offset += size;

        let size = size_of::<TokensPerSecond>();
        let rate = TokensPerSecond::from_le_bytes(data[offset..offset + size].try_into().ok()?);
        offset += size;

        let size = size_of::<Balance>();
        let allocation = Balance::from_le_bytes(data[offset..offset + size].try_into().ok()?);
        offset += size;

        let size = size_of::<Balance>();
        let accrued = Balance::from_le_bytes(data[offset..offset + size].try_into().ok()?);
        offset += size;

        let state = StreamState::from_discriminant(*data.get(offset)?)?;
        offset += size_of::<StreamState>();

        let size = size_of::<Timestamp>();
        let accrued_as_of = Timestamp::from_le_bytes(data[offset..offset + size].try_into().ok()?);

        Some(Self {
            version,
            stream_id,
            provider,
            rate,
            allocation,
            accrued,
            state,
            accrued_as_of,
        })
    }

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
    /// Map [`StreamConfig::validate_invariants`] failures to [`ErrorCode::ZeroStreamRate`], [`ErrorCode::ZeroStreamAllocation`], or [`ErrorCode::StreamExceedsAllocation`].
    /// Emit [`ErrorCode::TimeRegression`] when `t` is before [`StreamConfig::accrued_as_of`].
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
            let unaccrued_before_interval = self.unaccrued();
            // Ceiling division: the stream depleted partway through the last second.
            // Rounding up places `accrued_as_of` at the first second when `accrued == allocation`,
            // which is the earliest time a fold from `base_as_of` could reach depletion.
            let time_to_depletion = div_ceil_u128(unaccrued_before_interval, rate)
                .ok_or(ErrorCode::ArithmeticOverflow)?;
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
    /// Emit [`ErrorCode::StreamNotPaused`] unless state is [`StreamState::Paused`].
    /// Emit [`ErrorCode::ResumeZeroUnaccrued`] when [`StreamConfig::unaccrued`] is zero.
    pub fn resume_from_paused_at(self, now: Timestamp) -> Result<Self, ErrorCode> {
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
    /// release [`StreamConfig::unaccrued`] from `vault_total_allocated`,
    /// set [`StreamState::Closed`], trim `allocation` to accrued.
    ///
    /// Return the new vault [`crate::VaultConfig::total_allocated`] aggregate and the closed [`StreamConfig`].
    ///
    /// Emit [`ErrorCode::StreamClosed`] if the fold already produced [`StreamState::Closed`]. Otherwise match [`StreamConfig::at_time`] (for example [`ErrorCode::TimeRegression`]).
    pub fn close_at_time(
        self,
        now: Timestamp,
        vault_total_allocated: Balance,
    ) -> Result<(Balance, Self), ErrorCode> {
        let stream_config_now = self.at_time(now)?;
        if stream_config_now.state == StreamState::Closed {
            return Err(ErrorCode::StreamClosed);
        }
        let unaccrued_amount = stream_config_now.unaccrued();
        let next_vault_total_allocated =
            checked_total_allocated_after_release(vault_total_allocated, unaccrued_amount)?;
        let accrued = stream_config_now.accrued;
        let mut stream_after_close = stream_config_now;
        stream_after_close.state = StreamState::Closed;
        stream_after_close.allocation = accrued;
        Ok((next_vault_total_allocated, stream_after_close))
    }

    /// Pay out post-`at_time` [`StreamConfig::accrued`] at `now`:
    /// run [`StreamConfig::at_time`], shrink `allocation`, clear `accrued`, keep [`StreamState`].
    ///
    /// Emit [`ErrorCode::ZeroClaimAmount`] when accrued is zero after the fold.
    /// Otherwise follow [`StreamConfig::at_time`] or vault math.
    pub fn claim_at_time(
        self,
        now: Timestamp,
        vault_total_allocated: Balance,
    ) -> Result<(Balance, Balance, Self), ErrorCode> {
        let stream_config_now = self.at_time(now)?;
        let payout = stream_config_now.accrued;
        if payout == (0 as Balance) {
            return Err(ErrorCode::ZeroClaimAmount);
        }
        let next_vault_total_allocated =
            checked_total_allocated_after_release(vault_total_allocated, payout)?;
        let mut stream_after_claim = stream_config_now;
        stream_after_claim.allocation = stream_after_claim
            .allocation
            .checked_sub(payout)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        stream_after_claim.accrued = 0 as Balance;
        stream_after_claim.validate_invariants()?;
        Ok((next_vault_total_allocated, payout, stream_after_claim))
    }
}

/// `ceil(rem / rate)` with `rate > 0`. Zero remainder yields zero.
fn div_ceil_u128(rem: u128, rate: u64) -> Option<u64> {
    if rate == 0 {
        return None;
    }
    let r = u128::from(rate);
    let q = (rem + r - 1) / r;
    u64::try_from(q).ok()
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
mod stream_config_at_time_tests {
    use super::stream_test_fixtures::stream_active;
    use super::StreamState;
    use crate::error_codes::ErrorCode;
    use crate::Timestamp;
    use nssa_core::account::Balance;

    #[test]
    fn unaccrued_saturating_sub() {
        let s_active = stream_active(30, 100, 1, 0);
        assert_eq!(s_active.unaccrued(), 70 as Balance);
        let s_accrued_past_cap = stream_active(150, 100, 1, 0);
        assert_eq!(s_accrued_past_cap.unaccrued(), 0 as Balance);
    }

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
    fn at_time_idle_paused_zero_allocation_unchanged_succeeds() {
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
    fn validate_closed_stream_zero_allocation_zero_accrued_succeeds() {
        let mut s = stream_active(0, 0, 0, 0);
        s.state = StreamState::Closed;
        assert!(s.validate_invariants().is_ok());
    }

    #[test]
    fn at_time_when_t_equals_accrued_as_of_unchanged_accrued_succeeds() {
        let s_active = stream_active(50, 1000, 10, 100);
        let s_at_same_clock = s_active.at_time(100).unwrap();
        assert_eq!(s_at_same_clock.accrued, 50);
        assert_eq!(s_at_same_clock.accrued_as_of, 100);
        assert_eq!(s_at_same_clock.state, StreamState::Active);
    }

    #[test]
    fn at_time_linear_accrual_succeeds() {
        let s_active = stream_active(0, 1000, 10, 100);
        let s_after_at_time = s_active.at_time(105).unwrap();
        assert_eq!(s_after_at_time.accrued, 50);
        assert_eq!(s_after_at_time.accrued_as_of, 105);
        assert_eq!(s_after_at_time.state, StreamState::Active);
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
    fn at_time_caps_and_paused_accrued_as_of_depletion_instant_succeeds() {
        // allocation 100, rate 10/s, t0=0, accrued 0 -> deplete at t=10
        let s_active = stream_active(0, 100, 10, 0);
        let s_depleted_paused = s_active.at_time(50).unwrap();
        assert_eq!(s_depleted_paused.accrued, 100);
        assert_eq!(s_depleted_paused.state, StreamState::Paused);
        assert_eq!(s_depleted_paused.accrued_as_of, 10);
    }

    #[test]
    fn at_time_depletion_not_clock_t_when_t_past_instant_succeeds() {
        let s_active = stream_active(0, 100, 10, 0);
        let s_depleted_paused = s_active.at_time(100).unwrap();
        assert_eq!(s_depleted_paused.accrued_as_of, 10);
        assert_eq!(s_depleted_paused.accrued, 100);
        assert_eq!(s_depleted_paused.state, StreamState::Paused);
    }

    #[test]
    fn resume_from_paused_at_succeeds() {
        let mut s_paused = stream_active(10, 100, 5, 50);
        s_paused.state = StreamState::Paused;
        let now: Timestamp = 200;
        let s_resumed = s_paused.resume_from_paused_at(now).unwrap();
        assert_eq!(s_resumed.state, StreamState::Active);
        assert_eq!(s_resumed.accrued_as_of, now);
        assert_eq!(s_resumed.accrued, 10 as Balance);
        assert_eq!(s_resumed.allocation, 100 as Balance);
    }

    #[test]
    fn resume_from_paused_at_when_active_fails() {
        let s_active = stream_active(0, 100, 5, 0);
        assert_eq!(
            s_active.resume_from_paused_at(1),
            Err(ErrorCode::StreamNotPaused)
        );
    }

    #[test]
    fn resume_from_paused_at_when_closed_fails() {
        let mut s_closed = stream_active(0, 100, 5, 0);
        s_closed.state = StreamState::Closed;
        assert_eq!(
            s_closed.resume_from_paused_at(1),
            Err(ErrorCode::StreamNotPaused)
        );
    }

    #[test]
    fn resume_from_paused_at_zero_unaccrued_fails() {
        let mut s_paused_fully_accrued = stream_active(100, 100, 5, 10);
        s_paused_fully_accrued.state = StreamState::Paused;
        assert_eq!(
            s_paused_fully_accrued.resume_from_paused_at(20),
            Err(ErrorCode::ResumeZeroUnaccrued)
        );
    }

    #[test]
    fn close_at_time_folds_accrual_before_releasing_succeeds() {
        let s = stream_active(0, 100, 10, 0);
        let vault_total: Balance = 100;
        let now: Timestamp = 5;
        let (next_vault_total_allocated, closed) = s.close_at_time(now, vault_total).unwrap();
        assert_eq!(next_vault_total_allocated, 50 as Balance);
        assert_eq!(closed.state, StreamState::Closed);
        assert_eq!(closed.allocation, 50 as Balance);
        assert_eq!(closed.accrued, 50 as Balance);
    }

    #[test]
    fn close_at_time_releases_unaccrued_succeeds() {
        let s = stream_active(30, 100, 1, 0);
        let vault_total: Balance = 100;
        let now: Timestamp = 0;
        let (next_vault_total_allocated, closed) = s.close_at_time(now, vault_total).unwrap();
        assert_eq!(next_vault_total_allocated, 30 as Balance);
        assert_eq!(closed.state, StreamState::Closed);
        assert_eq!(closed.allocation, 30 as Balance);
        assert_eq!(closed.accrued, 30 as Balance);
    }

    #[test]
    fn close_at_time_zero_unaccrued_no_vault_change_succeeds() {
        let mut s = stream_active(100, 100, 1, 0);
        s.state = StreamState::Paused;
        let vault_total: Balance = 100;
        let now: Timestamp = 0;
        let (next_vault_total_allocated, closed) = s.close_at_time(now, vault_total).unwrap();
        assert_eq!(next_vault_total_allocated, vault_total);
        assert_eq!(closed.state, StreamState::Closed);
        assert_eq!(closed.allocation, 100 as Balance);
    }

    #[test]
    fn close_at_time_when_already_closed_fails() {
        let mut s = stream_active(0, 100, 1, 0);
        s.state = StreamState::Closed;
        assert_eq!(
            s.close_at_time(100, 100 as Balance),
            Err(ErrorCode::StreamClosed)
        );
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
        assert_eq!(
            s.claim_at_time(0, 100 as Balance),
            Err(ErrorCode::ZeroClaimAmount)
        );
    }

    #[test]
    fn claim_at_time_active_partial_payout_succeeds() {
        let s = stream_active(0, 100, 10, 0);
        let vault_total: Balance = 100;
        let now: Timestamp = 5;
        let (next_total, payout, stream_after_claim) = s.claim_at_time(now, vault_total).unwrap();
        assert_eq!(payout, 50 as Balance);
        assert_eq!(next_total, 50 as Balance);
        assert_eq!(stream_after_claim.accrued, 0 as Balance);
        assert_eq!(stream_after_claim.allocation, 50 as Balance);
        assert_eq!(stream_after_claim.state, StreamState::Active);
    }

    #[test]
    fn claim_at_time_paused_drains_to_zero_succeeds() {
        let mut s = stream_active(80, 80, 1, 0);
        s.state = StreamState::Paused;
        let (next_total, payout, stream_after_claim) = s.claim_at_time(0, 80 as Balance).unwrap();
        assert_eq!(payout, 80 as Balance);
        assert_eq!(next_total, 0 as Balance);
        assert_eq!(stream_after_claim.allocation, 0 as Balance);
        assert_eq!(stream_after_claim.accrued, 0 as Balance);
        assert_eq!(stream_after_claim.state, StreamState::Paused);
    }

    #[test]
    fn claim_at_time_closed_residual_succeeds() {
        let mut s = stream_active(30, 30, 1, 0);
        s.state = StreamState::Closed;
        let (next_total, payout, stream_after_claim) = s.claim_at_time(0, 30 as Balance).unwrap();
        assert_eq!(payout, 30 as Balance);
        assert_eq!(next_total, 0 as Balance);
        assert_eq!(stream_after_claim.allocation, 0 as Balance);
        assert_eq!(stream_after_claim.accrued, 0 as Balance);
        assert_eq!(stream_after_claim.state, StreamState::Closed);
    }

    #[test]
    fn claim_at_time_time_regression_fails() {
        let s = stream_active(0, 1000, 10, 100);
        assert_eq!(
            s.claim_at_time(99, 100 as Balance),
            Err(ErrorCode::TimeRegression)
        );
    }
}
