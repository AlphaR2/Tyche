use bytemuck::{Pod, Zeroable};

/// Arguments for the  instruction.
///
/// No caller input required. The processor validates phase and
///  directly from .
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct CancelCompetitionArgs;

impl CancelCompetitionArgs {
    pub const LEN: usize = core::mem::size_of::<Self>();
}
