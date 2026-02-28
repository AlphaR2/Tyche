/// PDA seed for `CompetitionState` accounts owned by `tyche-core`.
///
/// Full seeds: `[COMPETITION_SEED, authority_pubkey, id_bytes]`
pub const COMPETITION_SEED: &[u8] = b"competition";

/// PDA seed for `ParticipantRecord` accounts owned by `tyche-core`.
///
/// Full seeds: `[PARTICIPANT_SEED, competition_pubkey, participant_pubkey]`
pub const PARTICIPANT_SEED: &[u8] = b"participant";

/// PDA seed for `EscrowVault` accounts owned by `tyche-escrow`.
///
/// Full seeds: `[VAULT_SEED, competition_pubkey, depositor_pubkey]`
pub const VAULT_SEED: &[u8] = b"vault";

/// PDA seed for `AuctionState` accounts owned by `tyche-auction`.
///
/// Full seeds: `[AUCTION_SEED, competition_pubkey]`
pub const AUCTION_SEED: &[u8] = b"auction";

/// PDA seed for `BidRecord` accounts owned by `tyche-auction`.
///
/// Full seeds: `[BID_SEED, competition_pubkey, bidder_pubkey]`
pub const BID_SEED: &[u8] = b"bid";

/// PDA seed for the singleton `ProtocolConfig` account owned by `tyche-core`.
///
/// Full seeds: `[PROTOCOL_CONFIG_SEED]` — no additional components,
/// exactly one config account can exist per program deployment.
pub const PROTOCOL_CONFIG_SEED: &[u8] = b"protocol_config";
