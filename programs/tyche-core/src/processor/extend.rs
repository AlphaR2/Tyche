use pinocchio::{
    AccountView, ProgramResult,
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
};
use ephemeral_rollups_pinocchio::instruction::commit_accounts;
use tyche_common::phase::Phase;
use crate::{
    discriminator::{COMPETITION_STATE, PROTOCOL_CONFIG},
    error::TycheCoreError,
    state::{
        competition::CompetitionState,
        protocol_config::ProtocolConfig,
    },
};

/// Validated account context for `ExtendCompetition`.
pub struct ExtendCompetitionAccounts<'a> {
    pub competition:     &'a AccountView,
    pub crank:           &'a AccountView,
    pub magic_context:   &'a AccountView,
    pub magic_program:   &'a AccountView,
    pub protocol_config: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for ExtendCompetitionAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [competition, crank, magic_context, magic_program, protocol_config, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !competition.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !crank.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self { competition, crank, magic_context, magic_program, protocol_config })
    }
}

/// Instruction context for `ExtendCompetition`.
pub struct ExtendCompetitionInstruction<'a> {
    pub accounts: ExtendCompetitionAccounts<'a>,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for ExtendCompetitionInstruction<'a> {
    type Error = ProgramError;

    fn try_from(
        (accounts, _data): (&'a [AccountView], &'a [u8]),
    ) -> Result<Self, Self::Error> {
        let accounts = ExtendCompetitionAccounts::try_from(accounts)?;
        Ok(Self { accounts })
    }
}

impl<'a> ExtendCompetitionInstruction<'a> {
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

            // 2: Phase gate — only Active competitions can be extended.
            if state.phase != Phase::Active as u8 {
                return Err(TycheCoreError::InvalidPhase.into());
            }

            // 3: Crank authority check — read from ProtocolConfig.
            if *accounts.crank.address() != crank_authority {
                return Err(TycheCoreError::InvalidCrankAuthority.into());
            }

            // 4: Cap check.
            if state.soft_close_count >= state.max_soft_closes {
                return Err(TycheCoreError::SoftCloseCapReached.into());
            }

            // 5: Expiry check.
            let clock = Clock::get()?;
            if clock.unix_timestamp >= state.end_time {
                return Err(TycheCoreError::AuctionEnded.into());
            }

            // 6: Window check.
            let window_start = state.end_time
                .checked_sub(state.soft_close_window)
                .ok_or(TycheCoreError::ArithmeticOverflow)?;

            if clock.unix_timestamp < window_start {
                return Err(TycheCoreError::SoftCloseNotArmed.into());
            }

            // 7: Extend end_time.
            state.end_time = state.end_time
                .checked_add(state.soft_close_extension)
                .ok_or(TycheCoreError::ArithmeticOverflow)?;

            // 8: Increment soft-close counter.
            state.soft_close_count = state.soft_close_count
                .checked_add(1)
                .ok_or(TycheCoreError::ArithmeticOverflow)?;

        } // mutable borrow drops here

        commit_accounts(
            accounts.crank,
            &[*accounts.competition],
            accounts.magic_context,
            accounts.magic_program,
        )?;

        Ok(())
    }
}
