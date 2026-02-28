use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use solana_system_interface::program::ID as system_program;
use bytemuck::bytes_of;
use tyche_common::seeds::VAULT_SEED;
use crate::{
    args::deposit::DepositArgs,
    discriminator::DEPOSIT,
};

/// Derives the `EscrowVault` PDA for a given competition and depositor.
///
/// Seeds: `[VAULT_SEED, competition, depositor]`
///
/// Call this before `build_deposit` to get the correct `vault` pubkey.
/// The derived address is what the processor verifies against in step 3
/// of the handler.
pub fn derive_vault_pda(competition: &Pubkey, depositor: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[VAULT_SEED, competition.as_ref(), depositor.as_ref()],
        &Pubkey::from(*crate::ID.as_array()),
    )
}

/// Builds a `Deposit` instruction.
///
/// Derives the `EscrowVault` PDA automatically from `competition` and `depositor`.
/// Returns the instruction and the derived vault pubkey so the caller can include
/// the PDA address without deriving it a second time.
///
/// On first call this creates the vault account and transfers `amount` lamports.
/// On subsequent calls it tops up the existing vault with `amount` more lamports.
///
/// # Account order
///
/// Must match `DepositAccounts::try_from` destructure exactly.
///
/// | # | Account        | Writable | Signer |
/// |---|----------------|----------|--------|
/// | 0 | vault          | yes      | no     |
/// | 1 | depositor      | yes      | yes    |
/// | 2 | payer          | yes      | yes    |
/// | 3 | competition    | no       | no     |
/// | 4 | system_program | no       | no     |
pub fn build_deposit(
    depositor:   &Pubkey,
    payer:       &Pubkey,
    competition: &Pubkey,
    amount:      u64,
) -> (Instruction, Pubkey) {
    let program_id    = Pubkey::from(*crate::ID.as_array());
    let (vault, _)    = derive_vault_pda(competition, depositor);

    let args = DepositArgs { amount };

    let mut data = DEPOSIT.to_vec();
    data.extend_from_slice(bytes_of(&args));

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(vault, false),
            AccountMeta::new(*depositor, true),
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(*competition, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data,
    };

    (ix, vault)
}
