use shank::ShankType;

/// Arguments for the `FinalizeAuction` instruction.
///
/// No caller-supplied args — winner and final_amount read directly from `AuctionState`.
#[derive(ShankType)]
pub struct FinalizeAuctionArgs {}


