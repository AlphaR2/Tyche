use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use bytemuck::bytes_of;
use crate::{
    args::place_bid::PlaceBidArgs,
    discriminator::PLACE_BID,
};
use solana_system_interface::program::ID as system_program_id;

pub fn place_bid(
    auction_state:              &Pubkey,
    competition:                &Pubkey,
    bid_record:                 &Pubkey,
    vault:                      &Pubkey,
    bidder:                     &Pubkey,
    payer:                      &Pubkey,
    competition_participant_record: &Pubkey,
    args:                       PlaceBidArgs,
) -> Instruction {
    let program_id      = Pubkey::from(*crate::ID.as_array());
    let tyche_core_id   = Pubkey::from(*tyche_core::ID.as_array());

    let mut data = PLACE_BID.to_vec();
    data.extend_from_slice(bytes_of(&args));

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(*auction_state, false),
            AccountMeta::new(*competition, false),
            AccountMeta::new(*bid_record, false),
            AccountMeta::new_readonly(*vault, false),
            AccountMeta::new_readonly(*bidder, true),
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(tyche_core_id, false),
            AccountMeta::new(*competition_participant_record, false),
            AccountMeta::new_readonly(system_program_id, false),
        ],
        data,
    }
}
