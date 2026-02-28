use pinocchio::Address;
use bytemuck::{Pod, Zeroable};
use shank::ShankType;

/// Arguments for the `CreateAuction` instruction.
///
/// Supplied by the seller. The processor derives PDA addresses.
///
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, ShankType)]
pub struct CreateAuctionArgs {
    /// The NFT or token mint being listed.
    pub asset_mint:        Address,
    /// Minimum lamports above `current_high_bid` required for each new bid.
    pub min_bid_increment: u64,
}

impl CreateAuctionArgs {
    pub const LEN: usize = core::mem::size_of::<Self>();

    pub fn load(bytes: &[u8]) -> Result<&Self, pinocchio::error::ProgramError> {
        if bytes.len() != Self::LEN {
            return Err(pinocchio::error::ProgramError::InvalidInstructionData);
        }
        Ok(bytemuck::from_bytes::<Self>(bytes))
    }
}

const _: () = assert!(CreateAuctionArgs::LEN == 40);
