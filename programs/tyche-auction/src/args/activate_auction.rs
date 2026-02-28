use shank::ShankType;

/// Arguments for the `ActivateAuction` instruction.
///
/// No caller-supplied args — all inputs derived from accounts.
/// Instruction data is discriminator only (8 bytes).
#[derive(ShankType)]
pub struct ActivateAuctionArgs {}