use pinocchio::Address;
use bytemuck::{Pod, Zeroable};
use shank::ShankAccount;

/// Per-bidder SOL custody account for a single Tyche CEE competition.
///
/// Owned by `tyche-escrow`. Created on the bidder's first `Deposit` call for a
/// given competition. Topped up on subsequent `Deposit` calls. Closed (lamports
/// drained, data zeroed) by either `Release` or `Refund`.
///
/// # Lamport accounting
///
/// The vault always holds `rent-exempt reserve + cumulative bid amount` lamports.
/// `amount` tracks **only the cumulative bid** — never the rent portion.
///
/// ```text
/// vault.lamports() == rent_exempt_reserve + vault.amount
/// ```
///
/// On `Release`: `vault.amount` → competition authority; remaining → depositor.
/// On `Refund`:  all `vault.lamports()` → depositor (bid + rent, full refund).
///
/// # PDA
///
/// Seeds: `[VAULT_SEED, competition_pubkey, depositor_pubkey]`
/// 
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, ShankAccount)]
pub struct EscrowVault {
    /// Anchor-compatible discriminator — `sha256("account:EscrowVault")[0..8]`.
    pub discriminator: [u8; 8],
    /// The competition this vault is locked to.
    pub competition:   Address,
    /// The bidder who owns this vault.
    pub depositor:     Address,
    /// Cumulative bid amount in lamports — does not include rent reserve.
    pub amount:        u64,
    /// Canonical PDA bump stored to avoid re-deriving.
    pub bump:          u8,
    pub _pad:          [u8; 7],
}

impl EscrowVault {
    pub const LEN: usize = core::mem::size_of::<EscrowVault>();
}
