use bytemuck::{Pod, Zeroable};
use shank::ShankType;

/// Arguments for the `ActivateCompetition` instruction.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, ShankType)]
pub struct ActivateCompetitionArgs {
    /// How often the PER auto-commits `CompetitionState` to mainnet during
    /// the active phase, in milliseconds.
    /// Tune per vertical: auctions ~1000ms, prediction markets ~60000ms, liquidity batches ~200ms.
    pub commit_frequency_ms: u32,
}

impl ActivateCompetitionArgs {
    pub const LEN: usize = core::mem::size_of::<Self>();

    pub fn load(bytes: &[u8]) -> Result<&Self, pinocchio::error::ProgramError> {
        if bytes.len() != Self::LEN {
            return Err(pinocchio::error::ProgramError::InvalidInstructionData);
        }
        Ok(bytemuck::from_bytes::<Self>(bytes))
    }
}
