use bytemuck::{Pod, Zeroable};

/// Arguments for the  instruction.
///
/// No caller input required. The processor reads  from
///  and validates against .
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct CloseCompetitionArgs;

impl CloseCompetitionArgs {
    pub const LEN: usize = core::mem::size_of::<Self>();
}
