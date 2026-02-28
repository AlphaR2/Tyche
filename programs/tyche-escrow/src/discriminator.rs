/// Anchor-compatible discriminators for `tyche-escrow`.
///
/// Each value is the first 8 bytes of `SHA256("global:<instruction_name>")` for
/// instructions and `SHA256("account:<AccountName>")` for accounts, matching the
/// Anchor convention exactly.
///
/// Values were computed by extending the workspace `compute_discriminators` binary.
/// Do not edit manually — re-run the binary if instruction or account names change.


// ── Instruction discriminators ────────────────────────────────────────────────
pub const DEPOSIT: [u8; 8] = [242, 35, 198, 137, 82, 225, 242, 182];
pub const RELEASE: [u8; 8] = [253, 249, 15, 206, 28, 127, 193, 241];
pub const REFUND:  [u8; 8] = [2, 96, 183, 251, 63, 208, 46, 46];

// ── Account discriminators ────────────────────────────────────────────────────
pub const ESCROW_VAULT: [u8; 8] = [54, 84, 41, 149, 160, 181, 85, 114];
