use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use tyche_common::seeds::PARTICIPANT_SEED;
use crate::discriminator::REGISTER_BID;

use solana_system_interface::program::ID as system_program;

/// Derives the `ParticipantRecord` PDA for a given competition and bidder.
///
/// Seeds: `[PARTICIPANT_SEED, competition, bidder]`
pub fn derive_participant_record_pda(competition: &Pubkey, bidder: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[PARTICIPANT_SEED, competition.as_ref(), bidder.as_ref()],
        &Pubkey::from(*crate::ID.as_array()),
    )
}

/// Builds a `RegisterBid` instruction.
///
/// Intended to be called by vertical programs (e.g. `tyche-auction`) via CPI
/// from their `PlaceBid` instruction. `caller_program` must sign — CPI signer
/// propagation proves the vertical authorized this call.
///
/// On the bidder's first bid this creates a `ParticipantRecord` PDA and
/// increments `CompetitionState::participant_count`. On repeat bids it updates
/// `last_action` only.
///
/// Returns both the instruction and the derived `ParticipantRecord` pubkey so
/// callers can include it in their own account lists.
///
/// # Accounts
/// 0. `competition`       — writable
/// 1. `participant_record`— writable PDA (derived)
/// 2. `bidder`            — readonly signer
/// 3. `payer`             — writable signer (funds rent on first bid)
/// 4. `system_program`    — readonly
/// 5. `caller_program`    — readonly signer (CPI caller — proves vertical authorized the call)
pub fn build_register_bid(
    competition:    &Pubkey,
    bidder:         &Pubkey,
    payer:          &Pubkey,
    caller_program: &Pubkey,
) -> (Instruction, Pubkey) {
    let program_id                = Pubkey::from(*crate::ID.as_array());
    let (participant_record, _)   = derive_participant_record_pda(competition, bidder);

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(*competition, false),
            AccountMeta::new(participant_record, false),
            AccountMeta::new_readonly(*bidder, true),
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(system_program, false),
            AccountMeta::new_readonly(*caller_program, true),
        ],
        data: REGISTER_BID.to_vec(),
    };

    (ix, participant_record)
}
