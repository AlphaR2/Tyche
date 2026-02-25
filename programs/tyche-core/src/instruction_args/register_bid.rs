use bytemuck::{Pod, Zeroable};

/// Arguments for the `RegisterBid` instruction.
///
/// No caller-supplied data required — the bidder address and competition pubkey
/// are read directly from the account list. The instruction is purely account-driven.
///
/// The zero-sized struct is kept for consistency with the rest of the instruction
/// arg pattern so the entrypoint dispatcher can handle it uniformly.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct RegisterBidArgs;

impl RegisterBidArgs {
    pub const LEN: usize = core::mem::size_of::<Self>();
}
