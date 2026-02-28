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
    discriminator::BID_RECORD,
    error::TycheAuctionError,
    state::bid_record::BidRecord,
};

// ── Account context 

/// Validated account context for `CloseBidRecord`.
///
/// Closes the `BidRecord` PDA once the competition is Settled and returns rent
/// to the bidder. Called via CPI from `tyche-escrow` during withdrawal.
pub struct CloseBidRecordAccounts<'a> {
    pub bid_record:     &'a AccountView,
    pub competition:    &'a AccountView,
    pub bidder:         &'a AccountView,
    pub caller_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for CloseBidRecordAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [bid_record, competition, bidder, caller_program, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !bid_record.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !bidder.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !bidder.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        // caller_program must sign — CPI signer propagation proves tyche-escrow authorized this.
        if !caller_program.is_signer() {
            return Err(TycheAuctionError::NotEscrowProgram.into());
        }

        Ok(Self { bid_record, competition, bidder, caller_program })
    }
}

// ── Instruction context 

pub struct CloseBidRecordInstruction<'a> {
    pub accounts: CloseBidRecordAccounts<'a>,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for CloseBidRecordInstruction<'a> {
    type Error = ProgramError;

    fn try_from(
        (accounts, _data): (&'a [AccountView], &'a [u8]),
    ) -> Result<Self, Self::Error> {
        let accounts = CloseBidRecordAccounts::try_from(accounts)?;
        Ok(Self { accounts })
    }
}

// ── Handler 

impl<'a> CloseBidRecordInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;

        let bidder_pubkey = *accounts.bidder.address();

        // 1: Verify caller is tyche-escrow.
        if *accounts.caller_program.address() != tyche_escrow::ID {
            return Err(TycheAuctionError::NotEscrowProgram.into());
        }

        // 2: Read and verify BidRecord.
        {
            let data   = accounts.bid_record.try_borrow()?;
            let record = bytemuck::try_from_bytes::<BidRecord>(&*data)
                .map_err(|_| TycheAuctionError::InvalidDiscriminator)?;

            if record.discriminator != BID_RECORD {
                return Err(TycheAuctionError::InvalidDiscriminator.into());
            }
            if record.bidder != bidder_pubkey {
                return Err(TycheAuctionError::NotAuthority.into());
            }
        }

        // 3: Competition must be Settled.
        {
            let data  = accounts.competition.try_borrow()?;
            let state = bytemuck::try_from_bytes::<CompetitionState>(&*data)
                .map_err(|_| TycheAuctionError::InvalidCompetition)?;

            if state.discriminator != COMPETITION_STATE {
                return Err(TycheAuctionError::InvalidCompetition.into());
            }
            if state.phase != Phase::Settled as u8 {
                return Err(TycheAuctionError::InvalidPhase.into());
            }
        }

        // 4: Drain lamports to bidder.
        let rent = accounts.bid_record.lamports();
        accounts.bidder.set_lamports(
            accounts.bidder.lamports()
                .checked_add(rent)
                .ok_or(TycheAuctionError::ArithmeticOverflow)?,
        );
        accounts.bid_record.set_lamports(0);

        // 5: Zero the data to formally close the account.
        {
            let mut data = accounts.bid_record.try_borrow_mut()?;
            data.fill(0);
        }

        Ok(())
    }
}
