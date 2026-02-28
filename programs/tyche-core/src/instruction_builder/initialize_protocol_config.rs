use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use solana_system_interface::program::ID as system_program;
use bytemuck::bytes_of;
use pinocchio::Address;
use tyche_common::seeds::PROTOCOL_CONFIG_SEED;
use crate::{
    discriminator::INITIALIZE_PROTOCOL_CONFIG,
    instruction_args::initialize_protocol_config::InitializeProtocolConfigArgs,
};

/// Derives the singleton `ProtocolConfig` PDA.
///
/// Seeds: `[PROTOCOL_CONFIG_SEED]` — no variable components; exactly one
/// config account can exist per program deployment.
pub fn derive_protocol_config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[PROTOCOL_CONFIG_SEED],
        &Pubkey::from(*crate::ID.as_array()),
    )
}

/// Builds an `InitializeProtocolConfig` instruction.
///
/// Creates the singleton `ProtocolConfig` PDA and writes all governance
/// parameters. Call once at deployment. The PDA is derived internally.
///
/// # Account order
///
/// | # | Account         | Writable | Signer |
/// |---|-----------------|----------|--------|
/// | 0 | protocol_config | yes      | no     |
/// | 1 | authority       | no       | yes    |
/// | 2 | payer           | yes      | yes    |
/// | 3 | system_program  | no       | no     |
pub fn build_initialize_protocol_config(
    authority:           &Pubkey,
    payer:               &Pubkey,
    emergency_authority: &Pubkey,
    treasury:            &Pubkey,
    crank_authority:     &Pubkey,
    fee_basis_points:    u16,
    max_soft_closes_cap: u8,
    min_reserve_price:   u64,
    min_duration_secs:   i64,
) -> (Instruction, Pubkey) {
    let program_id           = Pubkey::from(*crate::ID.as_array());
    let (protocol_config, _) = derive_protocol_config_pda();

    let args = InitializeProtocolConfigArgs {
        authority:           Address::new_from_array(authority.to_bytes()),
        emergency_authority: Address::new_from_array(emergency_authority.to_bytes()),
        treasury:            Address::new_from_array(treasury.to_bytes()),
        crank_authority:     Address::new_from_array(crank_authority.to_bytes()),
        fee_basis_points,
        _pad:                [0u8; 2],
        max_soft_closes_cap,
        _pad2:               [0u8; 3],
        min_reserve_price,
        min_duration_secs,
    };

    let mut data = INITIALIZE_PROTOCOL_CONFIG.to_vec();
    data.extend_from_slice(bytes_of(&args));

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(protocol_config, false),
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(system_program, false),
        ],
        data,
    };

    (ix, protocol_config)
}
