use pinocchio::{
    AccountView, ProgramResult,
    error::ProgramError,
};
use tyche_common::phase::Phase;
use tyche_core::{
    discriminator::{COMPETITION_STATE, PARTICIPANT_RECORD},
    state::{
        competition::CompetitionState,
        participant::ParticipantRecord,
    },
};
use crate::{
    discriminator::ESCROW_VAULT,
    error::TycheEscrowError,
    state::vault::EscrowVault,
};

// ── Account context 

/// Validated account context for `Refund`.
///
/// Returns the full vault balance (bid amount + rent reserve) to the depositor.
/// Valid when the competition is `Cancelled` or when it is `Settled` and the
/// depositor did not win. Winners must use `Release` instead.
pub struct RefundAccounts<'a> {
    /// EscrowVault PDA — writable, drained and closed.
    pub vault:              &'a AccountView,
    /// Original depositor — writable signer, receives all lamports.
    pub depositor:          &'a AccountView,
    /// CompetitionState PDA — read-only, phase verified as Cancelled or Settled.
    pub competition:        &'a AccountView,
    /// ParticipantRecord PDA — read-only.
    /// Checked on Settled path to confirm is_winner == NOT_WINNER.
    /// Ignored (may be uninitialized) on Cancelled path.
    pub participant_record: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for RefundAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [vault, depositor, competition, participant_record, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // vault must be writable — lamports drained, data zeroed
        if !vault.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        // depositor must sign and be writable — only vault owner may refund
        if !depositor.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !depositor.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(Self { vault, depositor, competition, participant_record })
    }
}

// ── Instruction context 

/// Instruction context for `Refund`.
///
/// No args — all inputs read directly from account state.
pub struct RefundInstruction<'a> {
    pub accounts: RefundAccounts<'a>,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for RefundInstruction<'a> {
    type Error = ProgramError;

    fn try_from((accounts, _data): (&'a [AccountView], &'a [u8])) -> Result<Self, Self::Error> {
        let accounts = RefundAccounts::try_from(accounts)?;
        Ok(Self { accounts })
    }
}

// ── Handler 

impl<'a> RefundInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;

        // Cache pubkeys before borrows.
        let competition_pubkey = *accounts.competition.address();
        let depositor_pubkey   = *accounts.depositor.address();

        // 1: Read and validate vault — verify it belongs to this competition + depositor.
        {
            let data  = accounts.vault.try_borrow()?;
            let vault = bytemuck::try_from_bytes::<EscrowVault>(&*data)
                .map_err(|_| ProgramError::InvalidAccountData)?;

            if vault.discriminator != ESCROW_VAULT {
                return Err(TycheEscrowError::InvalidDiscriminator.into());
            }
            if vault.competition != competition_pubkey {
                return Err(ProgramError::InvalidAccountData);
            }
            if vault.depositor != depositor_pubkey {
                return Err(ProgramError::InvalidAccountData);
            }
        } // vault borrow drops here

        // 2: Verify competition phase.
        let phase = {
            let data  = accounts.competition.try_borrow()?;
            let state = bytemuck::try_from_bytes::<CompetitionState>(&*data)
                .map_err(|_| ProgramError::InvalidAccountData)?;

            if state.discriminator != COMPETITION_STATE {
                return Err(TycheEscrowError::InvalidDiscriminator.into());
            }

            state.phase
        }; // competition borrow drops here

        // 3: Phase-dependent winner check.
        if phase == Phase::Settled as u8 {
            // Settled path — check participant record.
            // Only non-winners may refund; winners must use Release.
            let data   = accounts.participant_record.try_borrow()?;
            let record = bytemuck::try_from_bytes::<ParticipantRecord>(&*data)
                .map_err(|_| ProgramError::InvalidAccountData)?;

            if record.discriminator != PARTICIPANT_RECORD {
                return Err(TycheEscrowError::InvalidDiscriminator.into());
            }
            if record.competition != competition_pubkey {
                return Err(ProgramError::InvalidAccountData);
            }
            if record.participant != depositor_pubkey {
                return Err(ProgramError::InvalidAccountData);
            }
            if record.is_winner == ParticipantRecord::IS_WINNER {
                return Err(TycheEscrowError::WinnerCannotRefund.into());
            }
            // NOT_WINNER confirmed — fall through to refund.

        } else if phase != Phase::Cancelled as u8 {
            // Only Settled and Cancelled allow refund.
            // Active / Scheduling / Settling → reject.
            return Err(TycheEscrowError::InvalidPhase.into());
        }
        // Cancelled path — no winner check needed. Any depositor may claim.

        // 4: Capture total vault lamports before draining.
        let vault_lamports = accounts.vault.lamports();

        // 5: Zero vault data.
        {
            let mut data = accounts.vault.try_borrow_mut()?;
            data.fill(0);
        } // mutable borrow drops here

        // 6: Return all lamports to depositor (bid amount + rent reserve).
        let new_depositor_lamports = accounts.depositor.lamports()
            .checked_add(vault_lamports)
            .ok_or(TycheEscrowError::ArithmeticOverflow)?;

        accounts.vault.set_lamports(0);
        accounts.depositor.set_lamports(new_depositor_lamports);

        Ok(())
    }
}
