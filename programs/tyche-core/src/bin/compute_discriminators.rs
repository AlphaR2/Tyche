use sha2::{Digest, Sha256};

fn discriminator(preimage: &str) -> [u8; 8] {
    let hash = Sha256::digest(preimage.as_bytes());
    hash[0..8].try_into().unwrap()
}

fn main() {
    println!("// ── Instruction discriminators ──────────────────────────────────────────────");
    println!("pub const CREATE_COMPETITION:   [u8; 8] = {:?};", discriminator("global:create_competition"));
    println!("pub const ACTIVATE_COMPETITION: [u8; 8] = {:?};", discriminator("global:activate_competition"));
    println!("pub const EXTEND_COMPETITION:   [u8; 8] = {:?};", discriminator("global:extend_competition"));
    println!("pub const CLOSE_COMPETITION:    [u8; 8] = {:?};", discriminator("global:close_competition"));
    println!("pub const SETTLE_COMPETITION:   [u8; 8] = {:?};", discriminator("global:settle_competition"));
    println!("pub const CANCEL_COMPETITION:   [u8; 8] = {:?};", discriminator("global:cancel_competition"));
    println!("pub const REGISTER_BID:         [u8; 8] = {:?};", discriminator("global:register_bid"));
    println!();
    println!("// ── Account discriminators ───────────────────────────────────────────────────");
    println!("pub const COMPETITION_STATE:    [u8; 8] = {:?};", discriminator("account:CompetitionState"));
    println!("pub const PARTICIPANT_RECORD:   [u8; 8] = {:?};", discriminator("account:ParticipantRecord"));
}
