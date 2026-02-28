use bytemuck::{Pod, Zeroable};
use pinocchio::{Address, error::ProgramError};

/// Arguments for the `CreateRegistrar` instruction.
///
/// Immediately follows the 8-byte discriminator in the instruction data.
/// All pubkey fields stored as `[u8; 32]` for `Pod` / `Zeroable` compatibility.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct CreateRegistrarArgs {
    /// The `spl-governance` program ID that owns the realm.
    pub governance_program_id: Address,

    /// The competition (CompetitionState PDA) this governance instance is scoped to.
    /// Only EscrowVault accounts whose `competition` field matches this pubkey
    /// contribute voter weight.
    pub competition: Address,

    /// The `tyche-escrow` program ID.
    /// Used to verify that EscrowVault accounts are genuinely owned by the
    /// escrow program, preventing spoofed vault accounts from inflating vote weight.
    pub tyche_escrow_program: Address,
}

impl CreateRegistrarArgs {
    pub const LEN: usize = core::mem::size_of::<CreateRegistrarArgs>();

    pub fn load(bytes: &[u8]) -> Result<&Self, ProgramError> {
        if bytes.len() != Self::LEN {
            return Err(ProgramError::InvalidInstructionData);
        }
        Ok(bytemuck::from_bytes::<Self>(bytes))
    }
}
