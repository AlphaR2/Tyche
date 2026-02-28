use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use crate::discriminator::FINALIZE_AUCTION;

pub fn finalize_auction(
    auction_state:     &Pubkey,
    competition:       &Pubkey,
    winner_participant: &Pubkey,
    crank:             &Pubkey,
    protocol_config:   &Pubkey,
    delegation_record: &Pubkey,
) -> Instruction {
    let program_id    = Pubkey::from(*crate::ID.as_array());
    let tyche_core_id = Pubkey::from(*tyche_core::ID.as_array());

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(*auction_state, false),
            AccountMeta::new(*competition, false),
            AccountMeta::new(*winner_participant, false),
            AccountMeta::new_readonly(*crank, true),
            AccountMeta::new_readonly(*protocol_config, false),
            AccountMeta::new_readonly(tyche_core_id, false),
            AccountMeta::new_readonly(*delegation_record, false),
        ],
        data: FINALIZE_AUCTION.to_vec(),
    }
}
