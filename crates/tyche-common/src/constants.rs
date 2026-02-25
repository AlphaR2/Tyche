use pinocchio::Address;

/// Default window before `end_time` during which a new bid arms a soft-close
/// extension, in seconds.
pub const SOFT_CLOSE_WINDOW_SECS: i64 = 300; // 5 min

/// Default extension added to `end_time` when a soft-close is triggered,
/// in seconds.
pub const SOFT_CLOSE_EXTENSION_SECS: i64 = 300; // 5 min

/// Default maximum number of soft-close extensions allowed per competition.
pub const MAX_SOFT_CLOSES: u8 = 5;

// ── Reserve price ────────────────────────────────────────────────────────────

/// Minimum valid `reserve_price` in lamports.
/// Prevents zero-reserve competitions that would accept any bid.
pub const MIN_RESERVE_PRICE_LAMPORTS: u64 = 1_000_000; // 0.001 SOL

// ── Participant cap ──────────────────────────────────────────────────────────

/// Maximum number of participants allowed in a single competition.
/// Bounds the on-chain linear scan performed during settlement.
pub const MAX_PARTICIPANTS: u32 = 1_000;

// ── Rent / sizing ────────────────────────────────────────────────────────────

/// Minimum lamport balance a competition account must hold above rent-exemption
/// before it can be settled. Guards against lamport-drain edge cases.
pub const COMPETITION_MIN_LAMPORTS: u64 = 10_000;

// ── Protocol crank ───────────────────────────────────────────────────────────

/// The protocol crank keypair pubkey.
///
/// `ExtendCompetition` and `CloseCompetition` are restricted to this signer.
/// This is the backend SessionManager keypair, not the per-competition authority —
/// it prevents any user from triggering soft-close extensions or premature closes.

pub const TYCHE_CRANK_PUBKEY: Address = Address::new_from_array([
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
]);
