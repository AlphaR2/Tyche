use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use solana_system_interface::program::ID as system_program;
use bytemuck::bytes_of;
use pinocchio::Address;
use tyche_common::seeds::COMPETITION_SEED;
use crate::{
    discriminator::CREATE_COMPETITION,
    instruction_args::create_competition::CreateCompetitionArgs,
};
use super::initialize_protocol_config::derive_protocol_config_pda;

/// Derives the `CompetitionState` PDA for a given authority and competition id.
pub fn derive_competition_pda(authority: &Pubkey, id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[COMPETITION_SEED, authority.as_ref(), id.as_ref()],
        &Pubkey::from(*crate::ID.as_array()),
    )
}

/// Builds a `CreateCompetition` instruction.
///
/// | # | Account         | Writable | Signer |
/// |---|-----------------|----------|--------|
/// | 0 | competition     | yes      | no     |
/// | 1 | authority       | no       | yes    |
/// | 2 | payer           | yes      | yes    |
/// | 3 | system_program  | no       | no     |
/// | 4 | protocol_config | no       | no     |
pub fn build_create_competition(
    authority:            &Pubkey,
    payer:                &Pubkey,
    id:                   [u8; 32],
    asset_type:           u8,
    start_time:           i64,
    duration_secs:        i64,
    soft_close_window:    i64,
    soft_close_extension: i64,
    max_soft_closes:      u8,
    reserve_price:        u64,
) -> (Instruction, Pubkey) {
    let program_id           = Pubkey::from(*crate::ID.as_array());
    let (competition, _)     = derive_competition_pda(authority, &id);
    let (protocol_config, _) = derive_protocol_config_pda();

    let args = CreateCompetitionArgs {
        id:                   Address::new_from_array(id),
        asset_type,
        _pad:                 [0u8; 7],
        start_time,
        duration_secs,
        soft_close_window,
        soft_close_extension,
        max_soft_closes,
        _pad2:                [0u8; 7],
        reserve_price,
    };

    let mut data = CREATE_COMPETITION.to_vec();
    data.extend_from_slice(bytes_of(&args));

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(competition, false),
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(system_program, false),
            AccountMeta::new_readonly(protocol_config, false),
        ],
        data,
    };

    (ix, competition)
}
