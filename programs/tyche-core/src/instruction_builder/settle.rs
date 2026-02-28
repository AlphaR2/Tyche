use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use bytemuck::bytes_of;
use pinocchio::Address;
use crate::{
    discriminator::SETTLE_COMPETITION,
    instruction_args::settle::SettleCompetitionArgs,
};
use super::initialize_protocol_config::derive_protocol_config_pda;

/// Builds a `SettleCompetition` instruction.
///
/// | # | Account                  | Writable | Signer |
/// |---|--------------------------|----------|--------|
/// | 0 | competition              | yes      | no     |
/// | 1 | crank                    | no       | yes    |
/// | 2 | delegation_record        | no       | no     |
/// | 3 | protocol_config          | no       | no     |
/// | 4 | winner_participant_record | yes      | no     |
///
/// Pass `winner = [0u8; 32]` when there is no winner.
/// When `winner` is non-zero tyche-core writes `IS_WINNER` to
/// `winner_participant_record` on behalf of the calling vertical.
pub fn build_settle_competition(
    competition:               &Pubkey,
    crank:                     &Pubkey,
    delegation_record:         &Pubkey,
    winner_participant_record: &Pubkey,
    settlement_ref:            [u8; 32],
    winner:                    [u8; 32],
) -> Instruction {
    let program_id           = Pubkey::from(*crate::ID.as_array());
    let (protocol_config, _) = derive_protocol_config_pda();

    let args = SettleCompetitionArgs {
        settlement_ref: Address::new_from_array(settlement_ref),
        winner:         Address::new_from_array(winner),
    };

    let mut data = SETTLE_COMPETITION.to_vec();
    data.extend_from_slice(bytes_of(&args));

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(*competition, false),
            AccountMeta::new_readonly(*crank, true),
            AccountMeta::new_readonly(*delegation_record, false),
            AccountMeta::new_readonly(protocol_config, false),
            AccountMeta::new(*winner_participant_record, false),
        ],
        data,
    }
}
