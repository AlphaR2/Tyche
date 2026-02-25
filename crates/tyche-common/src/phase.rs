use num_enum::TryFromPrimitive;

/// CEE lifecycle phases for a Tyche competition.
///
/// Stored as a `u8` in `CompetitionState::phase`.
/// All transitions are one-directional and enforced exclusively by
/// `tyche-core` processors. No other program may write this field.
///
/// ```text
/// Scheduled ──► Active ──► Settling ──► Settled
///     │            │
///     │            └──► Cancelled  (Active → Cancelled only if participant_count == 0)
///     │
///     └──────────────► Cancelled  (Scheduled → Cancelled at any time)
/// ```
///
/// `Cancelled` and `Settled` are terminal states. No further transitions
/// are permitted once either is reached.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum Phase {
    /// Created on-chain. Not yet delegated to the MagicBlock PER.
    /// Configuration is still mutable. No bids are accepted.
    Scheduled = 0,

    /// Delegated to the MagicBlock PER. Bids are live inside the TEE.
    /// Sealed fields (`current_high_bid`, `current_winner`) are unreadable
    /// on mainnet. Soft-close extensions may push `end_time` forward.
    Active = 1,

    /// `end_time` has elapsed. The PER session is finalizing.
    /// Accounts are being undelegated back to mainnet. No new bids
    /// are accepted. The competition awaits the settlement CPI.
    Settling = 2,

    /// Winner determined and escrow released. Terminal state.
    Settled = 3,

    /// Authority cancelled the competition before any bids were placed.
    /// Reachable from `Scheduled` at any time, or from `Active` only
    /// when `participant_count == 0`. Terminal state.
    Cancelled = 4,
}