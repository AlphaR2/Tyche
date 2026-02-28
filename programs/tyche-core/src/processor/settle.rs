use pinocchio::{
    AccountView, ProgramResult,
    error::ProgramError,
};
use tyche_common::phase::Phase;
use crate::{
    instruction_args::settle::SettleCompetitionArgs,
    discriminator::{COMPETITION_STATE, PARTICIPANT_RECORD, PROTOCOL_CONFIG},
    error::TycheCoreError,
    state::{
        competition::CompetitionState,
        participant::ParticipantRecord,
        protocol_config::ProtocolConfig,
    },
};

/// Validated account context for `SettleCompetition`.
pub struct SettleCompetitionAccounts<'a> {
    pub competition:              &'a AccountView,
    pub crank:                    &'a AccountView,
    pub delegation_record:        &'a AccountView,
    pub protocol_config:          &'a AccountView,
    pub winner_participant_record: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for SettleCompetitionAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [competition, crank, delegation_record, protocol_config, winner_participant_record, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !competition.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !crank.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        // winner_participant_record writable check is deferred to the handler.
        // When args.winner is the zero address there is no winner and the account
        // is never written; a readonly dummy account is acceptable in that case.
        // try_borrow_mut() inside the handler enforces writability when it is needed.

        Ok(Self { competition, crank, delegation_record, protocol_config, winner_participant_record })
    }
}

/// Instruction context for `SettleCompetition`.
pub struct SettleCompetitionInstruction<'a> {
    pub accounts: SettleCompetitionAccounts<'a>,
    pub args:     &'a SettleCompetitionArgs,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for SettleCompetitionInstruction<'a> {
    type Error = ProgramError;

    fn try_from(
        (accounts, data): (&'a [AccountView], &'a [u8]),
    ) -> Result<Self, Self::Error> {
        let accounts = SettleCompetitionAccounts::try_from(accounts)?;
        let args     = SettleCompetitionArgs::load(data)?;
        Ok(Self { accounts, args })
    }
}

impl<'a> SettleCompetitionInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;
        let args     = self.args;

        // 1: Read crank_authority from ProtocolConfig.
        let crank_authority = {
            let data   = accounts.protocol_config.try_borrow()?;
            let config = bytemuck::try_from_bytes::<ProtocolConfig>(&*data)
                .map_err(|_| ProgramError::InvalidAccountData)?;

            if config.discriminator != PROTOCOL_CONFIG {
                return Err(TycheCoreError::InvalidDiscriminator.into());
            }

            config.crank_authority
        };

        // Scoped borrow — competition mutations + all checks in one block so
        // the borrow drops before we potentially write to winner_participant_record.
        {
            let mut data = accounts.competition.try_borrow_mut()?;
            let state    = bytemuck::from_bytes_mut::<CompetitionState>(&mut *data);

            // 2: Verify discriminator.
            if state.discriminator != COMPETITION_STATE {
                return Err(TycheCoreError::InvalidDiscriminator.into());
            }

            // 3: Phase gate — must be Settling.
            if state.phase != Phase::Settling as u8 {
                return Err(TycheCoreError::InvalidPhase.into());
            }

            // 4: Crank authority check — read from ProtocolConfig.
            if *accounts.crank.address() != crank_authority {
                return Err(TycheCoreError::InvalidCrankAuthority.into());
            }

            // 5: Undelegation proof — delegation_record must have zero lamports.
            if accounts.delegation_record.lamports() != 0 {
                return Err(TycheCoreError::NotUndelegated.into());
            }

            // 6: Write settlement_ref.
            state.settlement_ref = args.settlement_ref;

            // 7: Transition to Settled — terminal state.
            state.phase = Phase::Settled as u8;
        } // competition borrow drops here

        // 8: Mark winner if one exists.
        // args.winner is zero address when there is no winner (e.g. zero-bid auction).
        // tyche-core owns ParticipantRecord so only it can write IS_WINNER — the vertical
        // program passes the winner pubkey here rather than writing directly.
        let zero_address = pinocchio::Address::default();
        if args.winner != zero_address {
            let mut record_data = accounts.winner_participant_record.try_borrow_mut()?;
            let record = bytemuck::from_bytes_mut::<ParticipantRecord>(&mut *record_data);

            if record.discriminator != PARTICIPANT_RECORD {
                return Err(TycheCoreError::InvalidDiscriminator.into());
            }
            if record.participant != args.winner {
                return Err(TycheCoreError::InvalidAccountData.into());
            }

            record.is_winner = ParticipantRecord::IS_WINNER;
        }

        Ok(())
    }
}
