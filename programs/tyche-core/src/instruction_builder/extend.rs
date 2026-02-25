use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use crate::discriminator::EXTEND_COMPETITION;

/// Builds an `ExtendCompetition` instruction.
///
/// Called by the protocol crank when a bid lands within the soft-close window.
/// Increments `end_time` by `soft_close_extension` and commits the updated state
/// to mainnet immediately via `commit_accounts`.
///
/// Only the protocol crank (`TYCHE_CRANK_PUBKEY`) may sign this instruction.
///
/// # Accounts
/// 0. `competition`  — writable (delegated to PER)
/// 1. `crank`        — readonly signer (protocol crank only)
/// 2. `magic_context`— writable (MagicBlock PER context)
/// 3. `magic_program`— readonly (MagicBlock program)
pub fn build_extend_competition(
    competition:  &Pubkey,
    crank:        &Pubkey,
    magic_context: &Pubkey,
    magic_program: &Pubkey,
) -> Instruction {
    let program_id = Pubkey::from(*crate::ID.as_array());

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(*competition, false),
            AccountMeta::new_readonly(*crank, true),
            AccountMeta::new(*magic_context, false),
            AccountMeta::new_readonly(*magic_program, false),
        ],
        data: EXTEND_COMPETITION.to_vec(),
    }
}
