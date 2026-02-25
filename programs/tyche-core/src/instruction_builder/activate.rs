use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use solana_system_interface::program::ID as system_program;
use bytemuck::bytes_of;
use crate::{
    discriminator::ACTIVATE_COMPETITION,
    instruction_args::activate::ActivateCompetitionArgs,
};

/// Builds an `ActivateCompetition` instruction.
///
/// Transitions a `Scheduled` competition to `Active`, computes `end_time`,
/// creates/delegates the ACL permission account, and delegates `CompetitionState`
/// to the MagicBlock PER.
///
/// The delegation-related PDAs (`permission`, `delegation_buffer`,
/// `delegation_record`, `delegation_metadata`) should be derived using the
/// ephemeral-rollups SDK before calling this function.
///
/// # Accounts
/// 0.  `competition`         — writable (delegated to PER after this call)
/// 1.  `authority`           — readonly signer (must match `state.authority`)
/// 2.  `payer`               — writable signer (funds permission rent)
/// 3.  `permission`          — writable (ACL permission PDA)
/// 4.  `delegation_buffer`   — writable (delegation program PDA)
/// 5.  `delegation_record`   — writable (delegation program PDA)
/// 6.  `delegation_metadata` — writable (delegation program PDA)
/// 7.  `delegation_program`  — readonly
/// 8.  `permission_program`  — readonly
/// 9.  `system_program`      — readonly
/// 10. `validator`           — readonly (target TEE validator)
#[allow(clippy::too_many_arguments)]
pub fn build_activate_competition(
    competition:         &Pubkey,
    authority:           &Pubkey,
    payer:               &Pubkey,
    permission:          &Pubkey,
    delegation_buffer:   &Pubkey,
    delegation_record:   &Pubkey,
    delegation_metadata: &Pubkey,
    delegation_program:  &Pubkey,
    permission_program:  &Pubkey,
    validator:           &Pubkey,
    args:                ActivateCompetitionArgs,
) -> Instruction {
    let program_id = Pubkey::from(*crate::ID.as_array());

    let mut data = ACTIVATE_COMPETITION.to_vec();
    data.extend_from_slice(bytes_of(&args));

    let sp = system_program;

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(*competition, false),
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new(*payer, true),
            AccountMeta::new(*permission, false),
            AccountMeta::new(*delegation_buffer, false),
            AccountMeta::new(*delegation_record, false),
            AccountMeta::new(*delegation_metadata, false),
            AccountMeta::new_readonly(*delegation_program, false),
            AccountMeta::new_readonly(*permission_program, false),
            AccountMeta::new_readonly(sp, false),
            AccountMeta::new_readonly(*validator, false),
            // Trailing slot — required by the processor's exact-length account pattern.
            AccountMeta::new_readonly(sp, false),
        ],
        data,
    }
}
