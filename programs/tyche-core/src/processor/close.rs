use pinocchio::{
    AccountView, ProgramResult,
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
};
use ephemeral_rollups_pinocchio::instruction::commit_and_undelegate_accounts;
use tyche_common::{
    phase::Phase,
    constants::TYCHE_CRANK_PUBKEY,
};
use crate::{
    discriminator::COMPETITION_STATE,
    error::TycheCoreError,
    state::competition::CompetitionState,
};


/// Validated account context for `CloseCompetition`.
///
/// Runs inside the MagicBlock PER. Called by the protocol crank when
/// `clock.unix_timestamp >= end_time`. Transitions phase to Settling and
/// triggers undelegation — returning `CompetitionState` to mainnet for settlement.
///
/// Both `competition` and `permission` are passed to `commit_and_undelegate_accounts`.
/// `permission` was delegated by `ActivateCompetition` via `DelegatePermissionCpiBuilder`
/// and has its own `delegation_record` inside the PER — it must be explicitly
/// undelegated here or it is orphaned inside the TEE.
pub struct CloseCompetitionAccounts<'a> {
    pub competition:   &'a AccountView,
    pub crank:         &'a AccountView,
    /// ACL permission PDA delegated during activation — must be undelegated alongside competition.
    pub permission:    &'a AccountView,
    pub magic_context: &'a AccountView,
    pub magic_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for CloseCompetitionAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [competition, crank, permission, magic_context, magic_program, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // competition must be writable — phase is mutated to Settling
        if !competition.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        // crank must sign — prevents unauthorized closure
        if !crank.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            competition,
            crank,
            permission,
            magic_context,
            magic_program,
        })
    }
}

/// Instruction context for `CloseCompetition`.
///
/// No args — all inputs read directly from `CompetitionState`.
/// The crank drives this instruction entirely from on-chain state.
pub struct CloseCompetitionInstruction<'a> {
    pub accounts: CloseCompetitionAccounts<'a>,
    // no args — end_time read directly from CompetitionState
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

        // Scope mutable borrow — drop before commit_and_undelegate CPI.
        {
            let mut data = accounts.competition.try_borrow_mut()?;
            let state    = bytemuck::from_bytes_mut::<CompetitionState>(&mut *data);

            // 1: Verify discriminator — reject accounts not initialized by this program.
            if state.discriminator != COMPETITION_STATE {
                return Err(TycheCoreError::InvalidDiscriminator.into());
            }

            // 2: Phase gate — only Active competitions can be closed.
            // Settling means undelegation already triggered — do not call twice.
            // Scheduled competitions were never delegated — nothing to undelegate.
            if state.phase != Phase::Active as u8 {
                return Err(TycheCoreError::InvalidPhase.into());
            }

            // 3: Crank authority check — only the protocol crank may close.
            // Prevents the authority from force-closing before end_time.
            // Prevents anyone else from closing at all.
            if *accounts.crank.address() != TYCHE_CRANK_PUBKEY {
                return Err(TycheCoreError::InvalidCrankAuthority.into());
            }

            // 4: Time gate — competition must have actually expired.
            // Crank may call this slightly early due to block time variance —
            // reject until end_time is definitively reached.
            let clock = Clock::get()?;
            if clock.unix_timestamp < state.end_time {
                return Err(TycheCoreError::AuctionNotExpired.into());
            }

            // 5: Transition phase to Settling.
            // Settling signals that undelegation is in progress.
            // SettleCompetition reads this phase to confirm undelegation completed
            // before writing winner and final_amount.
            state.phase = Phase::Settling as u8;

        } // mutable borrow drops here — competition free to pass to CPI

        // 6: Commit and undelegate both competition and permission back to mainnet.
        //
        // Both accounts were delegated during ActivateCompetition — competition via
        // delegate_account and permission via DelegatePermissionCpiBuilder. Each has
        // its own delegation_record inside the PER. Both must be passed here so the
        // delegation program resolves both records. Omitting permission orphans it
        // inside the TEE with no path back to mainnet.
        //
        // AuctionState undelegation is handled separately by tyche-auction —
        // tyche-core only undelegates what it owns.
        commit_and_undelegate_accounts(
            accounts.crank,
            &[*accounts.competition, *accounts.permission],
            accounts.magic_context,
            accounts.magic_program,
        )?;

        Ok(())
    }
}
