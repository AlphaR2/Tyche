use pinocchio::{
    AccountView, ProgramResult,
    error::ProgramError,
};
use ephemeral_rollups_pinocchio::{
    instruction::delegate_account,
    types::DelegateConfig,
};
use tyche_common::{phase::Phase, seeds::AUCTION_SEED};
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

/// Validated account context for `ActivateAuction`.
///
/// Delegates the `AuctionState` PDA to the ephemeral rollup so that
/// `PlaceBid` instructions can run inside the PER session.
/// `tyche-core`'s `ActivateCompetition` must have already delegated
/// the `CompetitionState` and set up the MagicBlock ACL permission.
pub struct ActivateAuctionAccounts<'a> {
    pub auction_state:       &'a AccountView,
    pub competition:         &'a AccountView,
    pub authority:           &'a AccountView,
    pub buffer:              &'a AccountView,
    pub delegation_record:   &'a AccountView,
    pub delegation_metadata: &'a AccountView,
    pub delegation_program:  &'a AccountView,
    pub system_program:      &'a AccountView,
    pub validator:           &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for ActivateAuctionAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [
            auction_state,
            competition,
            authority,
            buffer,
            delegation_record,
            delegation_metadata,
            delegation_program,
            system_program,
            validator,
            ..
        ] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !auction_state.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !authority.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            auction_state,
            competition,
            authority,
            buffer,
            delegation_record,
            delegation_metadata,
            delegation_program,
            system_program,
            validator,
        })
    }
}

// ── Instruction context 

pub struct ActivateAuctionInstruction<'a> {
    pub accounts: ActivateAuctionAccounts<'a>,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for ActivateAuctionInstruction<'a> {
    type Error = ProgramError;

    fn try_from(
        (accounts, _data): (&'a [AccountView], &'a [u8]),
    ) -> Result<Self, Self::Error> {
        let accounts = ActivateAuctionAccounts::try_from(accounts)?;
        Ok(Self { accounts })
    }
}

// ── Handler 

impl<'a> ActivateAuctionInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;

        // Cache competition pubkey before borrows.
        let competition_pubkey = *accounts.competition.address();

        // 1: Read and verify AuctionState — copy bump before borrow drops.
        let (bump, state_authority) = {
            let data  = accounts.auction_state.try_borrow()?;
            let state = bytemuck::try_from_bytes::<AuctionState>(&*data)
                .map_err(|_| TycheAuctionError::InvalidDiscriminator)?;

            if state.discriminator != AUCTION_STATE {
                return Err(TycheAuctionError::InvalidDiscriminator.into());
            }

            (state.bump, state.authority)
        };

        // 2: Authority check.
        if state_authority != *accounts.authority.address() {
            return Err(TycheAuctionError::NotAuthority.into());
        }

        // 3: Verify competition is Active — auction can only be delegated
        //    once the competition session has started.
        {
            let data  = accounts.competition.try_borrow()?;
            let state = bytemuck::try_from_bytes::<CompetitionState>(&*data)
                .map_err(|_| TycheAuctionError::InvalidCompetition)?;

            if state.discriminator != COMPETITION_STATE {
                return Err(TycheAuctionError::InvalidCompetition.into());
            }
            if state.phase != Phase::Active as u8 {
                return Err(TycheAuctionError::InvalidPhase.into());
            }
        }

        // 4: Delegate AuctionState to the ephemeral rollup.
        // Seeds (without bump) match the PDA derivation: [AUCTION_SEED, competition].
        let competition_bytes = competition_pubkey.as_array();
        let delegate_config   = DelegateConfig {
            validator: Some(*accounts.validator.address()),
            ..Default::default()
        };

        delegate_account(
            &[
                accounts.authority,        // [0] payer
                accounts.auction_state,    // [1] PDA being delegated
                accounts.delegation_program, // [2] owner_program (delegation program)
                accounts.buffer,           // [3] buffer PDA
                accounts.delegation_record,  // [4] delegation record
                accounts.delegation_metadata, // [5] delegation metadata
                accounts.system_program,   // [6] system program
            ],
            &[AUCTION_SEED, competition_bytes],
            bump,
            delegate_config,
        )?;

        Ok(())
    }
}
