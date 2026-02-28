use pinocchio::{
    Address, AccountView, ProgramResult,
    error::ProgramError,
};
use crate::{
    discriminator::{
        CREATE_REGISTRAR, CREATE_VOTER_WEIGHT_RECORD, UPDATE_VOTER_WEIGHT_RECORD,
        UPDATE_MAX_VOTER_WEIGHT_RECORD,
    },
    processor::{
        create_registrar::CreateRegistrarInstruction,
        create_voter_weight_record::CreateVoterWeightRecordInstruction,
        update_voter_weight_record::UpdateVoterWeightRecordInstruction,
        update_max_voter_weight_record::UpdateMaxVoterWeightRecordInstruction,
    },
};

/// Program entrypoint for `tyche-voter-weight-plugin`.
///
/// # Instruction data layout
///
/// ```text
/// [0..8]  — 8-byte Anchor-compatible discriminator
///           sha256("global:<instruction_name>")[0..8]
/// [8..]   — instruction-specific args serialised with bytemuck
///           empty for no-arg instructions (CreateVoterWeightRecord, UpdateVoterWeightRecord)
/// ```
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

    let args = &data[8..];

    match disc {
        d if d == CREATE_REGISTRAR =>
            CreateRegistrarInstruction::try_from((accounts, args))?.handler(),

        d if d == CREATE_VOTER_WEIGHT_RECORD =>
            CreateVoterWeightRecordInstruction::try_from((accounts, args))?.handler(),

        d if d == UPDATE_VOTER_WEIGHT_RECORD =>
            UpdateVoterWeightRecordInstruction::try_from((accounts, args))?.handler(),

        d if d == UPDATE_MAX_VOTER_WEIGHT_RECORD =>
            UpdateMaxVoterWeightRecordInstruction::try_from((accounts, args))?.handler(),

        _ => Err(ProgramError::InvalidInstructionData),
    }
}
