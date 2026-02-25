use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use bytemuck::bytes_of;
use pinocchio::Address;
use crate::{
    discriminator::SETTLE_COMPETITION,
    instruction_args::settle::SettleCompetitionArgs,
};

/// Builds a `SettleCompetition` instruction.
///
/// Called by the protocol crank after undelegation completes (i.e.
/// `delegation_record` lamports == 0). Writes `settlement_ref` — the pubkey of the
/// vertical's result account — and transitions `CompetitionState` to `Settled`.
///
/// Only the protocol crank (`TYCHE_CRANK_PUBKEY`) may sign this instruction.
///
/// # Accounts
/// 0. `competition`      — writable (back on mainnet after undelegation)
/// 1. `crank`            — readonly signer (protocol crank only)
/// 2. `delegation_record`— readonly (must have zero lamports — proves undelegation complete)
pub fn build_settle_competition(
    competition:      &Pubkey,
    crank:            &Pubkey,
    delegation_record: &Pubkey,
    settlement_ref:   [u8; 32],
) -> Instruction {
    let program_id = Pubkey::from(*crate::ID.as_array());

    let args = SettleCompetitionArgs {
        settlement_ref: Address::new_from_array(settlement_ref),
    };

    let mut data = SETTLE_COMPETITION.to_vec();
    data.extend_from_slice(bytes_of(&args));

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(*competition, false),
            AccountMeta::new_readonly(*crank, true),
            AccountMeta::new_readonly(*delegation_record, false),
        ],
        data,
    }
}
