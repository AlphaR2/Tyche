use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

use solana_system_interface::program::ID as system_program;

use bytemuck::bytes_of;
use pinocchio::Address;
use tyche_common::seeds::COMPETITION_SEED;
use crate::{
    discriminator::CREATE_COMPETITION,
    instruction_args::create_competition::CreateCompetitionArgs,
};

/// Derives the `CompetitionState` PDA for a given authority and competition id.
///
/// Seeds: `[COMPETITION_SEED, authority, id]`
///
/// Call this before `build_create_competition` to get the correct
/// `competition` pubkey. The derived address is what the processor
/// verifies against in step 6 of the handler.
///
/// # Example
/// ```rust
/// let (competition_pda, _bump) = derive_competition_pda(&authority, &id);
/// let (ix, _) = build_create_competition(&authority, &payer, id, ...);
/// ```
pub fn derive_competition_pda(authority: &Pubkey, id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[COMPETITION_SEED, authority.as_ref(), id.as_ref()],
        &Pubkey::from(*crate::ID.as_array()),
    )
}

/// Builds a `CreateCompetition` instruction.
///
/// Derives the `CompetitionState` PDA automatically from `authority` and `id`.
/// Returns the instruction and the derived competition pubkey so the caller
/// can use the PDA address without deriving it a second time.
///
/// # Account order
///
/// Must match `CreateCompetitionAccounts::try_from` destructure exactly.
///
/// | # | Account        | Writable | Signer |
/// |---|----------------|----------|--------|
/// | 0 | competition    | yes      | no     |
/// | 1 | authority      | no       | yes    |
/// | 2 | payer          | yes      | yes    |
/// | 3 | system_program | no       | no     |
pub fn build_create_competition(
    authority:            &Pubkey,
    payer:                &Pubkey,
    id:                   [u8; 32],
    asset_type:           u8,
    start_time:           i64,
    duration_secs:        i64,
    soft_close_window:    i64,
    soft_close_extension: i64,
    max_soft_closes:      u8,
    reserve_price:        u64,
) -> (Instruction, Pubkey) {
    let program_id       = Pubkey::from(*crate::ID.as_array());
    let (competition, _) = derive_competition_pda(authority, &id);

    let args = CreateCompetitionArgs {
        id:                   Address::new_from_array(id),
        asset_type,
        _pad:                 [0u8; 7],
        start_time,
        duration_secs,
        soft_close_window,
        soft_close_extension,
        max_soft_closes,
        _pad2:                [0u8; 7],
        reserve_price,
    };

    let mut data = CREATE_COMPETITION.to_vec();
    data.extend_from_slice(bytes_of(&args));

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(competition, false),
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(system_program, false),
        ],
        data,
    };

    (ix, competition)
}