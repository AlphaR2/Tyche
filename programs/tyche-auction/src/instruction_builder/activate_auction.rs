use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use crate::discriminator::ACTIVATE_AUCTION;
use solana_system_interface::program::ID as system_program_id;

pub fn activate_auction(
    auction_state:       &Pubkey,
    competition:         &Pubkey,
    authority:           &Pubkey,
    buffer:              &Pubkey,
    delegation_record:   &Pubkey,
    delegation_metadata: &Pubkey,
    delegation_program:  &Pubkey,
    validator:           &Pubkey,
) -> Instruction {
    let program_id = Pubkey::from(*crate::ID.as_array());

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(*auction_state, false),
            AccountMeta::new_readonly(*competition, false),
            AccountMeta::new(*authority, true),
            AccountMeta::new(*buffer, false),
            AccountMeta::new(*delegation_record, false),
            AccountMeta::new(*delegation_metadata, false),
            AccountMeta::new_readonly(*delegation_program, false),
            AccountMeta::new_readonly(system_program_id, false),
            AccountMeta::new_readonly(*validator, false),
        ],
        data: ACTIVATE_AUCTION.to_vec(),
    }
}
