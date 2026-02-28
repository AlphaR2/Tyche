use pinocchio::Address;
use bytemuck::{Pod, Zeroable};
use shank::ShankType;

/// Arguments for `UpdateCrankAuthority`.
///
/// Kept separate from `UpdateProtocolConfig` so crank rotation — a frequent
/// operational action — carries less risk surface than a full config update.

///
/// # TODO
///
/// Require `new_crank_authority` to co-sign the rotation.
/// This prevents rotating to a key you do not control.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, ShankType)]
pub struct UpdateCrankAuthorityArgs {
    pub new_crank_authority: Address,
}

impl UpdateCrankAuthorityArgs {
    pub const LEN: usize = core::mem::size_of::<UpdateCrankAuthorityArgs>();

    pub fn load(bytes: &[u8]) -> Result<&Self, pinocchio::error::ProgramError> {
        if bytes.len() != Self::LEN {
            return Err(pinocchio::error::ProgramError::InvalidInstructionData);
        }
        Ok(bytemuck::from_bytes::<Self>(bytes))
    }
}

const _: () = assert!(UpdateCrankAuthorityArgs::LEN == 32);
