use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use crate::discriminator::CLOSE_BID_RECORD;

pub fn close_bid_record(
    bid_record:     &Pubkey,
    competition:    &Pubkey,
    bidder:         &Pubkey,
    caller_program: &Pubkey,
) -> Instruction {
    let program_id      = Pubkey::from(*crate::ID.as_array());
    let tyche_escrow_id = Pubkey::from(*tyche_escrow::ID.as_array());

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(*bid_record, false),
            AccountMeta::new_readonly(*competition, false),
            AccountMeta::new(*bidder, true),
            AccountMeta::new_readonly(tyche_escrow_id, true), // caller_program — must sign via CPI
        ],
        data: CLOSE_BID_RECORD.to_vec(),
    }
}
