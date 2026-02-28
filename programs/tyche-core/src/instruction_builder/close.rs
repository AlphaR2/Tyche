use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use crate::discriminator::CLOSE_COMPETITION;
use super::initialize_protocol_config::derive_protocol_config_pda;

/// Builds a `CloseCompetition` instruction.
///
/// | # | Account         | Writable | Signer |
/// |---|-----------------|----------|--------|
/// | 0 | competition     | yes      | no     |
/// | 1 | crank           | no       | yes    |
/// | 2 | permission      | yes      | no     |
/// | 3 | magic_context   | yes      | no     |
/// | 4 | magic_program   | no       | no     |
/// | 5 | protocol_config | no       | no     |
pub fn build_close_competition(
    competition:   &Pubkey,
    crank:         &Pubkey,
    permission:    &Pubkey,
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
            AccountMeta::new(*permission, false),
            AccountMeta::new(*magic_context, false),
            AccountMeta::new_readonly(*magic_program, false),
            AccountMeta::new_readonly(protocol_config, false),
        ],
        data: CLOSE_COMPETITION.to_vec(),
    }
}
