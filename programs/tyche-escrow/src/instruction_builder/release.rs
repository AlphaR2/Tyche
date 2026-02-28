use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use tyche_common::seeds::VAULT_SEED;
use tyche_core::instruction_builder::initialize_protocol_config::derive_protocol_config_pda;
use crate::discriminator::RELEASE;

/// Derives the `EscrowVault` PDA for a given competition and depositor.
pub fn derive_vault_pda(competition: &Pubkey, depositor: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[VAULT_SEED, competition.as_ref(), depositor.as_ref()],
        &Pubkey::from(*crate::ID.as_array()),
    )
}

/// Builds a `Release` instruction.
///
/// Crank-only. Distributes vault funds using `vault.amount` as the canonical
/// purchase price — no `winning_amount` argument is accepted. The processor
/// reads all required values from on-chain state directly, preventing a
/// malicious crank from under-paying the seller.
///
/// Lamport distribution computed by the processor:
/// - protocol fee (`vault.amount × fee_basis_points / 10_000`) → `treasury`
/// - net bid (`vault.amount` − fee) → `authority` (seller)
/// - rent reserve (`vault.lamports()` − `vault.amount`) → `depositor` (winner)
///
/// The caller must supply `authority` = `CompetitionState::authority` and
/// `treasury` = `ProtocolConfig::treasury` from on-chain state — the processor
/// verifies both addresses match. The `protocol_config` PDA is derived internally.
///
/// # Account order
///
/// | # | Account            | Writable | Signer |
/// |---|--------------------|----------|--------|
/// | 0 | vault              | yes      | no     |
/// | 1 | authority          | yes      | no     |
/// | 2 | depositor          | yes      | no     |
/// | 3 | crank              | no       | yes    |
/// | 4 | competition        | no       | no     |
/// | 5 | participant_record | no       | no     |
/// | 6 | protocol_config    | no       | no     |
/// | 7 | treasury           | yes      | no     |
pub fn build_release(
    depositor:          &Pubkey,
    authority:          &Pubkey,
    crank:              &Pubkey,
    competition:        &Pubkey,
    participant_record: &Pubkey,
    treasury:           &Pubkey,
) -> (Instruction, Pubkey) {
    let program_id           = Pubkey::from(*crate::ID.as_array());
    let (vault, _)           = derive_vault_pda(competition, depositor);
    let (protocol_config, _) = derive_protocol_config_pda();

    // Instruction data is discriminator only — Release has no args.
    let data = RELEASE.to_vec();

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(vault, false),
            AccountMeta::new(*authority, false),
            AccountMeta::new(*depositor, false),
            AccountMeta::new_readonly(*crank, true),
            AccountMeta::new_readonly(*competition, false),
            AccountMeta::new_readonly(*participant_record, false),
            AccountMeta::new_readonly(protocol_config, false),
            AccountMeta::new(*treasury, false),
        ],
        data,
    };

    (ix, vault)
}
