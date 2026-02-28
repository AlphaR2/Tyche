use pinocchio::Address;
use bytemuck::{Pod, Zeroable};
use shank::ShankAccount;

/// Singleton governance account for `tyche-core`.
///
/// Initialized once at deployment via `InitializeProtocolConfig`. Controls
/// operational parameters that all competition processors read at runtime.

///
/// # TODOs
///
/// - Add `pause_flags: u16` to gate individual instruction families.
/// - Add `pending_authority: Address` for two-step authority transfer.
/// - Add timelock fields for sensitive parameter changes.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, ShankAccount)]
pub struct ProtocolConfig {
   
    pub discriminator:       [u8; 8],
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
    pub bump:                u8,
    pub _pad3:               [u8; 7],
}

impl ProtocolConfig {
    pub const LEN: usize = core::mem::size_of::<ProtocolConfig>();
}

// Compile-time size assertion: must be exactly 168 bytes.
const _: () = assert!(ProtocolConfig::LEN == 168);
