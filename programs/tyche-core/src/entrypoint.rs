use pinocchio::{
    Address, AccountView, ProgramResult,
    error::ProgramError,
};
use crate::{
    discriminator::{
        CREATE_COMPETITION, ACTIVATE_COMPETITION, EXTEND_COMPETITION,
        CLOSE_COMPETITION,  SETTLE_COMPETITION,  CANCEL_COMPETITION, REGISTER_BID,
        INITIALIZE_PROTOCOL_CONFIG, UPDATE_PROTOCOL_CONFIG, UPDATE_CRANK_AUTHORITY,
    },
    processor::{
        activate::ActivateCompetitionInstruction,
        cancel::CancelCompetitionInstruction,
        close::CloseCompetitionInstruction,
        create::CreateCompetitionInstruction,
        extend::ExtendCompetitionInstruction,
        initialize_protocol_config::InitializeProtocolConfigInstruction,
        register_bid::RegisterBidInstruction,
        settle::SettleCompetitionInstruction,
        update_crank_authority::UpdateCrankAuthorityInstruction,
        update_protocol_config::UpdateProtocolConfigInstruction,
    },
};

/// Program entrypoint for `tyche-core`.
///
/// # Instruction data layout
///
/// ```text
/// [0..8]  — 8-byte Anchor-compatible discriminator
///           sha256("global:<instruction_name>")[0..8]
/// [8..]   — instruction-specific args serialised with bytemuck
///           empty for no-arg instructions (Extend, Close, Cancel, RegisterBid)
/// ```
///
/// Discriminators match the Anchor convention so Anchor clients, Anchor CPI
/// callers, and Codama-generated TypeScript SDKs can derive the correct prefix
/// from the instruction name alone — no knowledge of internal numbering required.
pub fn process_instruction(
    _program_id: &Address,
    accounts:    &[AccountView],
    data:        &[u8],
) -> ProgramResult {

    // Every instruction carries the 8-byte discriminator header.
    if data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Read the first 8 bytes as the discriminator.
    let disc: [u8; 8] = data[0..8]
        .try_into()
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    // Everything after the 8-byte header is the args payload.
    // For no-arg instructions this is &[] — TryFrom impls ignore it via `_data`.
    // For arg instructions (Create, Activate, Settle), SomeArgs::load checks exact length.
    let args = &data[8..];

    match disc {
        d if d == CREATE_COMPETITION =>
            CreateCompetitionInstruction::try_from((accounts, args))?.handler(),

        d if d == ACTIVATE_COMPETITION =>
            ActivateCompetitionInstruction::try_from((accounts, args))?.handler(),

        d if d == EXTEND_COMPETITION =>
            ExtendCompetitionInstruction::try_from((accounts, args))?.handler(),

        d if d == CLOSE_COMPETITION =>
            CloseCompetitionInstruction::try_from((accounts, args))?.handler(),

        d if d == SETTLE_COMPETITION =>
            SettleCompetitionInstruction::try_from((accounts, args))?.handler(),

        d if d == CANCEL_COMPETITION =>
            CancelCompetitionInstruction::try_from((accounts, args))?.handler(),

        d if d == REGISTER_BID =>
            RegisterBidInstruction::try_from((accounts, args))?.handler(),

        d if d == INITIALIZE_PROTOCOL_CONFIG =>
            InitializeProtocolConfigInstruction::try_from((accounts, args))?.handler(),

        d if d == UPDATE_PROTOCOL_CONFIG =>
            UpdateProtocolConfigInstruction::try_from((accounts, args))?.handler(),

        d if d == UPDATE_CRANK_AUTHORITY =>
            UpdateCrankAuthorityInstruction::try_from((accounts, args))?.handler(),

        _ => Err(ProgramError::InvalidInstructionData),
    }
}
