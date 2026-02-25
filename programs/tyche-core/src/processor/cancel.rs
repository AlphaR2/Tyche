use pinocchio::{
    AccountView, ProgramResult,
    error::ProgramError,
};
use ephemeral_rollups_pinocchio::instruction::commit_and_undelegate_accounts;
use tyche_common::phase::Phase;
use crate::{
    discriminator::COMPETITION_STATE,
    error::TycheCoreError,
    state::competition::CompetitionState,
};

// Layer 1 — account validation: competition writable, authority signer.
/// Validated account context for `CancelCompetition`.
///
/// Valid from two phases:
/// - `Scheduled` — never delegated. Pure state write on mainnet, no CPI.
/// - `Active` with `participant_count == 0` — delegated but unused. Runs inside
///   the PER; `commit_and_undelegate_accounts` terminates the session with
///   `Cancelled` written as the final committed state.
///
/// `permission`, `magic_context`, and `magic_program` are always in the account list.
/// On the Scheduled path they are passed but never touched.
/// On the Active path both `competition` and `permission` are undelegated together —
/// `permission` was delegated during `ActivateCompetition` and has its own
/// `delegation_record` that must be resolved or it is orphaned inside the TEE.
pub struct CancelCompetitionAccounts<'a> {
    /// `CompetitionState` PDA — writable.
    pub competition:   &'a AccountView,
    /// Competition authority — must match `state.authority`. Signs cancellation.
    pub authority:     &'a AccountView,
    /// ACL permission PDA — delegated during activation, must be undelegated on Active path.
    pub permission:    &'a AccountView,
    /// MagicBlock context account — required by `commit_and_undelegate` on Active path.
    pub magic_context: &'a AccountView,
    /// MagicBlock program — required by `commit_and_undelegate` on Active path.
    pub magic_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for CancelCompetitionAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [competition, authority, permission, magic_context, magic_program, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // competition must be writable — phase is mutated to Cancelled
        if !competition.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        // authority must sign — only the creator may cancel
        if !authority.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self { competition, authority, permission, magic_context, magic_program })
    }
}

// Layer 2 — instruction context: no args, all inputs from CompetitionState.
/// Instruction context for `CancelCompetition`.
///
/// No args — phase and `participant_count` are read directly from `CompetitionState`.
pub struct CancelCompetitionInstruction<'a> {
    pub accounts: CancelCompetitionAccounts<'a>,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for CancelCompetitionInstruction<'a> {
    type Error = ProgramError;

    fn try_from(
        (accounts, _data): (&'a [AccountView], &'a [u8]),
    ) -> Result<Self, Self::Error> {
        let accounts = CancelCompetitionAccounts::try_from(accounts)?;
        Ok(Self { accounts })
    }
}

// Layer 3 — execution: discriminator → phase → authority → write Cancelled → conditionally undelegate.
impl<'a> CancelCompetitionInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;

        // Scope mutable borrow — must drop before commit_and_undelegate CPI on Active path.
        let was_active = {
            let mut data = accounts.competition.try_borrow_mut()?;
            let state    = bytemuck::from_bytes_mut::<CompetitionState>(&mut *data);

            // 1: Verify discriminator — reject accounts not initialized by this program.
            if state.discriminator != COMPETITION_STATE {
                return Err(TycheCoreError::InvalidDiscriminator.into());
            }

            // 2: Phase gate — Scheduled or Active with zero participants only.
            // Settling/Settled: session already finalized or in progress — cannot cancel.
            // Cancelled: already terminal.
            // Active with participants: escrow vaults exist — must go through settle/refund path.
            if state.phase == Phase::Scheduled as u8 {
                // Scheduled: never delegated — no CPI needed, fall through.
            } else if state.phase == Phase::Active as u8 {
                // Active is only cancellable if no bids have been placed.
                // participant_count > 0 means EscrowVault accounts exist and hold SOL —
                // cancelling without settlement would strand funds.
                if state.participant_count > 0 {
                    return Err(TycheCoreError::HasParticipants.into());
                }
                // Active with zero participants: session can be terminated cleanly.
            } else {
                return Err(TycheCoreError::InvalidPhase.into());
            }

            // 3: Authority check — only the original creator may cancel.
            if state.authority != *accounts.authority.address() {
                return Err(TycheCoreError::NotAuthority.into());
            }

            // 4: Capture phase before mutating — determines CPI path after borrow drops.
            let was_active = state.phase == Phase::Active as u8;

            // 5: Write Cancelled before the borrow drops.
            // On the Scheduled path this is the final write — done.
            // On the Active path commit_and_undelegate pushes this state to mainnet,
            // so Cancelled is the final committed state in both cases.
            state.phase = Phase::Cancelled as u8;

            was_active
        }; // mutable borrow drops here — competition free to pass to CPI

        // 6: Active path — terminate the PER session.
        // Both competition and permission must be undelegated together.
        // permission was delegated during ActivateCompetition and holds its own
        // delegation_record — omitting it here would orphan it inside the TEE.
        // Scheduled path skips this entirely — neither account was ever delegated.
        if was_active {
            commit_and_undelegate_accounts(
                accounts.authority,
                &[*accounts.competition, *accounts.permission],
                accounts.magic_context,
                accounts.magic_program,
            )?;
        }

        Ok(())
    }
}
