use pinocchio::Address;
use bytemuck::{Pod, Zeroable};
use shank::ShankType;

/// Arguments for `InitializeProtocolConfig`.
///
/// All addresses that cannot be changed without a future two-step transfer
/// instruction (post-hackathon) are set here at initialization.
/// `fee_basis_points` is set here at initialization and can be updated later by the authority.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, ShankType)]
pub struct InitializeProtocolConfigArgs {
    pub authority:           Address,
    pub emergency_authority: Address,
    pub treasury:            Address,
    pub crank_authority:     Address,
    pub fee_basis_points:    u16,
    pub _pad:                [u8; 2],
    pub max_soft_closes_cap: u8,
    pub _pad2:               [u8; 3],
    pub min_reserve_price:   u64,
    pub min_duration_secs:   i64,
}

impl InitializeProtocolConfigArgs {
    pub const LEN: usize = core::mem::size_of::<InitializeProtocolConfigArgs>();

    pub fn load(bytes: &[u8]) -> Result<&Self, pinocchio::error::ProgramError> {
        if bytes.len() != Self::LEN {
            return Err(pinocchio::error::ProgramError::InvalidInstructionData);
        }
        Ok(bytemuck::from_bytes::<Self>(bytes))
    }
}

const _: () = assert!(InitializeProtocolConfigArgs::LEN == 152);
