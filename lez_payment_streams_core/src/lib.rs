use serde::{Deserialize, Serialize};

use nssa_core::account::{AccountId, Balance};
use nssa_core::program::ProgramId;

use core::mem::size_of;

#[cfg(test)]
mod harness_seeds;

#[cfg(test)]
mod test_helpers;

#[cfg(test)]
mod program_tests;

mod mock_timestamp;
pub use mock_timestamp::MockTimestamp;

// ---- Type aliases ---- //

pub type VersionId = u8;
pub type VaultId = u64;
pub type StreamId = u64;
pub type TokensPerSecond = u64;
pub type Timestamp = u64;

// ---- Version ---- //

pub const DEFAULT_VERSION: VersionId = 1;

// ---- Custom errors --- //

pub const ERR_ZERO_DEPOSIT_AMOUNT: u32 = 6001;
pub const ERR_VERSION_MISMATCH: u32 = 6002;
pub const ERR_VAULT_ID_MISMATCH: u32 = 6003;
pub const ERR_INSUFFICIENT_FUNDS: u32 = 6004;
/// Addition, division, or other arithmetic does not fit the target type (e.g. balance, timestamp delta).
pub const ERR_ARITHMETIC_OVERFLOW: u32 = 6005;
pub const ERR_ZERO_WITHDRAW_AMOUNT: u32 = 6006;
pub const ERR_ZERO_STREAM_RATE: u32 = 6007;
pub const ERR_ZERO_STREAM_ALLOCATION: u32 = 6008;
pub const ERR_STREAM_ID_MISMATCH: u32 = 6009;
pub const ERR_TOTAL_ALLOCATED_OVERFLOW: u32 = 6010;
pub const ERR_INVALID_MOCK_TIMESTAMP: u32 = 6011;
pub const ERR_ALLOCATION_EXCEEDS_UNALLOCATED: u32 = 6012;
pub const ERR_NEXT_STREAM_ID_OVERFLOW: u32 = 6013;
pub const ERR_TIME_REGRESSION: u32 = 6014;
pub const ERR_STREAM_EXCEEDS_ALLOCATION: u32 = 6015;
/// Signer account is not [`VaultConfig::owner`] when the instruction requires the vault owner.
pub const ERR_VAULT_OWNER_MISMATCH: u32 = 6016;
/// `pause_stream` when post-`at_time` state is not [`StreamState::Active`].
pub const ERR_STREAM_NOT_ACTIVE: u32 = 6017;
/// `resume_stream` when post-`at_time` state is not [`StreamState::Paused`].
pub const ERR_STREAM_NOT_PAUSED: u32 = 6018;
/// `resume_stream` when remaining allocation is zero ([`StreamConfig::remaining_allocation`]).
pub const ERR_RESUME_ZERO_REMAINING_ALLOCATION: u32 = 6019;
/// `top_up_stream` when post-`at_time` state is [`StreamState::Closed`].
pub const ERR_STREAM_CLOSED: u32 = 6020;
/// `top_up_stream` when `vault_total_allocated_increase` is zero.
pub const ERR_ZERO_TOP_UP_AMOUNT: u32 = 6021;


// ---- VaultConfig ---- //

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultConfig {
    pub version: VersionId,
    pub owner: AccountId,
    pub vault_id: VaultId,
    pub next_stream_id: StreamId,
    pub total_allocated: Balance,
}

impl VaultConfig {
    pub const SIZE: usize = size_of::<VersionId>()
        + size_of::<AccountId>()
        + size_of::<VaultId>()
        + size_of::<StreamId>()
        + size_of::<Balance>();

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(Self::SIZE);
        buf.extend_from_slice(&self.version.to_le_bytes());
        buf.extend_from_slice(self.owner.value());
        buf.extend_from_slice(&self.vault_id.to_le_bytes());
        buf.extend_from_slice(&self.next_stream_id.to_le_bytes());
        buf.extend_from_slice(&self.total_allocated.to_le_bytes());
        buf
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() != Self::SIZE {
            return None;
        }
        // extract fields
        // version
        let mut offset = 0;
        let size = size_of::<VersionId>();
        let version = VersionId::from_le_bytes(data[offset..offset + size].try_into().ok()?);
        offset += size;

        // owner
        let size = size_of::<AccountId>();
        let owner = AccountId::new(data[offset..offset + size].try_into().ok()?);
        offset += size;

        // vault_id
        let size = size_of::<VaultId>();
        let vault_id = VaultId::from_le_bytes(data[offset..offset + size].try_into().ok()?);
        offset += size;

        // next_stream_id
        let size = size_of::<StreamId>();
        let next_stream_id = StreamId::from_le_bytes(data[offset..offset + size].try_into().ok()?);
        offset += size;

        // total_allocated
        let size = size_of::<Balance>();
        let total_allocated = Balance::from_le_bytes(data[offset..offset + size].try_into().ok()?);

        Some(Self {
            version,
            owner,
            vault_id,
            next_stream_id,
            total_allocated,
        })
    }

    pub fn new(owner: AccountId, vault_id: VaultId) -> Self {
        Self::new_with_version(owner, vault_id, DEFAULT_VERSION)
    }

    pub fn new_with_version(owner: AccountId, vault_id: VaultId, version: VersionId) -> Self {
        Self {
            version,
            owner,
            vault_id,
            next_stream_id: StreamId::MIN,
            total_allocated: Balance::MIN,
        }
    }
}

// ---- VaultHolding ---- //

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultHolding {
    pub version: VersionId,
}

impl VaultHolding {
    pub const SIZE: usize = size_of::<VersionId>();

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(Self::SIZE);
        buf.extend_from_slice(&self.version.to_le_bytes());
        buf
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() != Self::SIZE {
            return None;
        }
        // extract fields (one field - version - only)
        let version = VersionId::from_le_bytes(data[..Self::SIZE].try_into().ok()?);

        Some(Self { version })
    }

    pub fn new() -> Self {
        Self::new_with_version(DEFAULT_VERSION)
    }

    pub fn new_with_version(version: VersionId) -> Self {
        Self { version }
    }
}

// ---- Vault bookkeeping ---- //

/// Increase [`VaultConfig::total_allocated`] by `vault_total_allocated_increase`, capped by unallocated vault liquidity.
///
/// Unallocated is `vault_holding_balance.saturating_sub(vault_total_allocated)`.
/// This is vault-level bookkeeping only (how much of the holding balance is reserved across streams).
/// For [`Instruction::CreateStream`], the increase equals the new stream's allocation; for
/// [`Instruction::TopUpStream`], it equals the stream allocation delta.
///
/// Callers must reject zero `vault_total_allocated_increase` where the instruction forbids it
/// ([`ERR_ZERO_STREAM_ALLOCATION`], [`ERR_ZERO_TOP_UP_AMOUNT`]).
///
/// [`ERR_TOTAL_ALLOCATED_OVERFLOW`] from `checked_add` is defensive; for `Balance` as `u128`,
/// passing the unallocated check with realistic on-chain balances implies the sum fits in `u128`.
pub fn commit_vault_total_allocated_increase(
    vault_holding_balance: Balance,
    vault_total_allocated: Balance,
    vault_total_allocated_increase: Balance,
) -> Result<Balance, u32> {
    let unallocated = vault_holding_balance.saturating_sub(vault_total_allocated);
    if vault_total_allocated_increase > unallocated {
        return Err(ERR_ALLOCATION_EXCEEDS_UNALLOCATED);
    }
    vault_total_allocated
        .checked_add(vault_total_allocated_increase)
        .ok_or(ERR_TOTAL_ALLOCATED_OVERFLOW)
}

// ---- StreamConfig ---- //

/// Lifecycle state for a stream. Encoded as a single byte (ordinal) to match Borsh-style enums.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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

/// Serialized body of the stream PDA account.
/// Vault identity is not stored; it is fixed by `vault_config_pda` in the PDA seeds.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StreamConfig {
    pub version: VersionId,
    /// Must equal the `stream_id` seed used to derive this account’s PDA.
    pub stream_id: StreamId,
    pub provider: AccountId,
    /// Tokens per second (spec "tokens per time unit"; LEZ uses second granularity in MVP).
    pub rate: TokensPerSecond,
    pub allocation: Balance,
    pub accrued: Balance,
    pub state: StreamState,
    /// Chain time through which lazy accrual has been folded into `accrued`
    /// (may be before observation time).
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

    pub fn new(
        stream_id: StreamId,
        provider: AccountId,
        rate: TokensPerSecond,
        allocation: Balance,
        accrued_as_of: Timestamp,
    ) -> Self {
        Self::new_with_version(
            stream_id,
            provider,
            rate,
            allocation,
            accrued_as_of,
            DEFAULT_VERSION,
        )
    }

    pub fn new_with_version(
        stream_id: StreamId,
        provider: AccountId,
        rate: TokensPerSecond,
        allocation: Balance,
        accrued_as_of: Timestamp,
        version: VersionId,
    ) -> Self {
        Self {
            version,
            stream_id,
            provider,
            rate,
            allocation,
            accrued: Balance::MIN,
            state: StreamState::Active,
            accrued_as_of,
        }
    }

    /// Remaining allocation: portion of `allocation` not yet accrued (`allocation - accrued`, floored at zero).
    pub fn remaining_allocation(&self) -> Balance {
        self.allocation.saturating_sub(self.accrued)
    }

    /// Lazy accrual: compute stream state as of chain time `t`.
    ///
    /// Returns the stream unchanged when [`StreamState::Paused`] or [`StreamState::Closed`], or when
    /// `t` equals [`StreamConfig::accrued_as_of`] (no elapsed accrual interval).
    ///
    /// For a stream that was [`StreamState::Active`] with `t` strictly after
    /// [`StreamConfig::accrued_as_of`]: while [`StreamConfig::remaining_allocation`] stays positive
    /// after folding the interval, the result stays **Active**. When remaining allocation reaches
    /// zero, the result is **Paused** (accrual-induced pause; see `design.md`).
    ///
    /// Returns [`ERR_ZERO_STREAM_RATE`], [`ERR_ZERO_STREAM_ALLOCATION`], or [`ERR_STREAM_EXCEEDS_ALLOCATION`]
    /// when stored fields violate the same constraints as `create_stream` (non-zero rate and
    /// allocation; `accrued` not above `allocation`). Returns [`ERR_TIME_REGRESSION`] if `t` is
    /// strictly before [`StreamConfig::accrued_as_of`].
    pub fn at_time(&self, t: Timestamp) -> Result<Self, u32> {
        self.validate_invariants()?;

        match self.state {
            StreamState::Paused | StreamState::Closed => return Ok(self.clone()),
            StreamState::Active => {}
        }

        if t < self.accrued_as_of {
            return Err(ERR_TIME_REGRESSION);
        }

        if t == self.accrued_as_of {
            return Ok(self.clone());
        }

        let allocation = self.allocation;
        let base_as_of = self.accrued_as_of;
        let base_accrued = self.accrued;
        let rate = self.rate;

        // Tokens accrued by chain time `t`: `base_accrued + rate * (t - base_as_of)` with
        // saturating add, then capped at `allocation` ("saturated" = capped at the ceiling).
        // Here `t > base_as_of`, so `delta > 0`.
        let delta = t - base_as_of;
        let accrued_during_delta = u128::from(rate).saturating_mul(u128::from(delta));
        let new_accrued = base_accrued
            .saturating_add(accrued_during_delta)
            .min(allocation);

        let mut out = self.clone();
        out.accrued = new_accrued;

        if out.remaining_allocation() == (0 as Balance) {
            // Stream is depleted, transition to Paused.
            out.state = StreamState::Paused;
            // Remaining allocation at interval start (`accrued_as_of`), before applying this delta.
            let remaining_allocation_before_interval = self.remaining_allocation();
            let time_to_depletion = div_ceil_u128(remaining_allocation_before_interval, rate)
                .ok_or(ERR_ARITHMETIC_OVERFLOW)?;
            let depleted_at = base_as_of
                .checked_add(time_to_depletion)
                .ok_or(ERR_ARITHMETIC_OVERFLOW)?;
            out.accrued_as_of = depleted_at;
            Ok(out)
        } else {
            out.accrued_as_of = t;
            Ok(out)
        }
    }

    /// Same structural checks as `create_stream` on rate and allocation, plus `accrued <= allocation`.
    pub fn validate_invariants(&self) -> Result<(), u32> {
        if self.rate == 0 {
            return Err(ERR_ZERO_STREAM_RATE);
        }
        if self.allocation == 0 {
            return Err(ERR_ZERO_STREAM_ALLOCATION);
        }
        if self.accrued > self.allocation {
            return Err(ERR_STREAM_EXCEEDS_ALLOCATION);
        }
        Ok(())
    }

    /// Transition a **paused** stream to [`StreamState::Active`] at chain time `now`.
    ///
    /// Sets [`StreamConfig::accrued_as_of`] to `now` so the next [`StreamConfig::at_time`] does not
    /// treat wall-clock time spent paused as accrual time; the anchor is when streaming restarts.
    /// [`StreamConfig::accrued`] is unchanged.
    ///
    /// Returns [`ERR_STREAM_NOT_PAUSED`] if state is not [`StreamState::Paused`].
    /// Returns [`ERR_RESUME_ZERO_REMAINING_ALLOCATION`] if [`StreamConfig::remaining_allocation`]
    /// is zero.
    pub fn resume_from_paused_at(self, now: Timestamp) -> Result<Self, u32> {
        if self.state != StreamState::Paused {
            return Err(ERR_STREAM_NOT_PAUSED);
        }
        if self.remaining_allocation() == (0 as Balance) {
            return Err(ERR_RESUME_ZERO_REMAINING_ALLOCATION);
        }
        let mut out = self;
        out.state = StreamState::Active;
        out.accrued_as_of = now;
        Ok(out)
    }
}

/// `ceil(rem / rate)` for `rate > 0`. For `rem == 0`, the quotient is zero.
fn div_ceil_u128(rem: u128, rate: u64) -> Option<u64> {
    if rate == 0 {
        return None;
    }
    let r = u128::from(rate);
    let q = (rem + r - 1) / r;
    u64::try_from(q).ok()
}

#[cfg(test)]
mod stream_config_at_time_tests {
    use super::*;

    fn account(n: u8) -> AccountId {
        AccountId::new([n; 32])
    }

    fn stream_active(
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

    #[test]
    fn remaining_allocation_saturating_sub() {
        let s_active = stream_active(30, 100, 1, 0);
        assert_eq!(s_active.remaining_allocation(), 70 as Balance);
        let s_accrued_past_cap = stream_active(150, 100, 1, 0);
        assert_eq!(s_accrued_past_cap.remaining_allocation(), 0 as Balance);
    }

    #[test]
    fn at_time_rejects_time_regression() {
        let s_active = stream_active(0, 1000, 10, 100);
        assert_eq!(s_active.at_time(99), Err(ERR_TIME_REGRESSION));
    }

    #[test]
    fn at_time_rejects_accrued_above_allocation() {
        let s_invalid = stream_active(500, 100, 10, 100);
        assert_eq!(s_invalid.at_time(100), Err(ERR_STREAM_EXCEEDS_ALLOCATION));
    }

    #[test]
    fn at_time_rejects_zero_rate() {
        let s_zero_rate = stream_active(0, 100, 0, 0);
        assert_eq!(s_zero_rate.at_time(0), Err(ERR_ZERO_STREAM_RATE));
    }

    #[test]
    fn at_time_rejects_zero_allocation() {
        let s_zero_allocation = stream_active(0, 0, 10, 0);
        assert_eq!(s_zero_allocation.at_time(0), Err(ERR_ZERO_STREAM_ALLOCATION));
    }

    #[test]
    fn at_time_zero_delta_unchanged_accrued() {
        let s_active = stream_active(50, 1000, 10, 100);
        let s_at_same_clock = s_active.at_time(100).unwrap();
        assert_eq!(s_at_same_clock.accrued, 50);
        assert_eq!(s_at_same_clock.accrued_as_of, 100);
        assert_eq!(s_at_same_clock.state, StreamState::Active);
    }

    #[test]
    fn at_time_linear_accrual() {
        let s_active = stream_active(0, 1000, 10, 100);
        let s_after_at_time = s_active.at_time(105).unwrap();
        assert_eq!(s_after_at_time.accrued, 50);
        assert_eq!(s_after_at_time.accrued_as_of, 105);
        assert_eq!(s_after_at_time.state, StreamState::Active);
    }

    #[test]
    fn at_time_paused_no_accrual() {
        let mut s_paused = stream_active(100, 1000, 10, 100);
        s_paused.state = StreamState::Paused;
        let s_unchanged = s_paused.at_time(200).unwrap();
        assert_eq!(s_unchanged.accrued, 100);
        assert_eq!(s_unchanged.accrued_as_of, 100);
    }

    #[test]
    fn at_time_caps_and_paused_accrued_as_of_depletion_instant() {
        // allocation 100, rate 10/s, t0=0, accrued 0 -> deplete at t=10
        let s_active = stream_active(0, 100, 10, 0);
        let s_depleted_paused = s_active.at_time(50).unwrap();
        assert_eq!(s_depleted_paused.accrued, 100);
        assert_eq!(s_depleted_paused.state, StreamState::Paused);
        assert_eq!(s_depleted_paused.accrued_as_of, 10);
    }

    #[test]
    fn at_time_depletion_not_clock_t_when_t_past_instant() {
        let s_active = stream_active(0, 100, 10, 0);
        let s_depleted_paused = s_active.at_time(100).unwrap();
        assert_eq!(s_depleted_paused.accrued_as_of, 10);
        assert_eq!(s_depleted_paused.accrued, 100);
        assert_eq!(s_depleted_paused.state, StreamState::Paused);
    }

    #[test]
    fn resume_from_paused_at_success() {
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
    fn resume_from_paused_at_rejects_active() {
        let s_active = stream_active(0, 100, 5, 0);
        assert_eq!(
            s_active.resume_from_paused_at(1),
            Err(ERR_STREAM_NOT_PAUSED)
        );
    }

    #[test]
    fn resume_from_paused_at_rejects_closed() {
        let mut s_closed = stream_active(0, 100, 5, 0);
        s_closed.state = StreamState::Closed;
        assert_eq!(
            s_closed.resume_from_paused_at(1),
            Err(ERR_STREAM_NOT_PAUSED)
        );
    }

    #[test]
    fn resume_from_paused_at_rejects_zero_remaining() {
        let mut s_paused_fully_accrued = stream_active(100, 100, 5, 10);
        s_paused_fully_accrued.state = StreamState::Paused;
        assert_eq!(
            s_paused_fully_accrued.resume_from_paused_at(20),
            Err(ERR_RESUME_ZERO_REMAINING_ALLOCATION)
        );
    }
}

#[cfg(test)]
mod commit_vault_total_allocated_increase_tests {
    use super::*;

    #[test]
    fn commit_succeeds_within_unallocated() {
        let new_vault_total_allocated =
            commit_vault_total_allocated_increase(500, 200, 100).unwrap();
        assert_eq!(new_vault_total_allocated, 300 as Balance);
    }

    #[test]
    fn commit_rejects_exceeds_unallocated() {
        assert_eq!(
            commit_vault_total_allocated_increase(500, 400, 200),
            Err(ERR_ALLOCATION_EXCEEDS_UNALLOCATED)
        );
    }

}

// ---- Instruction ---- //

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instruction {
    InitializeVault {
        vault_id: VaultId,
    },
    Deposit {
        vault_id: VaultId,
        amount: Balance,
        authenticated_transfer_program_id: ProgramId,
    },
    Withdraw {
        vault_id: VaultId,
        amount: Balance,
    },
    CreateStream {
        vault_id: VaultId,
        stream_id: StreamId,
        provider: AccountId,
        rate: TokensPerSecond,
        allocation: Balance,
    },
    SyncStream {
        vault_id: VaultId,
        stream_id: StreamId,
    },
    PauseStream {
        vault_id: VaultId,
        stream_id: StreamId,
    },
    ResumeStream {
        vault_id: VaultId,
        stream_id: StreamId,
    },
    TopUpStream {
        vault_id: VaultId,
        stream_id: StreamId,
        vault_total_allocated_increase: Balance,
    },
}
