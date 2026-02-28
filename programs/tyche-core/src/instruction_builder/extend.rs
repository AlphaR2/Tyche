use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use crate::discriminator::EXTEND_COMPETITION;
use super::initialize_protocol_config::derive_protocol_config_pda;

/// Builds an `ExtendCompetition` instruction.
///
/// | # | Account         | Writable | Signer |
/// |---|-----------------|----------|--------|
/// | 0 | competition     | yes      | no     |
/// | 1 | crank           | no       | yes    |
/// | 2 | magic_context   | yes      | no     |
/// | 3 | magic_program   | no       | no     |
/// | 4 | protocol_config | no       | no     |
pub fn build_extend_competition(
    competition:   &Pubkey,
    crank:         &Pubkey,
    magic_context: &Pubkey,
    magic_program: &Pubkey,
) -> Instruction {
    let program_id           = Pubkey::from(*crate::ID.as_array());
    let (protocol_config, _) = derive_protocol_config_pda();

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(*competition, false),
            AccountMeta::new_readonly(*crank, true),
            AccountMeta::new(*magic_context, false),
            AccountMeta::new_readonly(*magic_program, false),
            AccountMeta::new_readonly(protocol_config, false),
        ],
        data: EXTEND_COMPETITION.to_vec(),
    }
}
