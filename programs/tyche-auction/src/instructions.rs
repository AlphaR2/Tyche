use shank::ShankInstruction;

use crate::args::{
    create_auction::CreateAuctionArgs,
    activate_auction::ActivateAuctionArgs,
    place_bid::PlaceBidArgs,
    finalize_auction::FinalizeAuctionArgs,
    cancel_auction::CancelAuctionArgs,
    close_bid_record::CloseBidRecordArgs,
};

#[derive(ShankInstruction)]
pub enum TycheAuctionInstruction {
    #[account(
        0, writable, name="auction_state", desc="AuctionState PDA being created"
    )]
    #[account(1, name="competition", desc="CompetitionState PDA")]
    #[account(2, signer, name="authority", desc="Competition authority")]
    #[account(3, signer, writable, name="payer", desc="Payer for rent")]
    #[account(4, name="system_program", desc="System program")]
    CreateAuction(CreateAuctionArgs),

    #[account(0, writable, name="auction_state", desc="AuctionState PDA")]
    #[account(1, name="competition", desc="CompetitionState PDA")]
    #[account(2, signer, name="authority", desc="Competition authority")]
    #[account(3, writable, name="buffer", desc="Delegation buffer")]
    #[account(4, writable, name="delegation_record", desc="Delegation record")]
    #[account(5, writable, name="delegation_metadata", desc="Delegation metadata")]
    #[account(6, name="delegation_program", desc="MagicBlock delegation program")]
    #[account(7, name="system_program", desc="System program")]
    #[account(8, name="validator", desc="TEE Validator Node")]
    ActivateAuction(ActivateAuctionArgs),

    #[account(0, writable, name="auction_state", desc="AuctionState PDA")]
    #[account(1, writable, name="competition", desc="CompetitionState PDA")]
    #[account(2, writable, name="bid_record", desc="BidRecord PDA")]
    #[account(3, name="vault", desc="EscrowVault PDA")]
    #[account(4, signer, name="bidder", desc="Bidder")]
    #[account(5, signer, writable, name="payer", desc="Payer")]
    #[account(6, name="tyche_core_program", desc="tyche-core program")]
    #[account(7, writable, name="competition_participant_record", desc="ParticipantRecord PDA")]
    #[account(8, name="system_program", desc="System program")]
    PlaceBid(PlaceBidArgs),

    #[account(0, writable, name="auction_state", desc="AuctionState PDA")]
    #[account(1, writable, name="competition", desc="CompetitionState PDA")]
    #[account(2, writable, name="winner_participant", desc="ParticipantRecord PDA for winner")]
    #[account(3, signer, name="crank", desc="Crank authority")]
    #[account(4, name="protocol_config", desc="ProtocolConfig PDA")]
    #[account(5, name="tyche_core_program", desc="tyche-core program")]
    #[account(6, name="delegation_record", desc="MagicBlock delegation record PDA — forwarded to SettleCompetition")]
    FinalizeAuction(FinalizeAuctionArgs),

    #[account(0, writable, name="auction_state", desc="AuctionState PDA")]
    #[account(1, name="competition", desc="CompetitionState PDA")]
    #[account(2, signer, name="authority", desc="Competition authority")]
    #[account(3, writable, name="rent_recipient", desc="Rent recipient")]
    CancelAuction(CancelAuctionArgs),

    #[account(0, writable, name="bid_record", desc="BidRecord PDA")]
    #[account(1, name="competition", desc="CompetitionState PDA")]
    #[account(2, signer, writable, name="bidder", desc="Bidder")]
    #[account(3, signer, name="caller_program", desc="escrow program CPI caller")]
    CloseBidRecord(CloseBidRecordArgs),
}
