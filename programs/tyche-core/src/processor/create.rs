use pinocchio::{
    Address, AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{clock::Clock, rent::Rent, Sysvar},
};
use pinocchio_system::instructions::CreateAccount;
use tyche_common::{
    asset_type::AssetType,
    phase::Phase,
    seeds::COMPETITION_SEED,
};
use num_enum::TryFromPrimitive;
use crate::{
    instruction_args::create_competition::CreateCompetitionArgs,
    discriminator::{COMPETITION_STATE, PROTOCOL_CONFIG},
    error::TycheCoreError,
    state::{
        competition::CompetitionState,
        protocol_config::ProtocolConfig,
    },
};

/// Validated account context for `CreateCompetition`.
    pub struct CreateCompetitionAccounts<'a> {
    pub competition:     &'a AccountView,
    pub authority:       &'a AccountView,
    pub payer:           &'a AccountView,
    pub system_program:  &'a AccountView,
    pub protocol_config: &'a AccountView,
    }

    impl<'a> TryFrom<&'a [AccountView]> for CreateCompetitionAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [competition, authority, payer, system_program, protocol_config, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !competition.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !competition.is_data_empty() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }
        if !authority.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self { competition, authority, payer, system_program, protocol_config })
    }
}

/// Instruction context for `CreateCompetition`.
pub struct CreateCompetitionInstruction<'a> {
    pub accounts: CreateCompetitionAccounts<'a>,
    pub args:     &'a CreateCompetitionArgs,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for CreateCompetitionInstruction<'a> {
    type Error = ProgramError;

    fn try_from((accounts, data): (&'a [AccountView], &'a [u8])) -> Result<Self, Self::Error> {
        let accounts = CreateCompetitionAccounts::try_from(accounts)?;
        let args     = CreateCompetitionArgs::load(data)?;
        Ok(Self { accounts, args })
    }
}

impl<'a> CreateCompetitionInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;
        let args     = self.args;

        // 1: Validate asset_type is a known variant.
        AssetType::try_from_primitive(args.asset_type)
            .map_err(|_| TycheCoreError::InvalidAccountData)?;

        // 2: Read ProtocolConfig — validate reserve_price, duration, soft-close cap.
        let (min_reserve_price, min_duration_secs, max_soft_closes_cap) = {
            let data   = accounts.protocol_config.try_borrow()?;
            let config = bytemuck::try_from_bytes::<ProtocolConfig>(&*data)
                .map_err(|_| ProgramError::InvalidAccountData)?;

            if config.discriminator != PROTOCOL_CONFIG {
                return Err(TycheCoreError::InvalidDiscriminator.into());
            }

            (config.min_reserve_price, config.min_duration_secs, config.max_soft_closes_cap)
        };

        // 3: Validate reserve_price meets the protocol minimum.
        if args.reserve_price < min_reserve_price {
            return Err(TycheCoreError::InvalidAccountData.into());
        }

        // 4: Validate duration meets the protocol minimum.
        if args.duration_secs < min_duration_secs {
            return Err(TycheCoreError::InvalidAccountData.into());
        }

        // 5: Validate soft-close params are non-negative.
        if args.soft_close_window < 0 || args.soft_close_extension < 0 {
            return Err(TycheCoreError::InvalidAccountData.into());
        }

        // 6: Validate max_soft_closes does not exceed the protocol cap.
        if args.max_soft_closes > max_soft_closes_cap {
            return Err(TycheCoreError::InvalidAccountData.into());
        }

        // 7: Validate start_time is not already in the past.
        let clock = Clock::get()?;
        if args.start_time < clock.unix_timestamp {
            return Err(TycheCoreError::AuctionNotStarted.into());
        }

        // 8: Derive competition PDA and verify the provided account matches.
        let authority_bytes = accounts.authority.address().as_array();
        let id_bytes        = args.id.as_array();

        let (expected_pda, bump) = Address::find_program_address(
            &[COMPETITION_SEED, authority_bytes, id_bytes],
            &crate::ID,
        );

        if expected_pda.ne(accounts.competition.address()) {
            return Err(ProgramError::InvalidSeeds);
        }

        // 9: Allocate CompetitionState account via system program CPI.
        let space    = CompetitionState::LEN;
        let lamports = Rent::get()?.try_minimum_balance(space)?;

        CreateAccount {
            from:  accounts.payer,
            to:    accounts.competition,
            space: space as u64,
            lamports,
            owner: &crate::ID,
        }
        .invoke_signed(&[Signer::from(&[
            Seed::from(COMPETITION_SEED),
            Seed::from(authority_bytes),
            Seed::from(id_bytes),
            Seed::from(&[bump]),
        ])])?;

        // 10: Initialize CompetitionState.
        {
            let mut data = accounts.competition.try_borrow_mut()?;
            let state    = bytemuck::from_bytes_mut::<CompetitionState>(&mut *data);

            state.discriminator        = COMPETITION_STATE;
            state.id                   = args.id;
            state.authority            = *accounts.authority.address();
            state.asset_type           = args.asset_type;
            state.phase                = Phase::Scheduled as u8;
            state.start_time           = args.start_time;
            state.end_time             = 0;
            state.duration_secs        = args.duration_secs;
            state.soft_close_window    = args.soft_close_window;
            state.soft_close_extension = args.soft_close_extension;
            state.soft_close_count     = 0;
            state.max_soft_closes      = args.max_soft_closes;
            state.reserve_price        = args.reserve_price;
            state.participant_count    = 0;
            state.bump                 = bump;
        }

        Ok(())
    }
}
