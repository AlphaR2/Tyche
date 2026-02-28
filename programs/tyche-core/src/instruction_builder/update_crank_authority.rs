use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use bytemuck::bytes_of;
use pinocchio::Address;
use crate::{
    discriminator::UPDATE_CRANK_AUTHORITY,
    instruction_args::update_crank_authority::UpdateCrankAuthorityArgs,
};
use super::initialize_protocol_config::derive_protocol_config_pda;

/// Builds an `UpdateCrankAuthority` instruction.
///
/// Replaces `config.crank_authority` with a new keypair. Separated from
/// `UpdateProtocolConfig` so ops keypairs can rotate the crank without
/// touching fee or treasury parameters.
///
/// # Account order
///
/// | # | Account         | Writable | Signer |
/// |---|-----------------|----------|--------|
/// | 0 | protocol_config | yes      | no     |
/// | 1 | authority       | no       | yes    |
pub fn build_update_crank_authority(
    authority:           &Pubkey,
    new_crank_authority: &Pubkey,
) -> Instruction {
    let program_id           = Pubkey::from(*crate::ID.as_array());
    let (protocol_config, _) = derive_protocol_config_pda();

    let args = UpdateCrankAuthorityArgs {
        new_crank_authority: Address::new_from_array(new_crank_authority.to_bytes()),
    };

    let mut data = UPDATE_CRANK_AUTHORITY.to_vec();
    data.extend_from_slice(bytes_of(&args));

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(protocol_config, false),
            AccountMeta::new_readonly(*authority, true),
        ],
        data,
    }
}
