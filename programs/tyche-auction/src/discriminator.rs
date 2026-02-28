/// Anchor-compatible discriminators for `tyche-auction`.
///
/// Each value is the first 8 bytes of `SHA256("global:<instruction_name>")` for
/// instructions and `SHA256("account:<AccountName>")` for accounts, matching the
/// Anchor convention exactly. Any Anchor client, CPI caller, or Codama-generated
/// TypeScript SDK can derive these values independently from the name alone.
///
/// Values computed via SHA256 of the canonical preimage strings.
/// Do not edit manually — re-run the compute binary if names change.


// ── Instruction discriminators 
pub const CREATE_AUCTION:    [u8; 8] = [234, 6, 201, 246, 47, 219, 176, 107];
pub const ACTIVATE_AUCTION:  [u8; 8] = [212, 24, 210, 7, 183, 147, 66, 109];
pub const PLACE_BID:         [u8; 8] = [238, 77, 148, 91, 200, 151, 92, 146];
pub const FINALIZE_AUCTION:  [u8; 8] = [220, 209, 175, 193, 57, 132, 241, 168];
pub const CANCEL_AUCTION:    [u8; 8] = [156, 43, 197, 110, 218, 105, 143, 182];
pub const CLOSE_BID_RECORD:  [u8; 8] = [191, 178, 243, 199, 31, 166, 172, 200];

// ── Account discriminators 
pub const AUCTION_STATE: [u8; 8] = [252, 227, 205, 147, 72, 64, 250, 126];
pub const BID_RECORD:    [u8; 8] = [135, 88, 154, 228, 192, 219, 168, 168];
