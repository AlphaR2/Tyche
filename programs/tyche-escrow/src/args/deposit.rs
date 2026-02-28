use bytemuck::{Pod, Zeroable};
use shank::ShankType;

/// Arguments for the `Deposit` instruction.
///
/// The processor creates (or tops up) the `EscrowVault` PDA for the caller
/// and transfers `amount` lamports from the depositor to the vault.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, ShankType)]
pub struct DepositArgs {
    /// Bid amount in lamports to lock into the escrow vault.
    /// Must be non-zero. Accumulated in `EscrowVault::amount` across calls.
    pub amount: u64,
}

impl DepositArgs {
    pub const LEN: usize = core::mem::size_of::<DepositArgs>();

    pub fn load(bytes: &[u8]) -> Result<&Self, pinocchio::error::ProgramError> {
        if bytes.len() != Self::LEN {
            return Err(pinocchio::error::ProgramError::InvalidInstructionData);
        }
        Ok(bytemuck::from_bytes::<Self>(bytes))
    }
}
