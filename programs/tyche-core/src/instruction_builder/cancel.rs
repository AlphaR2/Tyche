use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use crate::discriminator::CANCEL_COMPETITION;

/// Builds a `CancelCompetition` instruction.
///
/// Valid from two phases:
/// - `Scheduled` — pure state write on mainnet, no CPI. `permission`,
///   `magic_context`, `magic_program` are passed but untouched.
/// - `Active` with `participant_count == 0` — commits and undelegates both
///   `competition` and `permission` back to mainnet.
///
/// The competition `authority` must sign.
///
/// # Accounts
/// 0. `competition`  — writable
/// 1. `authority`    — readonly signer (must match `state.authority`)
/// 2. `permission`   — writable (ACL permission PDA; undelegated on Active path)
/// 3. `magic_context`— writable (MagicBlock PER context; unused on Scheduled path)
/// 4. `magic_program`— readonly (MagicBlock program; unused on Scheduled path)
pub fn build_cancel_competition(
    competition:   &Pubkey,
    authority:     &Pubkey,
    permission:    &Pubkey,
    magic_context: &Pubkey,
    magic_program: &Pubkey,
) -> Instruction {
    let program_id = Pubkey::from(*crate::ID.as_array());

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(*competition, false),
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new(*permission, false),
            AccountMeta::new(*magic_context, false),
            AccountMeta::new_readonly(*magic_program, false),
        ],
        data: CANCEL_COMPETITION.to_vec(),
    }
}
