use bytemuck::{Pod, Zeroable};
use pinocchio::Address;
use shank::ShankAccount;

/// Registrar PDA — one per (realm, governing_token_mint) pair.
///
/// Owned by `tyche-voter-weight-plugin`. Created once by the realm authority
/// via `CreateRegistrar`. Stores the plugin's configuration.
///
/// # PDA
///
/// Seeds: `[realm_pubkey, b"registrar", governing_token_mint_pubkey]`
///
/// # Layout
///
/// `#[repr(C)]` + `Pod` — zero-copy read/write via bytemuck.
/// All pubkey fields use `[u8; 32]` which is always `Pod + Zeroable`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, ShankAccount)]
pub struct Registrar {
    /// 8-byte discriminator: SHA256("account:Registrar")[0..8]
    pub discriminator: [u8; 8],

    // ── Required by the Realms voter-weight plugin interface 

    /// The `spl-governance` program ID this realm belongs to.
    pub governance_program_id: Address,

    /// The Realm this registrar is configured for.
    pub realm: Address,

    /// Community mint of the Realm.
    pub governing_token_mint: Address,

    /// Previous plugin in the voter-weight chain (all zeros = standalone).
    pub prev_plugin_program_id: Address,

    /// 1 = has a previous plugin in the chain, 0 = standalone.
    pub has_prev_plugin: u8,
    pub _pad0: [u8; 7],

    // ── Tyche-specific configuration 

    /// The `tyche-escrow` program ID.
    /// Used to verify that `EscrowVault` accounts are genuinely owned by the escrow program.
    pub tyche_escrow_program_id: Address,

    /// The specific competition this governance instance is scoped to.
    pub competition: Address,

    /// Canonical PDA bump.
    pub bump: u8,
    pub _pad1: [u8; 7],

    /// Reserved for future use. Must be all zeros.
    pub reserved: [u8; 128],
}

impl Registrar {
    pub const LEN: usize = core::mem::size_of::<Registrar>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registrar_size() {
        // discriminator(8) + governance_program_id(32) + realm(32) + governing_token_mint(32)
        // + prev_plugin_program_id(32) + has_prev_plugin(1) + _pad0(7)
        // + tyche_escrow_program_id(32) + competition(32) + bump(1) + _pad1(7) + reserved(128)
        // = 8 + 32 + 32 + 32 + 32 + 8 + 32 + 32 + 8 + 128 = 344
        assert_eq!(Registrar::LEN, 344);
    }
}
