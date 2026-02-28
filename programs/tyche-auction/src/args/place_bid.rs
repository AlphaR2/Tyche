use bytemuck::{Pod, Zeroable};
use shank::ShankType;

/// Arguments for the `PlaceBid` instruction.
///
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, ShankType)]
pub struct PlaceBidArgs {
    /// Bid amount in lamports. Must satisfy reserve_price and min_bid_increment.
    pub amount: u64,
}

impl PlaceBidArgs {
    pub const LEN: usize = core::mem::size_of::<Self>();

    pub fn load(bytes: &[u8]) -> Result<&Self, pinocchio::error::ProgramError> {
        if bytes.len() != Self::LEN {
            return Err(pinocchio::error::ProgramError::InvalidInstructionData);
        }
        Ok(bytemuck::from_bytes::<Self>(bytes))
    }
}

const _: () = assert!(PlaceBidArgs::LEN == 8);
