/// Anchor-compatible discriminators for `tyche-voter-weight-plugin`.
///
/// Each value is the first 8 bytes of `SHA256("global:<instruction_name>")` for
/// instructions and `SHA256("account:<AccountName>")` for accounts, matching the
/// Anchor convention exactly.
///
/// # How to compute
///
/// Run the `compute_discriminators` binary from the workspace root:
///
/// ```sh
/// cargo run --manifest-path programs/tyche-voter-weight-plugin/Cargo.toml \
///     --bin compute_discriminators
/// ```
///
/// Copy the printed constants into this file.
///
/// The `VOTER_WEIGHT_RECORD` value is fixed by the spl-governance-addin-api /
/// Anchor convention and must equal SHA256("account:VoterWeightRecord")[0..8].
/// SPL Governance checks this discriminator when reading the record at vote time.


// ── Instruction discriminators ────────────────────────────────────────────────
pub const CREATE_REGISTRAR:              [u8; 8] = [132, 235, 36, 49, 139, 66, 202, 69];
pub const CREATE_VOTER_WEIGHT_RECORD:    [u8; 8] = [184, 249, 133, 178, 88, 152, 250, 186];
pub const UPDATE_VOTER_WEIGHT_RECORD:    [u8; 8] = [45, 185, 3, 36, 109, 190, 115, 169];
pub const UPDATE_MAX_VOTER_WEIGHT_RECORD:[u8; 8] = [103, 175, 201, 251, 2, 9, 251, 179];

// ── Account discriminators ────────────────────────────────────────────────────
pub const REGISTRAR:               [u8; 8] = [193, 202, 205, 51, 78, 168, 150, 128];
pub const MAX_VOTER_WEIGHT_RECORD: [u8; 8] = [157, 95, 242, 151, 16, 98, 26, 118];

/// Fixed by the Anchor / spl-governance-addin-api convention.
/// = SHA256("account:VoterWeightRecord")[0..8]  = 0x2ef99b4b99f87409
/// SPL Governance on-chain program reads this value verbatim — do not change.
pub const VOTER_WEIGHT_RECORD:     [u8; 8] = [46, 249, 155, 75, 153, 248, 116, 9];
