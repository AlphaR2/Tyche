/// PDA seed for the Registrar account.
///
/// Convention used by all SPL Governance voter-weight plugins:
///   seeds: `[realm_pubkey, b"registrar", governing_token_mint_pubkey]`
pub const REGISTRAR_SEED: &[u8] = b"registrar";

/// PDA seed prefix for the VoterWeightRecord account.
///
/// Convention used by all SPL Governance voter-weight plugins:
///   seeds: `[b"voter-weight-record", realm_pubkey, governing_token_mint_pubkey, voter_pubkey]`
pub const VOTER_WEIGHT_RECORD_SEED: &[u8] = b"voter-weight-record";

/// PDA seed prefix for the MaxVoterWeightRecord account.
///
/// Convention used by all SPL Governance voter-weight plugins:
///   seeds: `[realm_pubkey, b"max-voter-weight-record", governing_token_mint_pubkey]`
pub const MAX_VOTER_WEIGHT_RECORD_SEED: &[u8] = b"max-voter-weight-record";
