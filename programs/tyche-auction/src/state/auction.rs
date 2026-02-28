use pinocchio::Address;
use bytemuck::{Pod, Zeroable};
use shank::ShankAccount;


/// On-chain state for a single Tyche sealed-bid auction.
///
/// Owned by `tyche-auction`. Linked to a `CompetitionState` in `tyche-core`
/// via `competition` field. Delegated to the MagicBlock PER alongside
/// `CompetitionState` during `ActivateAuction` — the `current_high_bid` and
/// `current_winner` fields are sealed inside the TEE while the account is delegated.
///
/// # PDA
///
/// Seeds: `[AUCTION_SEED, competition_pubkey]`
///
/// # Lifecycle
///
/// Created by `CreateAuction`. Delegated to PER by `ActivateAuction`.
/// Sealed fields (`current_high_bid`, `current_winner`) update on every `PlaceBid`
/// inside the TEE — unreadable on mainnet during the active phase.
/// Committed back to mainnet by the process-undelegation handler after settlement.
/// Closed by `CancelAuction` on the cancelled path.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, ShankAccount)]
pub struct AuctionState {
    pub discriminator:     [u8; 8],
    /// The `CompetitionState` this auction is linked to.
    pub competition:       Address,
    /// The seller — creates the auction, receives the winning bid via `tyche-escrow::Release`.
    pub authority:         Address,
    /// The NFT or token mint being auctioned.
    pub asset_mint:        Address,
    /// Minimum lamports a new bid must exceed the current high bid by.
    pub min_bid_increment: u64,
    /// Current highest bid in lamports. SEALED inside the PER TEE during active phase.
    pub current_high_bid:  u64,
    /// Current winning bidder. SEALED inside the PER TEE during active phase.
    /// Zero-initialized at creation; set on the first qualifying bid.
    pub current_winner:    Address,
    /// Total number of bids placed (not unique bidders).
    /// Incremented on every `PlaceBid` — even repeat bids from the same address.
    pub bid_count:         u32,
    /// Canonical PDA bump stored at creation to avoid re-derivation.
    pub bump:              u8,
    pub _pad:              [u8; 3],
}

impl AuctionState {
    pub const LEN: usize = core::mem::size_of::<AuctionState>();
}

// Compile-time size assertion: must be exactly 160 bytes.
const _: () = assert!(AuctionState::LEN == 160);
