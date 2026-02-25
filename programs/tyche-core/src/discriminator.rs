/// Anchor-compatible discriminators for `tyche-core`.
///
/// Each value is the first 8 bytes of `SHA256("global:<instruction_name>")` for
/// instructions and `SHA256("account:<AccountName>")` for accounts, matching the
/// Anchor convention exactly. Any Anchor client, CPI caller, or Codama-generated
/// TypeScript SDK can derive these values independently from the instruction or
/// account name — no knowledge of internal numbering required.
///
/// Values were computed by `src/bin/compute_discriminators.rs`.
/// Do not edit manually — re-run the binary if instruction or account names change.


// ── Instruction discriminators 
pub const CREATE_COMPETITION:   [u8; 8] = [110, 212, 234, 212, 118, 128, 158, 244];
pub const ACTIVATE_COMPETITION: [u8; 8] = [153, 105, 130, 88, 198, 208, 30, 118];
pub const EXTEND_COMPETITION:   [u8; 8] = [9, 0, 18, 247, 115, 18, 176, 115];
pub const CLOSE_COMPETITION:    [u8; 8] = [49, 166, 127, 67, 43, 108, 132, 96];
pub const SETTLE_COMPETITION:   [u8; 8] = [83, 121, 9, 141, 170, 133, 230, 151];
pub const CANCEL_COMPETITION:   [u8; 8] = [62, 4, 198, 98, 200, 41, 255, 72];
pub const REGISTER_BID:         [u8; 8] = [26, 173, 93, 67, 171, 107, 118, 212];

// ── Account discriminators 
pub const COMPETITION_STATE:    [u8; 8] = [92, 143, 28, 37, 251, 106, 9, 146];
pub const PARTICIPANT_RECORD:   [u8; 8] = [106, 52, 124, 24, 80, 173, 194, 4];
