use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use tyche_common::seeds::{PARTICIPANT_SEED, VAULT_SEED};
use crate::discriminator::REFUND;

/// Derives the `EscrowVault` PDA for a given competition and depositor.
pub fn derive_vault_pda(competition: &Pubkey, depositor: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[VAULT_SEED, competition.as_ref(), depositor.as_ref()],
        &Pubkey::from(*crate::ID.as_array()),
    )
}

/// Derives the `ParticipantRecord` PDA for a given competition and depositor.
///
/// Seeds: `[PARTICIPANT_SEED, competition, depositor]` — owned by `tyche-core`.
///
/// The caller should pass the derived PDA even for `Cancelled` competitions where
/// the record may not be initialized. The `Refund` handler skips the participant
/// check on the `Cancelled` path, so the account is read but its data is ignored.
pub fn derive_participant_record_pda(competition: &Pubkey, depositor: &Pubkey) -> (Pubkey, u8) {
    let tyche_core_id = Pubkey::from(*tyche_core::ID.as_array());
    Pubkey::find_program_address(
        &[PARTICIPANT_SEED, competition.as_ref(), depositor.as_ref()],
        &tyche_core_id,
    )
}

/// Builds a `Refund` instruction.
///
/// Returns the full vault balance (bid + rent) to the depositor.
///
/// Valid when the competition is `Cancelled`, OR when it is `Settled` and the
/// depositor is not the winner. For the `Cancelled` path, the `participant_record`
/// account may not be initialized — pass the derived PDA and the handler will
/// skip the winner check.
///
/// # Account order
///
/// Must match `RefundAccounts::try_from` destructure exactly.
///
/// | # | Account           | Writable | Signer |
/// |---|-------------------|----------|--------|
/// | 0 | vault             | yes      | no     |
/// | 1 | depositor         | yes      | yes    |
/// | 2 | competition       | no       | no     |
/// | 3 | participant_record| no       | no     |
pub fn build_refund(
    depositor:   &Pubkey,
    competition: &Pubkey,
) -> (Instruction, Pubkey) {
    let program_id              = Pubkey::from(*crate::ID.as_array());
    let (vault, _)              = derive_vault_pda(competition, depositor);
    let (participant_record, _) = derive_participant_record_pda(competition, depositor);

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(vault, false),
            AccountMeta::new(*depositor, true),
            AccountMeta::new_readonly(*competition, false),
            AccountMeta::new_readonly(participant_record, false),
        ],
        data: REFUND.to_vec(),
    };

    (ix, vault)
}
