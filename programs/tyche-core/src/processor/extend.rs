use pinocchio::{
    AccountView, ProgramResult,
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
};
use ephemeral_rollups_pinocchio::instruction::commit_accounts;
use tyche_common::{
    phase::Phase,
    constants::TYCHE_CRANK_PUBKEY,
};
use crate::{
    discriminator::COMPETITION_STATE,
    error::TycheCoreError,
    state::competition::CompetitionState,
};

// Account context

/// Validated account context for `ExtendCompetition`.
///
/// Runs inside the MagicBlock PER during the Active phase.
/// Called by the protocol crank when a bid lands within the soft-close window.
/// No user-facing accounts — this is a protocol-level operation.
pub struct ExtendCompetitionAccounts<'a> {
    /// `CompetitionState` PDA — writable, currently delegated to PER.
    pub competition:   &'a AccountView,
    /// Protocol crank keypair — must sign, verified against `TYCHE_CRANK_PUBKEY`.
    pub crank:         &'a AccountView,
    /// MagicBlock context account — required for `commit_accounts` CPI.
    pub magic_context: &'a AccountView,
    /// MagicBlock program — required for `commit_accounts` CPI.
    pub magic_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for ExtendCompetitionAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [competition, crank, magic_context, magic_program, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // competition must be writable — end_time and soft_close_count are mutated
        if !competition.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        // crank must sign — prevents unauthorized extensions
        if !crank.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            competition,
            crank,
            magic_context,
            magic_program,
        })
    }
}

// Instruction context

/// Instruction context for `ExtendCompetition`.
///
/// No args — all inputs read directly from `CompetitionState`.
/// The crank drives this instruction entirely from on-chain state.
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

// Handler 

impl<'a> ExtendCompetitionInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;

        // Scope mutable borrow — drop before commit_accounts CPI.
        {
            let mut data = accounts.competition.try_borrow_mut()?;
            let state    = bytemuck::from_bytes_mut::<CompetitionState>(&mut *data);

            // 1: Verify discriminator — reject accounts not initialized by this program.
            if state.discriminator != COMPETITION_STATE {
                return Err(TycheCoreError::InvalidDiscriminator.into());
            }

            // 2: Phase gate — only Active competitions can be extended.
            // Scheduled competitions have no end_time yet.
            // Settling/Settled/Cancelled competitions must not be touched.
            if state.phase != Phase::Active as u8 {
                return Err(TycheCoreError::InvalidPhase.into());
            }

            // 3: Crank authority check — only the protocol crank may extend.
            // TYCHE_CRANK_PUBKEY is the backend keypair that monitors bid activity
            // and calls this instruction when a bid lands in the soft-close window.
            // Per-competition authority cannot trigger this — prevents self-extension abuse.
            if *accounts.crank.address() != TYCHE_CRANK_PUBKEY {
                return Err(TycheCoreError::InvalidCrankAuthority.into());
            }

            // 4: Cap check — reject if max soft-close extensions already reached.
            // max_soft_closes == 0 means soft-close is disabled for this competition.
            // This check catches both the disabled case and the exhausted case.
            if state.soft_close_count >= state.max_soft_closes {
                return Err(TycheCoreError::SoftCloseCapReached.into());
            }

            // 5: Expiry check — competition must not already be over.
            // Guards against crank calling extend on an expired competition
            // before CloseCompetition has been called to transition the phase.
            let clock = Clock::get()?;
            if clock.unix_timestamp >= state.end_time {
                return Err(TycheCoreError::AuctionEnded.into());
            }

            // 6: Window check — bid must have landed inside the soft-close window.
            // window_start = end_time - soft_close_window.
            // If clock is before window_start, the bid landed too early to arm extension.
            let window_start = state.end_time
                .checked_sub(state.soft_close_window)
                .ok_or(TycheCoreError::ArithmeticOverflow)?;

            if clock.unix_timestamp < window_start {
                return Err(TycheCoreError::SoftCloseNotArmed.into());
            }

            // 7: Extend end_time by soft_close_extension seconds.
            // Computed with overflow protection — end_time is a protocol-critical value.
            state.end_time = state.end_time
                .checked_add(state.soft_close_extension)
                .ok_or(TycheCoreError::ArithmeticOverflow)?;

            // 8: Increment soft-close counter.
            // Saturating add would silently allow bypass of the cap check on overflow.
            // Checked add ensures this never wraps — soft_close_count is u8 max 255.
            state.soft_close_count = state.soft_close_count
                .checked_add(1)
                .ok_or(TycheCoreError::ArithmeticOverflow)?;

        } // mutable borrow drops here — competition is free to pass to commit CPI

        // 9: Commit CompetitionState to mainnet immediately.
        //
        // Bypasses the periodic commit_frequency_ms snapshot so bidders on mainnet
        // see the updated end_time without waiting for the next scheduled commit.
        // This is a simple commit — competition stays delegated, bidding continues.
        //
        // Note: Magic Actions (commit + attached base-layer instruction) would allow
        // automatic notification of the extension to off-chain indexers and other
        // programs immediately after commit. Not available in ephemeral_rollups_pinocchio
        // 0.8.2 — planned as a post-hackathon upgrade once Pinocchio support lands.
        commit_accounts(
            accounts.crank,
            &[*accounts.competition],
            accounts.magic_context,
            accounts.magic_program,
        )?;

        Ok(())
    }
}