use bytemuck::{Pod, Zeroable};

/// Arguments for the  instruction.
///
/// No caller input required. The processor reads ,
/// , and  directly from
/// .
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ExtendCompetitionArgs;

impl ExtendCompetitionArgs {
    pub const LEN: usize = core::mem::size_of::<Self>();
}
