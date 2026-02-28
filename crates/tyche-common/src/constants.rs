
/// Default window before `end_time` during which a new bid arms a soft-close
/// extension, in seconds.
pub const SOFT_CLOSE_WINDOW_SECS: i64 = 300; // 5 min

/// Default extension added to `end_time` when a soft-close is triggered,
/// in seconds.
pub const SOFT_CLOSE_EXTENSION_SECS: i64 = 300; // 5 min

/// Default maximum number of soft-close extensions allowed per competition.
pub const MAX_SOFT_CLOSES: u8 = 5;


// ── Participant cap

/// Maximum number of participants allowed in a single competition.
/// Bounds the on-chain linear scan performed during settlement.
pub const MAX_PARTICIPANTS: u32 = 1_000;

// ── Rent / sizing

/// Minimum lamport balance a competition account must hold above rent-exemption
/// before it can be settled. Guards against lamport-drain edge cases.
pub const COMPETITION_MIN_LAMPORTS: u64 = 10_000;

// ── Fee ceiling

/// Maximum protocol fee in basis points (1 bp = 0.01%).
///
/// Hard-coded to 1 000 bp (10%). This ceiling can never be raised by config —
/// it is enforced at the instruction level in `InitializeProtocolConfig` and
/// `UpdateProtocolConfig`.
pub const MAX_FEE_BASIS_POINTS: u16 = 1_000; // 10%
