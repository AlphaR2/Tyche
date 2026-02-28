use pinocchio::{
    Address, AccountView, ProgramResult,
    error::ProgramError,
};
use crate::{
    discriminator::{
        CREATE_AUCTION, ACTIVATE_AUCTION, PLACE_BID,
        FINALIZE_AUCTION, CANCEL_AUCTION, CLOSE_BID_RECORD,
    },
    processor::{
        activate_auction::ActivateAuctionInstruction,
        cancel_auction::CancelAuctionInstruction,
        close_bid_record::CloseBidRecordInstruction,
        create_auction::CreateAuctionInstruction,
        finalize_auction::FinalizeAuctionInstruction,
        place_bid::PlaceBidInstruction,
        process_undelegation::ProcessUndelegationInstruction,
    },
};

/// MagicBlock external-undelegation discriminator.
///
/// MagicBlock calls the program's own entrypoint with this discriminator when
/// the AuctionState account is being undelegated back to mainnet, giving the
/// program a chance to merge the buffer data into the live account.
const PROCESS_UNDELEGATION: [u8; 8] =
    ephemeral_rollups_pinocchio::consts::EXTERNAL_UNDELEGATE_DISCRIMINATOR;

pub fn process_instruction(
    _program_id: &Address,
    accounts:    &[AccountView],
    data:        &[u8],
) -> ProgramResult {
    // Every instruction carries the 8-byte discriminator header.
    if data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let disc: [u8; 8] = data[0..8]
        .try_into()
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    // Everything after the 8-byte header is the args payload.
    let args = &data[8..];

    match disc {
        d if d == CREATE_AUCTION =>
            CreateAuctionInstruction::try_from((accounts, args))?.handler(),

        d if d == ACTIVATE_AUCTION =>
            ActivateAuctionInstruction::try_from((accounts, args))?.handler(),

        d if d == PLACE_BID =>
            PlaceBidInstruction::try_from((accounts, args))?.handler(),

        d if d == FINALIZE_AUCTION =>
            FinalizeAuctionInstruction::try_from((accounts, args))?.handler(),

        d if d == CANCEL_AUCTION =>
            CancelAuctionInstruction::try_from((accounts, args))?.handler(),

        d if d == CLOSE_BID_RECORD =>
            CloseBidRecordInstruction::try_from((accounts, args))?.handler(),

        d if d == PROCESS_UNDELEGATION =>
            ProcessUndelegationInstruction::try_from((accounts, args))?.handler(),

        _ => Err(ProgramError::InvalidInstructionData),
    }
}
