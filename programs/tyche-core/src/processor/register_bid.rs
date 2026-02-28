use pinocchio::{
    Address, AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{clock::Clock, rent::Rent, Sysvar},
};
use pinocchio_system::instructions::CreateAccount;
use tyche_common::{
    constants::MAX_PARTICIPANTS,
    phase::Phase,
    seeds::PARTICIPANT_SEED,
};
use crate::{
    discriminator::{COMPETITION_STATE, PARTICIPANT_RECORD},
    error::TycheCoreError,
    state::{
        competition::CompetitionState,
        participant::ParticipantRecord,
    },
};

// ── Account context 

/// Validated account context for `RegisterBid`.
///
/// Called by `tyche-auction` and other verticals via CPI from `PlaceBid`.
/// On the bidder's first bid it creates a `ParticipantRecord` PDA and increments
/// `CompetitionState::participant_count`. On repeat bids from the same address
/// it updates `last_action` only — `participant_count` is not incremented again.
pub struct RegisterBidAccounts<'a> {
    pub competition:        &'a AccountView,
    pub participant_record: &'a AccountView,
    pub bidder:             &'a AccountView,
    pub payer:              &'a AccountView,
    pub system_program:     &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for RegisterBidAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [competition, participant_record, bidder, payer, system_program, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // competition must be writable — participant_count incremented on first bid
        if !competition.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        // participant_record must be writable — created or updated every call
        if !participant_record.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        // bidder must sign — prevents registering a bid for an arbitrary address
        if !bidder.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // payer must sign and be writable — funds rent on first bid
        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !payer.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(Self {
            competition,
            participant_record,
            bidder,
            payer,
            system_program,
        })
    }
}

// ── Instruction context 

/// Instruction context for `RegisterBid`.
///
/// No args — bidder address and competition pubkey derived from accounts.
pub struct RegisterBidInstruction<'a> {
    pub accounts: RegisterBidAccounts<'a>,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for RegisterBidInstruction<'a> {
    type Error = ProgramError;

    fn try_from(
        (accounts, _data): (&'a [AccountView], &'a [u8]),
    ) -> Result<Self, Self::Error> {
        let accounts = RegisterBidAccounts::try_from(accounts)?;
        Ok(Self { accounts })
    }
}

// ── Handler 

impl<'a> RegisterBidInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;

        // Cache competition address before any borrow — needed for PDA derivation
        // and record cross-checks after the guard block.
        let competition_pubkey = *accounts.competition.address();

        // 1: Guard competition state.
        // Scoped mutable borrow — reads discriminator, phase, and current participant_count,
        // then drops. participant_count is returned as a local so the cap check below can
        // reject the instruction before the CreateAccount CPI fires, avoiding wasted rent.
        let current_participant_count = {
            let mut data = accounts.competition.try_borrow_mut()?;
            let state    = bytemuck::from_bytes_mut::<CompetitionState>(&mut *data);

            // Reject accounts not initialized by this program.
            if state.discriminator != COMPETITION_STATE {
                return Err(TycheCoreError::InvalidDiscriminator.into());
            }

            // RegisterBid is only valid while the competition is Active.
            // Scheduled: account not yet delegated — no bids accepted.
            // Settling / Settled / Cancelled: session over or terminal — no bids accepted.
            if state.phase != Phase::Active as u8 {
                return Err(TycheCoreError::InvalidPhase.into());
            }

            state.participant_count
        }; // competition borrow drops here

        // 2: Derive and verify ParticipantRecord PDA.
        let bidder_bytes      = accounts.bidder.address().as_array();
        let competition_bytes = competition_pubkey.as_array();

        let (expected_pda, bump) = Address::find_program_address(
            &[PARTICIPANT_SEED, competition_bytes, bidder_bytes],
            &crate::ID,
        );

        if expected_pda.ne(accounts.participant_record.address()) {
            return Err(ProgramError::InvalidSeeds);
        }

        // 3: First bid or repeat bid?
        let is_first_bid = accounts.participant_record.is_data_empty();

        if is_first_bid {
            // 3a: Participant cap — reject before any CPI fires.
            // Checked here, before CreateAccount, so no rent is transferred and no
            // account is created for a competition that is already full.
            // MAX_PARTICIPANTS is the protocol-wide ceiling defined in tyche-common.
            if current_participant_count >= MAX_PARTICIPANTS {
                return Err(TycheCoreError::ParticipantCapReached.into());
            }

            // 3b: Allocate ParticipantRecord via system program CPI.
            // invoke_signed signs with PDA seeds so the system program
            // accepts this program as the rightful creator of the PDA address.
            let space    = ParticipantRecord::LEN;
            let lamports = Rent::get()?.try_minimum_balance(space)?;

            CreateAccount {
                from:  accounts.payer,
                to:    accounts.participant_record,
                space: space as u64,
                lamports,
                owner: &crate::ID,
            }
            .invoke_signed(&[Signer::from(&[
                Seed::from(PARTICIPANT_SEED),
                Seed::from(competition_bytes),
                Seed::from(bidder_bytes),
                Seed::from(&[bump]),
            ])])?;

            // 3c: Initialize ParticipantRecord fields.
            // is_winner is NOT_WINNER at creation — FinalizeAuction in tyche-auction
            // marks the winner post-settlement after undelegation completes.
            let clock = Clock::get()?;
            {
                let mut data = accounts.participant_record.try_borrow_mut()?;
                let record   = bytemuck::from_bytes_mut::<ParticipantRecord>(&mut *data);

                record.discriminator = PARTICIPANT_RECORD;
                record.competition   = competition_pubkey;
                record.participant   = *accounts.bidder.address();
                record.is_winner     = ParticipantRecord::NOT_WINNER;
                record.bump          = bump;
                record.last_action   = clock.unix_timestamp;
            }

            // 3d: Increment participant_count on CompetitionState.
            // Fresh borrow — guard borrow from step 1 has already dropped.
            // Checked add: participant_count is u32, and we already know it is
            // below MAX_PARTICIPANTS from the cap check above.
            {
                let mut data = accounts.competition.try_borrow_mut()?;
                let state    = bytemuck::from_bytes_mut::<CompetitionState>(&mut *data);

                state.participant_count = state.participant_count
                    .checked_add(1)
                    .ok_or(TycheCoreError::ArithmeticOverflow)?;
            }

        } else {
            // 3d: Repeat bid — update last_action only.
            // participant_count not incremented — bidder already counted on first bid.
            // Discriminator and participant checks guard against a mismatched account
            // from a different competition being passed in.
            let clock = Clock::get()?;
            {
                let mut data = accounts.participant_record.try_borrow_mut()?;
                let record   = bytemuck::from_bytes_mut::<ParticipantRecord>(&mut *data);

                if record.discriminator != PARTICIPANT_RECORD {
                    return Err(TycheCoreError::InvalidDiscriminator.into());
                }
                if record.competition != competition_pubkey {
                    return Err(TycheCoreError::InvalidAccountData.into());
                }
                if record.participant != *accounts.bidder.address() {
                    return Err(TycheCoreError::InvalidAccountData.into());
                }

                record.last_action = clock.unix_timestamp;
            }
        }

        Ok(())
    }
}