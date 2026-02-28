use pinocchio::Address;
use bytemuck::{Pod, Zeroable};
use shank::ShankType;

/// Arguments for `UpdateProtocolConfig`.
///
/// Updates operational parameters. `authority` and `emergency_authority` are
/// intentionally excluded — authority transfer is a post-hackathon feature.
///
/// # Layout (56 bytes, 8-byte aligned)
///TODOs
///
/// - Enforce 48hr timelock on `new_fee_basis_points` changes.
/// - Enforce 48hr timelock on `new_min_reserve_price` changes.
/// - Emit change log for off-chain indexers.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, ShankType)]
pub struct UpdateProtocolConfigArgs {
    pub new_treasury:            Address,
    pub new_fee_basis_points:    u16,
    pub new_max_soft_closes_cap: u8,
    pub _pad:                    [u8; 5],
    pub new_min_reserve_price:   u64,
    pub new_min_duration_secs:   i64,
}

impl UpdateProtocolConfigArgs {
    pub const LEN: usize = core::mem::size_of::<UpdateProtocolConfigArgs>();

    pub fn load(bytes: &[u8]) -> Result<&Self, pinocchio::error::ProgramError> {
        if bytes.len() != Self::LEN {
            return Err(pinocchio::error::ProgramError::InvalidInstructionData);
        }
        Ok(bytemuck::from_bytes::<Self>(bytes))
    }
}

const _: () = assert!(UpdateProtocolConfigArgs::LEN == 56);
