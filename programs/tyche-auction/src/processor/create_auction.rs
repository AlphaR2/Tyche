use pinocchio::{
    Address, AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
};
use pinocchio_system::instructions::CreateAccount;
use tyche_common::{phase::Phase, seeds::AUCTION_SEED};
use tyche_core::{
    discriminator::COMPETITION_STATE,
    state::competition::CompetitionState,
};
use crate::{
    args::create_auction::CreateAuctionArgs,
    discriminator::AUCTION_STATE,
    error::TycheAuctionError,
    state::auction::AuctionState,
};

// ── Account context 

pub struct CreateAuctionAccounts<'a> {
    pub auction_state:  &'a AccountView,
    pub competition:    &'a AccountView,
    pub authority:      &'a AccountView,
    pub payer:          &'a AccountView,
    pub system_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for CreateAuctionAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [auction_state, competition, authority, payer, system_program, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !auction_state.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !authority.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !payer.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(Self { auction_state, competition, authority, payer, system_program })
    }
}

// ── Instruction context 

pub struct CreateAuctionInstruction<'a> {
    pub accounts: CreateAuctionAccounts<'a>,
    pub args:     &'a CreateAuctionArgs,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for CreateAuctionInstruction<'a> {
    type Error = ProgramError;

    fn try_from(
        (accounts, data): (&'a [AccountView], &'a [u8]),
    ) -> Result<Self, Self::Error> {
        let accounts = CreateAuctionAccounts::try_from(accounts)?;
        let args     = CreateAuctionArgs::load(data)?;
        Ok(Self { accounts, args })
    }
}

// ── Handler 

impl<'a> CreateAuctionInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;
        let args     = self.args;

        // 1: Validate competition state — must be Scheduled and authority must match.
        let competition_pubkey = *accounts.competition.address();
        let comp_authority = {
            let data  = accounts.competition.try_borrow()?;
            let state = bytemuck::try_from_bytes::<CompetitionState>(&*data)
                .map_err(|_| TycheAuctionError::InvalidCompetition)?;

            if state.discriminator != COMPETITION_STATE {
                return Err(TycheAuctionError::InvalidCompetition.into());
            }
            if state.phase != Phase::Scheduled as u8 {
                return Err(TycheAuctionError::InvalidPhase.into());
            }

            state.authority
        };

        // 2: Authority must match competition.authority.
        if comp_authority != *accounts.authority.address() {
            return Err(TycheAuctionError::NotAuthority.into());
        }

        // 3: Validate args.
        if args.min_bid_increment == 0 {
            return Err(TycheAuctionError::BidTooLow.into());
        }

        // 4: Derive and verify AuctionState PDA.
        let competition_bytes = competition_pubkey.as_array();
        let (expected_pda, bump) = Address::find_program_address(
            &[AUCTION_SEED, competition_bytes],
            &crate::ID,
        );

        if expected_pda.ne(accounts.auction_state.address()) {
            return Err(TycheAuctionError::InvalidPda.into());
        }

        // 5: Auction must not already exist.
        if !accounts.auction_state.is_data_empty() {
            return Err(TycheAuctionError::AuctionAlreadyExists.into());
        }

        // 6: Allocate AuctionState via system program CPI.
        let space    = AuctionState::LEN;
        let lamports = Rent::get()?.try_minimum_balance(space)?;

        CreateAccount {
            from:  accounts.payer,
            to:    accounts.auction_state,
            space: space as u64,
            lamports,
            owner: &crate::ID,
        }
        .invoke_signed(&[Signer::from(&[
            Seed::from(AUCTION_SEED),
            Seed::from(competition_bytes),
            Seed::from(&[bump]),
        ])])?;

        // 7: Initialize AuctionState fields.
        {
            let mut data = accounts.auction_state.try_borrow_mut()?;
            let state    = bytemuck::from_bytes_mut::<AuctionState>(&mut *data);

            state.discriminator     = AUCTION_STATE;
            state.competition       = competition_pubkey;
            state.authority         = *accounts.authority.address();
            state.asset_mint        = args.asset_mint;
            state.min_bid_increment = args.min_bid_increment;
            state.current_high_bid  = 0;
            state.current_winner    = Address::default();
            state.bid_count         = 0;
            state.bump              = bump;
        }

        Ok(())
    }
}
