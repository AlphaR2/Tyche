/// SPL Governance voter-weight addin — MaxVoterWeightRecord account.
///
/// Account layout (Borsh-compatible, 97 bytes when expiry = Some):

/// Maximum account allocation for a MaxVoterWeightRecord.
pub const MAX_VOTER_WEIGHT_RECORD_SIZE: usize = 97;

/// Write a Borsh-encoded `MaxVoterWeightRecord` into `data`.
///
/// `data` must be at least [`MAX_VOTER_WEIGHT_RECORD_SIZE`] bytes long.
#[inline]
pub fn write_max_voter_weight_record(
    data:             &mut [u8],
    discriminator:    &[u8; 8],
    realm:            &[u8; 32],
    mint:             &[u8; 32],
    max_voter_weight: u64,
    expiry:           Option<u64>,
) {
    debug_assert!(data.len() >= MAX_VOTER_WEIGHT_RECORD_SIZE);

    let mut c: usize = 0;

    // account_discriminator [u8; 8]
    data[c..c + 8].copy_from_slice(discriminator);         c += 8;
    // realm [u8; 32]
    data[c..c + 32].copy_from_slice(realm);                c += 32;
    // governing_token_mint [u8; 32]
    data[c..c + 32].copy_from_slice(mint);                 c += 32;
    // max_voter_weight u64 LE
    data[c..c + 8].copy_from_slice(&max_voter_weight.to_le_bytes()); c += 8;

    // max_voter_weight_expiry Option<u64>
    match expiry {
        None       => { data[c] = 0; c += 1; }
        Some(slot) => {
            data[c] = 1; c += 1;
            data[c..c + 8].copy_from_slice(&slot.to_le_bytes()); c += 8;
        }
    }

    // reserved [u8; 8]
    data[c..c + 8].copy_from_slice(&[0u8; 8]);
}
