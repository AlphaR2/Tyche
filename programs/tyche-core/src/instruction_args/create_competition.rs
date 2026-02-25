use pinocchio::Address;
use bytemuck::{Pod, Zeroable};

/// Arguments for the `CreateCompetition` instruction.
///
/// Supplied by the competition creator. The processor derives all PDA
/// addresses and reads clock state from the sysvar. Only values the
/// creator controls are passed here.

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct CreateCompetitionArgs {
    pub id:                   Address,  
    pub asset_type:           u8,       
    pub _pad:                 [u8; 7],   
    pub start_time:           i64,      
    pub duration_secs:        i64,      
    pub soft_close_window:    i64,      
    pub soft_close_extension: i64,      
    pub max_soft_closes:      u8,        
    pub _pad2:                [u8; 7],  
    pub reserve_price:        u64,      
}

impl CreateCompetitionArgs {
    pub const LEN: usize = core::mem::size_of::<CreateCompetitionArgs>();

    pub fn load(bytes: &[u8]) -> Result<&Self, pinocchio::error::ProgramError> {
        if bytes.len() != Self::LEN {
            return Err(pinocchio::error::ProgramError::InvalidInstructionData);
        }
        Ok(bytemuck::from_bytes::<Self>(bytes))
    }
}