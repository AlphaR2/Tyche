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
    constants::MIN_RESERVE_PRICE_LAMPORTS,
    seeds::COMPETITION_SEED,
};
use num_enum::TryFromPrimitive;
use crate::{
    instruction_args::create_competition::CreateCompetitionArgs,
    discriminator::COMPETITION_STATE,
    error::TycheCoreError,
    state::competition::CompetitionState,
};

/// Validated account context for `CreateCompetition`.
    pub struct CreateCompetitionAccounts<'a> {
    pub competition:    &'a AccountView,
    pub authority:      &'a AccountView,
    pub payer:          &'a AccountView,
    pub system_program: &'a AccountView,
    }

    impl<'a> TryFrom<&'a [AccountView]> for CreateCompetitionAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [competition, authority, payer, system_program, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // competition must be writable and not yet initialized
        if !competition.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !competition.is_data_empty() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        // authority must sign — they own this competition
        if !authority.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // payer must sign — they fund the rent
        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self { competition, authority, payer, system_program })
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
        // Rejects any u8 that does not map to a defined AssetType.
        AssetType::try_from_primitive(args.asset_type)
            .map_err(|_| TycheCoreError::InvalidAccountData)?;

        // 2: Validate reserve_price meets the protocol minimum.
        // Zero or dust reserves would accept any bid.
        if args.reserve_price < MIN_RESERVE_PRICE_LAMPORTS {
            return Err(TycheCoreError::InvalidAccountData.into());
        }

        // 3: Validate duration is positive.
        // Non-positive duration means end_time <= activation time — immediately closeable.
        if args.duration_secs <= 0 {
            return Err(TycheCoreError::InvalidAccountData.into());
        }

        // 4: Validate soft-close params are non-negative.
        // max_soft_closes == 0 is valid — disables soft-close for prediction markets / batches.
        if args.soft_close_window < 0 || args.soft_close_extension < 0 {
            return Err(TycheCoreError::InvalidAccountData.into());
        }

        // 5: Validate start_time is not already in the past.
        // Stale timestamps are a footgun — reject early.
        let clock = Clock::get()?;
        if args.start_time < clock.unix_timestamp {
            return Err(TycheCoreError::AuctionNotStarted.into());
        }

        // 6: Derive competition PDA and verify the provided account matches.
        // Canonical bump is stored on state so processors never call find again.
        let authority_bytes = accounts.authority.address().as_array();
        let id_bytes        = args.id.as_array();

        let (expected_pda, bump) = Address::find_program_address(
            &[COMPETITION_SEED, authority_bytes, id_bytes],
            &crate::ID,
        );

        if expected_pda.ne(accounts.competition.address()) {
            return Err(ProgramError::InvalidSeeds);
        }

        // 7: Allocate CompetitionState account via system program CPI.
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

        // 8: Initialize CompetitionState.
        // Borrow scoped so it drops before the function returns.
        // end_time written 0 — computed as clock + duration_secs by ActivateCompetition.
        // winner and final_amount left zero — written by SettleCompetition.
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
