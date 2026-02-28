use pinocchio::{
    AccountView, ProgramResult,
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
};
use ephemeral_rollups_pinocchio::instruction::commit_and_undelegate_accounts;
use tyche_common::phase::Phase;
use crate::{
    discriminator::{COMPETITION_STATE, PROTOCOL_CONFIG},
    error::TycheCoreError,
    state::{
        competition::CompetitionState,
        protocol_config::ProtocolConfig,
    },
};

/// Validated account context for `CloseCompetition`.
pub struct CloseCompetitionAccounts<'a> {
    pub competition:     &'a AccountView,
    pub crank:           &'a AccountView,
    pub permission:      &'a AccountView,
    pub magic_context:   &'a AccountView,
    pub magic_program:   &'a AccountView,
    pub protocol_config: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for CloseCompetitionAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [competition, crank, permission, magic_context, magic_program, protocol_config, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !competition.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !crank.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self { competition, crank, permission, magic_context, magic_program, protocol_config })
    }
}

/// Instruction context for `CloseCompetition`.
pub struct CloseCompetitionInstruction<'a> {
    pub accounts: CloseCompetitionAccounts<'a>,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for CloseCompetitionInstruction<'a> {
    type Error = ProgramError;

    fn try_from((accounts, _data): (&'a [AccountView], &'a [u8])) -> Result<Self, Self::Error> {
        let accounts = CloseCompetitionAccounts::try_from(accounts)?;
        Ok(Self { accounts })
    }
}

impl<'a> CloseCompetitionInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;

        // Read crank_authority from ProtocolConfig before borrowing competition.
        let crank_authority = {
            let data   = accounts.protocol_config.try_borrow()?;
            let config = bytemuck::try_from_bytes::<ProtocolConfig>(&*data)
                .map_err(|_| ProgramError::InvalidAccountData)?;

            if config.discriminator != PROTOCOL_CONFIG {
                return Err(TycheCoreError::InvalidDiscriminator.into());
            }

            config.crank_authority
        };

        {
            let mut data = accounts.competition.try_borrow_mut()?;
            let state    = bytemuck::from_bytes_mut::<CompetitionState>(&mut *data);

            // 1: Verify discriminator.
            if state.discriminator != COMPETITION_STATE {
                return Err(TycheCoreError::InvalidDiscriminator.into());
            }

            // 2: Phase gate — only Active competitions can be closed.
            if state.phase != Phase::Active as u8 {
                return Err(TycheCoreError::InvalidPhase.into());
            }

            // 3: Crank authority check — read from ProtocolConfig.
            if *accounts.crank.address() != crank_authority {
                return Err(TycheCoreError::InvalidCrankAuthority.into());
            }

            // 4: Time gate — competition must have actually expired.
            let clock = Clock::get()?;
            if clock.unix_timestamp < state.end_time {
                return Err(TycheCoreError::AuctionNotExpired.into());
            }

            // 5: Transition phase to Settling.
            state.phase = Phase::Settling as u8;

        } // mutable borrow drops here

        commit_and_undelegate_accounts(
            accounts.crank,
            &[*accounts.competition, *accounts.permission],
            accounts.magic_context,
            accounts.magic_program,
        )?;

        Ok(())
    }
}
