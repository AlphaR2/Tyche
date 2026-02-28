use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use bytemuck::bytes_of;
use pinocchio::Address;
use crate::{
    discriminator::UPDATE_PROTOCOL_CONFIG,
    instruction_args::update_protocol_config::UpdateProtocolConfigArgs,
};
use super::initialize_protocol_config::derive_protocol_config_pda;

/// Builds an `UpdateProtocolConfig` instruction.
///
/// Updates treasury, fee, soft-close cap, reserve-price floor, and duration
/// floor in the singleton `ProtocolConfig`. Authority-gated.
///
/// # Account order
///
/// | # | Account         | Writable | Signer |
/// |---|-----------------|----------|--------|
/// | 0 | protocol_config | yes      | no     |
/// | 1 | authority       | no       | yes    |
pub fn build_update_protocol_config(
    authority:               &Pubkey,
    new_treasury:            &Pubkey,
    new_fee_basis_points:    u16,
    new_max_soft_closes_cap: u8,
    new_min_reserve_price:   u64,
    new_min_duration_secs:   i64,
) -> Instruction {
    let program_id           = Pubkey::from(*crate::ID.as_array());
    let (protocol_config, _) = derive_protocol_config_pda();

    let args = UpdateProtocolConfigArgs {
        new_treasury:            Address::new_from_array(new_treasury.to_bytes()),
        new_fee_basis_points,
        new_max_soft_closes_cap,
        _pad:                    [0u8; 5],
        new_min_reserve_price,
        new_min_duration_secs,
    };

    let mut data = UPDATE_PROTOCOL_CONFIG.to_vec();
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
