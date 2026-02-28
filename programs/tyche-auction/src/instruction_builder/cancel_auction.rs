use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use crate::discriminator::CANCEL_AUCTION;

pub fn cancel_auction(
    auction_state:  &Pubkey,
    competition:    &Pubkey,
    authority:      &Pubkey,
    rent_recipient: &Pubkey,
) -> Instruction {
    let program_id = Pubkey::from(*crate::ID.as_array());

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(*auction_state, false),
            AccountMeta::new_readonly(*competition, false),
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new(*rent_recipient, false),
        ],
        data: CANCEL_AUCTION.to_vec(),
    }
}
