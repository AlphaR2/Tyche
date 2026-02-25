use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use crate::discriminator::CLOSE_COMPETITION;

/// Builds a `CloseCompetition` instruction.
///
/// Called by the protocol crank after `clock.unix_timestamp >= end_time`.
/// Transitions phase from `Active` → `Settling` and triggers
/// `commit_and_undelegate_accounts` to return both `competition` and `permission`
/// to mainnet.
///
/// Only the protocol crank (`TYCHE_CRANK_PUBKEY`) may sign this instruction.
///
/// # Accounts
/// 0. `competition`  — writable (delegated to PER)
/// 1. `crank`        — readonly signer (protocol crank only)
/// 2. `permission`   — writable (ACL permission PDA; undelegated alongside competition)
/// 3. `magic_context`— writable (MagicBlock PER context)
/// 4. `magic_program`— readonly (MagicBlock program)
pub fn build_close_competition(
    competition:   &Pubkey,
    crank:         &Pubkey,
    permission:    &Pubkey,
    magic_context: &Pubkey,
    magic_program: &Pubkey,
) -> Instruction {
    let program_id = Pubkey::from(*crate::ID.as_array());

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(*competition, false),
            AccountMeta::new_readonly(*crank, true),
            AccountMeta::new(*permission, false),
            AccountMeta::new(*magic_context, false),
            AccountMeta::new_readonly(*magic_program, false),
        ],
        data: CLOSE_COMPETITION.to_vec(),
    }
}
