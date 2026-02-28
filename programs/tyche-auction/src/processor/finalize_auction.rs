use pinocchio::{
    Address, AccountView, ProgramResult,
    cpi,
    error::ProgramError,
    instruction::{InstructionAccount, InstructionView},
};
use tyche_common::{phase::Phase, seeds::PARTICIPANT_SEED};
use tyche_core::{
    discriminator::{COMPETITION_STATE, PARTICIPANT_RECORD, SETTLE_COMPETITION},
    state::competition::CompetitionState,
};
use crate::{
    discriminator::AUCTION_STATE,
    error::TycheAuctionError,
    state::auction::AuctionState,
};

// ── Account context 

/// Validated account context for `FinalizeAuction`.
///
/// Called by the crank after the competition session ends and undelegation
/// completes. CPIs to tyche-core `SettleCompetition` which transitions the
/// competition to `Settled` and writes `IS_WINNER` on the winner's
/// `ParticipantRecord` (tyche-core–owned, so only it can write).
pub struct FinalizeAuctionAccounts<'a> {
    pub auction_state:     &'a AccountView,
    pub competition:       &'a AccountView,
    pub winner_participant: &'a AccountView,
    pub crank:             &'a AccountView,
    pub protocol_config:   &'a AccountView,
    pub tyche_core:        &'a AccountView,
    pub delegation_record: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for FinalizeAuctionAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [
            auction_state,
            competition,
            winner_participant,
            crank,
            protocol_config,
            tyche_core,
            delegation_record,
            ..
        ] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !auction_state.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        // winner_participant writable check is deferred to handler — the account
        // only needs to be writable when there IS a winner (args.winner != zero).
        // When there is no winner a readonly dummy account may be supplied.
        if !crank.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            auction_state,
            competition,
            winner_participant,
            crank,
            protocol_config,
            tyche_core,
            delegation_record,
        })
    }
}

// ── Instruction context 

pub struct FinalizeAuctionInstruction<'a> {
    pub accounts: FinalizeAuctionAccounts<'a>,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for FinalizeAuctionInstruction<'a> {
    type Error = ProgramError;

    fn try_from(
        (accounts, _data): (&'a [AccountView], &'a [u8]),
    ) -> Result<Self, Self::Error> {
        let accounts = FinalizeAuctionAccounts::try_from(accounts)?;
        Ok(Self { accounts })
    }
}

// ── Handler 

impl<'a> FinalizeAuctionInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;

        let competition_pubkey = *accounts.competition.address();

        // 1: Read AuctionState — verify discriminator and copy winner address.
        let (auction_state_pubkey, winner_address) = {
            let data  = accounts.auction_state.try_borrow()?;
            let state = bytemuck::try_from_bytes::<AuctionState>(&*data)
                .map_err(|_| TycheAuctionError::InvalidDiscriminator)?;

            if state.discriminator != AUCTION_STATE {
                return Err(TycheAuctionError::InvalidDiscriminator.into());
            }
            if state.competition != competition_pubkey {
                return Err(TycheAuctionError::InvalidCompetition.into());
            }

            (*accounts.auction_state.address(), state.current_winner)
        };

        // 2: Competition must be Settling (undelegated after CloseCompetition).
        {
            let data  = accounts.competition.try_borrow()?;
            let state = bytemuck::try_from_bytes::<CompetitionState>(&*data)
                .map_err(|_| TycheAuctionError::InvalidCompetition)?;

            if state.discriminator != COMPETITION_STATE {
                return Err(TycheAuctionError::InvalidCompetition.into());
            }
            if state.phase != Phase::Settling as u8 {
                return Err(TycheAuctionError::InvalidPhase.into());
            }
        }

        // 3: Verify winner_participant PDA when there is a winner.
        //    No winner → zero address → winner_participant account is ignored.
        let zero: Address = Address::default();
        if winner_address != zero {
            let winner_bytes      = winner_address.as_array();
            let competition_bytes = competition_pubkey.as_array();

            let (expected_participant, _) = Address::find_program_address(
                &[PARTICIPANT_SEED, competition_bytes, winner_bytes],
                accounts.tyche_core.address(),
            );

            if expected_participant.ne(accounts.winner_participant.address()) {
                return Err(TycheAuctionError::InvalidPda.into());
            }

            // Quick discriminator check — full validation done by tyche-core.
            let data = accounts.winner_participant.try_borrow()?;
            let record = bytemuck::try_from_bytes::<tyche_core::state::participant::ParticipantRecord>(&*data)
                .map_err(|_| TycheAuctionError::InvalidDiscriminator)?;

            if record.discriminator != PARTICIPANT_RECORD {
                return Err(TycheAuctionError::InvalidDiscriminator.into());
            }
        }

        // 4: CPI to tyche-core SettleCompetition.
        //    settlement_ref = auction_state pubkey
        //    winner         = current_winner (zero if no winner)
        //
        // SettleCompetitionArgs layout (64 bytes):

        //#[repr(C)]
        //#[derive(Clone, Copy, Debug, Pod, Zeroable, ShankType)]
        //pub struct SettleCompetitionArgs {
        //pub settlement_ref: Address,
        //pub winner:         Address,
        //}

        //   [0..32]  settlement_ref: Address
        //   [32..64] winner:         Address

        //run data to be slice to form the data for the CPI to tyche-core. We need to construct the data according to the expected layout of SettleCompetitionArgs. The first 8 bytes are for the discriminator of the SettleCompetition instruction, followed by the settlement_ref and winner addresses. We can use the constants defined in the tyche-core crate for the discriminator and then copy the bytes of the auction_state_pubkey and winner_address into the correct positions in the data array. I do not know if this is the best way to construct the data, but it should work so far we have compatible size 

        let mut cpi_data = [0u8; 8 + 64];
        cpi_data[0..8].copy_from_slice(&SETTLE_COMPETITION);  //discriminator
        cpi_data[8..40].copy_from_slice(auction_state_pubkey.as_array()); //settlement_ref as above
        cpi_data[40..72].copy_from_slice(winner_address.as_array()); //winner as above

        // SettleCompetition accounts:
        // [0] competition (w)
        // [1] crank (s)
        // [2] delegation_record
        // [3] protocol_config
        // [4] winner_participant_record (w when winner exists, r when no winner)
        //
        // winner_participant is writable only when there IS a winner.
        // When args.winner is zero, SettleCompetition skips the IS_WINNER write,
        // so a readonly dummy account (e.g. system program) is sufficient.
        let ix_accounts = [
            InstructionAccount::writable(accounts.competition.address()),
            InstructionAccount::readonly_signer(accounts.crank.address()),
            InstructionAccount::readonly(accounts.delegation_record.address()),
            InstructionAccount::readonly(accounts.protocol_config.address()),
            if winner_address != zero {
                InstructionAccount::writable(accounts.winner_participant.address())
            } else {
                InstructionAccount::readonly(accounts.winner_participant.address())
            },
        ];

        let ix = InstructionView {
            program_id: accounts.tyche_core.address(),
            accounts:   &ix_accounts,
            data:       &cpi_data,
        };

        cpi::invoke::<5>(&ix, &[
            accounts.competition,
            accounts.crank,
            accounts.delegation_record,
            accounts.protocol_config,
            accounts.winner_participant,
        ])?;

        Ok(())
    }
}
