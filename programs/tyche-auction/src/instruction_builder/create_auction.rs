use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use solana_system_interface::program::ID as system_program_id;
use bytemuck::bytes_of;
use crate::{
    args::create_auction::CreateAuctionArgs,
    discriminator::CREATE_AUCTION,
};

pub fn create_auction(
    auction_state: &Pubkey,
    competition:   &Pubkey,
    authority:     &Pubkey,
    payer:         &Pubkey,
    args:          CreateAuctionArgs,
) -> Instruction {
    let program_id = Pubkey::from(*crate::ID.as_array());

    let mut data = CREATE_AUCTION.to_vec();
    data.extend_from_slice(bytes_of(&args));

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(*auction_state, false),
            AccountMeta::new_readonly(*competition, false),
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(system_program_id, false),
        ],
        data,
    }
}
