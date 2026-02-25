use pinocchio::Address;
use shank::ShankAccount;
use bytemuck::{Pod, Zeroable};



/// Per-bidder record for a single Tyche CEE competition.
///
/// Owned by `tyche-core`. Created on the first bid from each address.
/// One record per `(competition, participant)` pair.
///
/// # PDA
///
/// Seeds: `[b"participant", competition_pubkey, participant_pubkey]`
///
/// # Lifecycle
///
/// Created by `PlaceBid` on the bidder's first bid.
/// `is_winner` is sealed inside the TEE during the active phase — set to
/// `IS_WINNER` only after `SettleCompetition` commits the final state.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, ShankAccount)]
pub struct ParticipantRecord {
    pub discriminator: [u8; 8],   
    pub competition:  Address,
    pub participant:  Address,
    pub is_winner:     u8,      
    pub bump:          u8,       
    pub _padding:      [u8; 6], // 6 bytes of padding to align the following i64 to an 8-byte boundary
    pub last_action:   i64,
}

impl ParticipantRecord {
    pub const LEN: usize = core::mem::size_of::<ParticipantRecord>();

    pub const IS_WINNER: u8 = 1;
    pub const NOT_WINNER: u8 = 0;
}