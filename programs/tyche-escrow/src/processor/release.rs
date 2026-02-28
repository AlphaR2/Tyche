use pinocchio::{
    AccountView, ProgramResult,
    error::ProgramError,
};
use tyche_common::phase::Phase;
use tyche_core::{
    discriminator::{COMPETITION_STATE, PARTICIPANT_RECORD, PROTOCOL_CONFIG},
    state::{
        competition::CompetitionState,
        participant::ParticipantRecord,
        protocol_config::ProtocolConfig,
    },
};
use crate::{
    discriminator::ESCROW_VAULT,
    error::TycheEscrowError,
    state::vault::EscrowVault,
};

// ── Account context

/// Validated account context for `Release`.
///
/// Crank-only. Distributes vault funds after a competition settles.
/// `vault.amount` is the canonical purchase price — no caller-supplied amount
/// is accepted to prevent a malicious crank from under-paying the seller.
///
/// Lamport distribution:
/// - fee (`vault.amount × fee_basis_points / 10_000`) → treasury
/// - net bid (`vault.amount` − fee) → competition authority (seller)
/// - rent reserve (`vault.lamports()` − `vault.amount`) → original depositor
///
/// Vault is closed after this call.
pub struct ReleaseAccounts<'a> {

    pub vault:              &'a AccountView,
    pub authority:          &'a AccountView,
    pub depositor:          &'a AccountView,
    pub crank:              &'a AccountView,
    pub competition:        &'a AccountView,
    pub participant_record: &'a AccountView,
    pub protocol_config:    &'a AccountView,
    pub treasury:           &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for ReleaseAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [vault, authority, depositor, crank, competition, participant_record,
             protocol_config, treasury, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !vault.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !authority.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !depositor.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !crank.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !treasury.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(Self {
            vault, authority, depositor, crank, competition,
            participant_record, protocol_config, treasury,
        })
    }
}

// ── Instruction context

/// Instruction context for `Release`.
///
/// No args — all inputs read directly from account state.
pub struct ReleaseInstruction<'a> {
    pub accounts: ReleaseAccounts<'a>,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for ReleaseInstruction<'a> {
    type Error = ProgramError;

    fn try_from((accounts, _data): (&'a [AccountView], &'a [u8])) -> Result<Self, Self::Error> {
        let accounts = ReleaseAccounts::try_from(accounts)?;
        Ok(Self { accounts })
    }
}

// ── Handler

impl<'a> ReleaseInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;

        let competition_pubkey = *accounts.competition.address();
        let depositor_pubkey   = *accounts.depositor.address();

        // 1: Read and validate vault — verify it belongs to this competition + depositor.
        let vault_amount = {
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

            vault.amount
        }; // vault borrow drops here

        // 1a: vault.amount is the canonical purchase price — must be non-zero.
        if vault_amount == 0 {
            return Err(TycheEscrowError::InvalidAmount.into());
        }

        // 2: Read ProtocolConfig — get crank_authority, fee_basis_points, treasury pubkey.
        let (crank_authority, fee_basis_points, config_treasury) = {
            let data   = accounts.protocol_config.try_borrow()?;
            let config = bytemuck::try_from_bytes::<ProtocolConfig>(&*data)
                .map_err(|_| TycheEscrowError::InvalidProtocolConfig)?;

            if config.discriminator != PROTOCOL_CONFIG {
                return Err(TycheEscrowError::InvalidProtocolConfig.into());
            }

            (config.crank_authority, config.fee_basis_points, config.treasury)
        }; // config borrow drops here

        // 3: Verify competition — must be Settled. Extract authority for payout validation.
        let authority_pubkey = {
            let data  = accounts.competition.try_borrow()?;
            let state = bytemuck::try_from_bytes::<CompetitionState>(&*data)
                .map_err(|_| ProgramError::InvalidAccountData)?;

            if state.discriminator != COMPETITION_STATE {
                return Err(TycheEscrowError::InvalidDiscriminator.into());
            }
            if state.phase != Phase::Settled as u8 {
                return Err(TycheEscrowError::InvalidPhase.into());
            }

            state.authority
        }; // competition borrow drops here

        // 4: Verify the authority account matches competition.authority.
        if *accounts.authority.address() != authority_pubkey {
            return Err(ProgramError::InvalidAccountData);
        }

        // 5: Verify the treasury account matches config.treasury.
        if *accounts.treasury.address() != config_treasury {
            return Err(TycheEscrowError::InvalidTreasury.into());
        }

        // 6: Verify participant record — depositor must be IS_WINNER.
        {
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
            if record.is_winner != ParticipantRecord::IS_WINNER {
                return Err(TycheEscrowError::NotWinner.into());
            }
        } // participant_record borrow drops here

        // 7: Crank authority check — read from ProtocolConfig.
        if *accounts.crank.address() != crank_authority {
            return Err(TycheEscrowError::InvalidCrankAuthority.into());
        }

        // 8: Compute protocol fee on vault.amount (the canonical purchase price).
        //    fee = vault.amount * fee_basis_points / 10_000
        //    u128 intermediate prevents overflow for large amounts × high fee rates.
        let fee: u64 = if fee_basis_points == 0 {
            0
        } else {
            ((vault_amount as u128)
                .checked_mul(fee_basis_points as u128)
                .ok_or(TycheEscrowError::ArithmeticOverflow)?
                / 10_000_u128) as u64
        };

        // Net bid amount that goes to the competition authority (seller).
        let net_bid = vault_amount
            .checked_sub(fee)
            .ok_or(TycheEscrowError::ArithmeticOverflow)?;

        // 9: Compute what goes back to the depositor (winner).
        //    rent_back = vault.lamports() − vault.amount  (rent reserve returned)
        let vault_lamports = accounts.vault.lamports();
        let rent_back = vault_lamports
            .checked_sub(vault_amount)
            .ok_or(TycheEscrowError::ArithmeticOverflow)?;

        // 10: Zero vault data before draining lamports.
        // Prevents the closed account from appearing initialized if re-deposited.
        {
            let mut data = accounts.vault.try_borrow_mut()?;
            data.fill(0);
        } // mutable borrow drops here

        // 11: Move lamports — vault drained, split across authority, treasury, depositor.
        let new_authority_lamports = accounts.authority.lamports()
            .checked_add(net_bid)
            .ok_or(TycheEscrowError::ArithmeticOverflow)?;
        let new_treasury_lamports = accounts.treasury.lamports()
            .checked_add(fee)
            .ok_or(TycheEscrowError::ArithmeticOverflow)?;
        let new_depositor_lamports = accounts.depositor.lamports()
            .checked_add(rent_back)
            .ok_or(TycheEscrowError::ArithmeticOverflow)?;

        accounts.vault.set_lamports(0);
        accounts.authority.set_lamports(new_authority_lamports);
        // Only write to treasury if fee > 0 — skips a lamport write when fee_basis_points == 0.
        if fee > 0 {
            accounts.treasury.set_lamports(new_treasury_lamports);
        }
        accounts.depositor.set_lamports(new_depositor_lamports);

        Ok(())
    }
}
