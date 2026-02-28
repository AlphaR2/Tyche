use pinocchio::{
    Address, AccountView, ProgramResult,
    error::ProgramError,
};
use crate::{
    discriminator::{DEPOSIT, RELEASE, REFUND},
    processor::{
        deposit::DepositInstruction,
        release::ReleaseInstruction,
        refund::RefundInstruction,
    },
};

/// Program entrypoint for `tyche-escrow`.
///
/// # Instruction data layout
///
/// ```text
/// [0..8]  — 8-byte Anchor-compatible discriminator
///           sha256("global:<instruction_name>")[0..8]
/// [8..]   — instruction-specific args serialised with bytemuck
///           empty for no-arg instructions (Release, Refund)
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

    // Read the first 8 bytes as the discriminator.
    let disc: [u8; 8] = data[0..8]
        .try_into()
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    // Everything after the 8-byte header is the args payload.
    // For Deposit, DepositArgs::load checks exact length (8 bytes).
    // For Release and Refund (no args), this is &[] and TryFrom ignores it via `_data`.
    let args = &data[8..];

    match disc {
        d if d == DEPOSIT =>
            DepositInstruction::try_from((accounts, args))?.handler(),

        d if d == RELEASE =>
            ReleaseInstruction::try_from((accounts, args))?.handler(),

        d if d == REFUND =>
            RefundInstruction::try_from((accounts, args))?.handler(),

        _ => Err(ProgramError::InvalidInstructionData),
    }
}
