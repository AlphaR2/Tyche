//! Compute Anchor-compatible 8-byte discriminators for `tyche-voter-weight-plugin`.
//!
//! Run from the workspace root:
//!
//! ```sh
//! cargo run --manifest-path programs/tyche-voter-weight-plugin/Cargo.toml \
//!     --bin compute_discriminators
//! ```
//!
//! Copy the printed output into `src/discriminator.rs`.

use sha2::{Digest, Sha256};

fn discriminator(preimage: &str) -> [u8; 8] {
    let hash = Sha256::digest(preimage.as_bytes());
    hash[0..8].try_into().unwrap()
}

fn main() {
    println!("// ── Instruction discriminators ──────────────────────────────────────────────");
    println!(
        "pub const CREATE_REGISTRAR:              [u8; 8] = {:?};",
        discriminator("global:create_registrar")
    );
    println!(
        "pub const CREATE_VOTER_WEIGHT_RECORD:    [u8; 8] = {:?};",
        discriminator("global:create_voter_weight_record")
    );
    println!(
        "pub const UPDATE_VOTER_WEIGHT_RECORD:    [u8; 8] = {:?};",
        discriminator("global:update_voter_weight_record")
    );
    println!(
        "pub const UPDATE_MAX_VOTER_WEIGHT_RECORD:[u8; 8] = {:?};",
        discriminator("global:update_max_voter_weight_record")
    );

    println!();
    println!("// ── Account discriminators ───────────────────────────────────────────────────");
    println!(
        "pub const REGISTRAR:               [u8; 8] = {:?};",
        discriminator("account:Registrar")
    );
    println!(
        "pub const MAX_VOTER_WEIGHT_RECORD: [u8; 8] = {:?};",
        discriminator("account:MaxVoterWeightRecord")
    );

    let vwr_disc = discriminator("account:VoterWeightRecord");
    println!(
        "pub const VOTER_WEIGHT_RECORD:     [u8; 8] = {:?};",
        vwr_disc
    );

    println!();
    println!("// VoterWeightRecord discriminator in hex (for cross-reference):");
    print!("// 0x");
    for b in vwr_disc {
        print!("{:02x}", b);
    }
    println!();
    println!(
        "// This value must match what SPL Governance expects when reading VoterWeightRecord accounts."
    );
}
