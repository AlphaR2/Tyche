use pinocchio::Address;
use bytemuck::{Pod, Zeroable};
use shank::ShankAccount;

/// Per-bidder sealed bid record for a single Tyche auction.
///
/// Owned by `tyche-auction`. Created inside the PER on the bidder's first `PlaceBid`
/// call for a given competition. Updated on repeat bids from the same address.
/// Closed by `CloseBidRecord`, which is called via CPI from `tyche-escrow::Refund`
/// when a losing bidder pulls their refund — returning the rent to the bidder.
///
/// # PDA
///
/// Seeds: `[BID_SEED, competition_pubkey, bidder_pubkey]`
///
///
/// # Lifecycle
///
/// Created on the bidder's first `PlaceBid` (inside PER).
/// `amount` updated on every subsequent bid from the same address.
/// Closed by `CloseBidRecord` after settlement — rent returned to bidder.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, ShankAccount)]
pub struct BidRecord {
    /// Anchor-compatible discriminator: SHA256("account:BidRecord")[0..8].
    pub discriminator: [u8; 8],
    /// The competition this bid belongs to.
    pub competition:   Address,
    /// The bidder who owns this record.
    pub bidder:        Address,
    /// The bidder's latest bid amount in lamports.
    pub amount:        u64,
    /// Canonical PDA bump stored at creation.
    pub bump:          u8,
    pub _pad:          [u8; 7],
}

impl BidRecord {
    pub const LEN: usize = core::mem::size_of::<BidRecord>();
}

// Compile-time size assertion: must be exactly 88 bytes.
const _: () = assert!(BidRecord::LEN == 88);
