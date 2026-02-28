use pinocchio::Address;
use bytemuck::{Pod, Zeroable};
use shank::ShankType;

/// Arguments for the `SettleCompetition` instruction
///
/// Passed by the vertical program (e.g. `tyche-auction`) via CPI after
/// undelegation completes. `settlement_ref` points to the vertical's own
/// result account — `AuctionState`, `PredictionState`, etc. `tyche-core`
/// stores the reference without interpreting it, keeping the state machine
/// vertical-agnostic.
///
/// `winner` is the pubkey of the winning participant. Pass `Address::default()`
/// (all-zeros) when there is no winner (e.g. auction with zero bids).
/// `tyche-core` writes `IS_WINNER` to the `winner_participant_record` account
/// when `winner` is non-zero, avoiding an ownership-violation cross-CPI.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, ShankType)]
pub struct SettleCompetitionArgs {
    pub settlement_ref: Address,
    pub winner:         Address,
}

impl SettleCompetitionArgs {
    pub const LEN: usize = core::mem::size_of::<Self>();

    pub fn load(bytes: &[u8]) -> Result<&Self, pinocchio::error::ProgramError> {
        if bytes.len() != Self::LEN {
            return Err(pinocchio::error::ProgramError::InvalidInstructionData);
        }
        Ok(bytemuck::from_bytes::<Self>(bytes))
    }
}
