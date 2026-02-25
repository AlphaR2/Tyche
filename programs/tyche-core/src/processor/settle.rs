use pinocchio::{
    AccountView, ProgramResult,
    error::ProgramError,
};
use tyche_common::{
    phase::Phase,
    constants::TYCHE_CRANK_PUBKEY,
};
use crate::{
    instruction_args::settle::SettleCompetitionArgs,
    discriminator::COMPETITION_STATE,
    error::TycheCoreError,
    state::competition::CompetitionState,
};

// ── Account context

/// Validated account context for `SettleCompetition`.
///
/// Runs on mainnet after `CloseCompetition` triggered `commit_and_undelegate_accounts`
/// and the undelegation callback completed. `CompetitionState` is back on mainnet
/// in `Settling` phase. Called by the crank which reads the vertical's result account
/// and supplies the `settlement_ref` pointing to it.
pub struct SettleCompetitionAccounts<'a> {
    /// `CompetitionState` PDA — writable, returned to mainnet after undelegation.
    pub competition:       &'a AccountView,
    /// Protocol crank — must sign. Trusted backend that reads vertical result
    /// and calls settle with the correct settlement_ref.
    pub crank:             &'a AccountView,
    /// Delegation record PDA — must have zero lamports, proving full undelegation.
    /// The delegation program closes this account when the PER session terminates.
    pub delegation_record: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for SettleCompetitionAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [competition, crank, delegation_record, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // competition must be writable — settlement_ref and phase are mutated
        if !competition.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        // crank must sign — trusted backend supplies settlement result
        if !crank.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            competition,
            crank,
            delegation_record,
        })
    }
}

// ── Instruction context ───────────────────────────────────────────────────────

/// Instruction context for `SettleCompetition`.
///
/// `settlement_ref` points to the vertical's own result account —
/// `AuctionState`, `PredictionState`, etc. `tyche-core` stores the reference
/// without interpreting it, keeping the state machine vertical-agnostic.
/// The crank reads the vertical's result account after undelegation and
/// supplies the reference here.
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

// ── Handler ───────────────────────────────────────────────────────────────────

impl<'a> SettleCompetitionInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;
        let args     = self.args;

        let mut data = accounts.competition.try_borrow_mut()?;
        let state    = bytemuck::from_bytes_mut::<CompetitionState>(&mut *data);

        // 1: Verify discriminator — reject accounts not initialized by this program.
        if state.discriminator != COMPETITION_STATE {
            return Err(TycheCoreError::InvalidDiscriminator.into());
        }

        // 2: Phase gate — must be Settling.
        // Settling is set by CloseCompetition immediately before triggering undelegation.
        // Active = session still live.
        // Settled = already finalized, reject duplicate settlement.
        // Cancelled = terminal, cannot settle.
        if state.phase != Phase::Settling as u8 {
            return Err(TycheCoreError::InvalidPhase.into());
        }

        // 3: Crank authority check — only the protocol crank may settle.
        // Prevents fabricated settlement_ref from being written by an arbitrary caller.
        // The crank is the trusted backend that verifies the vertical's result account
        // before supplying settlement_ref.
        if *accounts.crank.address() != TYCHE_CRANK_PUBKEY {
            return Err(TycheCoreError::InvalidCrankAuthority.into());
        }

        // 4: Undelegation proof — delegation_record must have zero lamports.
        // The delegation program closes delegation_record when the PER session
        // fully terminates and the undelegation callback completes.
        // Zero lamports = account closed = undelegation complete.
        // Reject if the session is still in progress — CompetitionState may still
        // be mid-flight between PER and mainnet.
        if accounts.delegation_record.lamports() != 0 {
            return Err(TycheCoreError::NotUndelegated.into());
        }

        // 5: Write settlement_ref to CompetitionState.
        // Points to the vertical's own result account as the canonical proof
        // of outcome. tyche-core does not read or validate the referenced account —
        // that is the vertical's responsibility. The reference is the permanent
        // on-chain record that this competition settled and where to find the result.
        state.settlement_ref = args.settlement_ref;

        // 6: Transition to Settled — terminal state.
        // tyche-auction gates FinalizeAuction on phase == Settled.
        // tyche-escrow gates ReleaseWinner and Refund on phase == Settled.
        // No further writes to CompetitionState are permitted after this point.
        state.phase = Phase::Settled as u8;

        Ok(())
    }
}