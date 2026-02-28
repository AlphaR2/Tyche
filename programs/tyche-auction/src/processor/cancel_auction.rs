use pinocchio::{
    AccountView, ProgramResult,
    error::ProgramError,
};
use tyche_common::phase::Phase;
use tyche_core::{
    discriminator::COMPETITION_STATE,
    state::competition::CompetitionState,
};
use crate::{
    discriminator::AUCTION_STATE,
    error::TycheAuctionError,
    state::auction::AuctionState,
};

// ── Account context 

/// Validated account context for `CancelAuction`.
///
/// Closes the `AuctionState` account and returns its rent to `rent_recipient`
/// once the underlying competition has been cancelled.
pub struct CancelAuctionAccounts<'a> {
    pub auction_state:  &'a AccountView,
    pub competition:    &'a AccountView,
    pub authority:      &'a AccountView,
    pub rent_recipient: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for CancelAuctionAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [auction_state, competition, authority, rent_recipient, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !auction_state.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !authority.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !rent_recipient.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(Self { auction_state, competition, authority, rent_recipient })
    }
}

// ── Instruction context 

pub struct CancelAuctionInstruction<'a> {
    pub accounts: CancelAuctionAccounts<'a>,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for CancelAuctionInstruction<'a> {
    type Error = ProgramError;

    fn try_from(
        (accounts, _data): (&'a [AccountView], &'a [u8]),
    ) -> Result<Self, Self::Error> {
        let accounts = CancelAuctionAccounts::try_from(accounts)?;
        Ok(Self { accounts })
    }
}

// ── Handler 

impl<'a> CancelAuctionInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;

        // 1: Read and verify AuctionState — copy authority before borrow drops.
        let state_authority = {
            let data  = accounts.auction_state.try_borrow()?;
            let state = bytemuck::try_from_bytes::<AuctionState>(&*data)
                .map_err(|_| TycheAuctionError::InvalidDiscriminator)?;

            if state.discriminator != AUCTION_STATE {
                return Err(TycheAuctionError::InvalidDiscriminator.into());
            }

            state.authority
        };

        // 2: Authority check.
        if state_authority != *accounts.authority.address() {
            return Err(TycheAuctionError::NotAuthority.into());
        }

        // 3: Competition must be Cancelled — only then can the auction be closed.
        {
            let data  = accounts.competition.try_borrow()?;
            let state = bytemuck::try_from_bytes::<CompetitionState>(&*data)
                .map_err(|_| TycheAuctionError::InvalidCompetition)?;

            if state.discriminator != COMPETITION_STATE {
                return Err(TycheAuctionError::InvalidCompetition.into());
            }
            if state.phase != Phase::Cancelled as u8 {
                return Err(TycheAuctionError::InvalidPhase.into());
            }
        }

        // 4: Drain lamports from auction_state to rent_recipient.
        let rent = accounts.auction_state.lamports();
        accounts.rent_recipient.set_lamports(
            accounts.rent_recipient.lamports()
                .checked_add(rent)
                .ok_or(TycheAuctionError::ArithmeticOverflow)?,
        );
        accounts.auction_state.set_lamports(0);

        // 5: Zero the data so the account is formally closed.
        {
            let mut data = accounts.auction_state.try_borrow_mut()?;
            data.fill(0);
        }

        Ok(())
    }
}
