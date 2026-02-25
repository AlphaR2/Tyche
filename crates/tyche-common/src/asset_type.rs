use num_enum::TryFromPrimitive;

/// The on-chain asset representation used in a Tyche competition.
///
/// Stored as a `u8` in `CompetitionState::asset_type`.
/// All Tyche competition assets are either NFTs (Metaplex standard or
/// compressed) or fungible tokens (SPL Token or Token-2022). In-game
/// items, collectibles, and other digital goods map to one of these two
/// primitives at the protocol level.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum AssetType {
    Nft   = 0,
    Token = 1,
}