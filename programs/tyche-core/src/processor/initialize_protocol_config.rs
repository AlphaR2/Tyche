use pinocchio::{
    Address, AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
};
use pinocchio_system::instructions::CreateAccount;
use tyche_common::{
    constants::MAX_FEE_BASIS_POINTS,
    seeds::PROTOCOL_CONFIG_SEED,
};
use crate::{
    discriminator::PROTOCOL_CONFIG,
    error::TycheCoreError,
    instruction_args::initialize_protocol_config::InitializeProtocolConfigArgs,
    state::protocol_config::ProtocolConfig,
};

// ── Account context

/// Validated account context for `InitializeProtocolConfig`.
///
/// Called once at deployment. Creates the singleton `ProtocolConfig` PDA and
/// writes all governance parameters. The PDA is derived from
/// `[PROTOCOL_CONFIG_SEED]` — only one config account can exist per program.
pub struct InitializeProtocolConfigAccounts<'a> {
    pub protocol_config: &'a AccountView,
    pub authority:       &'a AccountView,
    pub payer:           &'a AccountView,
    pub system_program:  &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for InitializeProtocolConfigAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [protocol_config, authority, payer, system_program, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !protocol_config.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        if !authority.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if !payer.is_signer() || !payer.is_writable() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self { protocol_config, authority, payer, system_program })
    }
}

// ── Instruction context 

pub struct InitializeProtocolConfigInstruction<'a> {
    pub accounts: InitializeProtocolConfigAccounts<'a>,
    pub args:     &'a InitializeProtocolConfigArgs,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for InitializeProtocolConfigInstruction<'a> {
    type Error = ProgramError;

    fn try_from((accounts, data): (&'a [AccountView], &'a [u8])) -> Result<Self, Self::Error> {
        let accounts = InitializeProtocolConfigAccounts::try_from(accounts)?;
        let args     = InitializeProtocolConfigArgs::load(data)?;
        Ok(Self { accounts, args })
    }
}

// ── Handler 

impl<'a> InitializeProtocolConfigInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;
        let args     = self.args;

        // 1: Singleton guard — reject if already initialized.
        if !accounts.protocol_config.is_data_empty() {
            return Err(TycheCoreError::ConfigAlreadyInitialized.into());
        }

        // 2: Validate fee ceiling — enforced at the instruction level,
        // independent of any future config change. The 10% hard ceiling
        // cannot be bypassed even by the authority.
        if args.fee_basis_points > MAX_FEE_BASIS_POINTS {
            return Err(TycheCoreError::FeeTooHigh.into());
        }

        // 3: Derive and verify PDA.
        // Seeds: [PROTOCOL_CONFIG_SEED] — singleton, no variable components.
        let (expected_pda, bump) = Address::find_program_address(
            &[PROTOCOL_CONFIG_SEED],
            &crate::ID,
        );

        if expected_pda.ne(accounts.protocol_config.address()) {
            return Err(ProgramError::InvalidSeeds);
        }

        // 4: Allocate ProtocolConfig account via system program CPI.
        let space    = ProtocolConfig::LEN;
        let lamports = Rent::get()?.try_minimum_balance(space)?;

        CreateAccount {
            from:  accounts.payer,
            to:    accounts.protocol_config,
            space: space as u64,
            lamports,
            owner: &crate::ID,
        }
        .invoke_signed(&[Signer::from(&[
            Seed::from(PROTOCOL_CONFIG_SEED),
            Seed::from(&[bump]),
        ])])?;

        // 5: Initialize all fields.
        {
            let mut data = accounts.protocol_config.try_borrow_mut()?;
            let config   = bytemuck::from_bytes_mut::<ProtocolConfig>(&mut *data);

            config.discriminator       = PROTOCOL_CONFIG;
            config.authority           = args.authority;
            config.emergency_authority = args.emergency_authority;
            config.treasury            = args.treasury;
            config.crank_authority     = args.crank_authority;
            config.fee_basis_points    = args.fee_basis_points;
            config.max_soft_closes_cap = args.max_soft_closes_cap;
            config.min_reserve_price   = args.min_reserve_price;
            config.min_duration_secs   = args.min_duration_secs;
            config.bump                = bump;
        }

        Ok(())
    }
}
