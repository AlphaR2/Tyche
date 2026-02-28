use pinocchio::{
    Address, AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
};
use pinocchio_system::instructions::{CreateAccount, Transfer};
use tyche_common::{
    phase::Phase,
    seeds::VAULT_SEED,
};
use tyche_core::{
    discriminator::COMPETITION_STATE,
    state::competition::CompetitionState,
};
use crate::{
    args::deposit::DepositArgs,
    discriminator::ESCROW_VAULT,
    error::TycheEscrowError,
    state::vault::EscrowVault,
};

// ── Account context 

/// Validated account context for `Deposit`.
///
/// Creates a new `EscrowVault` PDA on the depositor's first bid for a competition,
/// or tops up an existing vault on repeat bids. The vault lamports always equal
/// `rent_exempt_reserve + cumulative_bid_amount`.
pub struct DepositAccounts<'a> {
    /// EscrowVault PDA — writable. Created on first call, updated on top-up.
    pub vault:          &'a AccountView,
    /// Bidder — writable signer. Pays the bid amount via system Transfer CPI.
    pub depositor:      &'a AccountView,
    /// Rent payer — writable signer. Funds vault rent on first deposit only.
    pub payer:          &'a AccountView,
    /// CompetitionState PDA — read-only. Phase verified as Active.
    pub competition:    &'a AccountView,
    /// System program — required for CreateAccount and Transfer CPIs.
    pub system_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for DepositAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [vault, depositor, payer, competition, system_program, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // vault must be writable — created or updated by this instruction
        if !vault.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        // depositor must sign and be writable — pays bid amount
        if !depositor.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !depositor.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        // payer must sign and be writable — funds rent on first deposit
        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !payer.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(Self { vault, depositor, payer, competition, system_program })
    }
}

// ── Instruction context 

/// Instruction context for `Deposit`.
pub struct DepositInstruction<'a> {
    pub accounts: DepositAccounts<'a>,
    pub args:     &'a DepositArgs,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for DepositInstruction<'a> {
    type Error = ProgramError;

    fn try_from((accounts, data): (&'a [AccountView], &'a [u8])) -> Result<Self, Self::Error> {
        let accounts = DepositAccounts::try_from(accounts)?;
        let args     = DepositArgs::load(data)?;
        Ok(Self { accounts, args })
    }
}

// ── Handler 

impl<'a> DepositInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;
        let args     = self.args;

        // 1: Reject zero-amount deposits.
        if args.amount == 0 {
            return Err(TycheEscrowError::InvalidAmount.into());
        }

        // Cache pubkeys before any borrows.
        let competition_pubkey = *accounts.competition.address();
        let depositor_pubkey   = *accounts.depositor.address();

        // 2: Verify competition state — Active phase only.
        // Deposits are only accepted while bidding is live inside the PER.
        {
            let data  = accounts.competition.try_borrow()?;
            let state = bytemuck::try_from_bytes::<CompetitionState>(&*data)
                .map_err(|_| ProgramError::InvalidAccountData)?;

            if state.discriminator != COMPETITION_STATE {
                return Err(TycheEscrowError::InvalidDiscriminator.into());
            }
            if state.phase != Phase::Active as u8 {
                return Err(TycheEscrowError::InvalidPhase.into());
            }
        } // competition borrow drops here

        // 3: Derive and verify vault PDA.
        // Canonical bump is stored in EscrowVault so processors never call
        // find_program_address again — but we always derive on first deposit.
        let competition_bytes = competition_pubkey.as_array();
        let depositor_bytes   = depositor_pubkey.as_array();

        let (expected_pda, bump) = Address::find_program_address(
            &[VAULT_SEED, competition_bytes, depositor_bytes],
            &crate::ID,
        );

        if expected_pda.ne(accounts.vault.address()) {
            return Err(ProgramError::InvalidSeeds);
        }

        // 4: First deposit or top-up?
        let is_first_deposit = accounts.vault.is_data_empty();

        if is_first_deposit {
            // 4a: Allocate EscrowVault via system program CreateAccount CPI.
            // invoke_signed uses PDA seeds so the system program accepts
            // this program as the rightful creator of the vault address.
            let space    = EscrowVault::LEN;
            let lamports = Rent::get()?.try_minimum_balance(space)?;

            CreateAccount {
                from:  accounts.payer,
                to:    accounts.vault,
                space: space as u64,
                lamports,
                owner: &crate::ID,
            }
            .invoke_signed(&[Signer::from(&[
                Seed::from(VAULT_SEED),
                Seed::from(competition_bytes),
                Seed::from(depositor_bytes),
                Seed::from(&[bump]),
            ])])?;

            // 4b: Transfer bid amount from depositor to vault.
            // depositor signs the transaction so the system program accepts
            // the transfer. The vault's owner (tyche-escrow) is irrelevant to
            // system Transfer — only `from` must sign and be system-owned.
            Transfer {
                from:     accounts.depositor,
                to:       accounts.vault,
                lamports: args.amount,
            }
            .invoke()?;

            // 4c: Initialize EscrowVault fields.
            {
                let mut data = accounts.vault.try_borrow_mut()?;
                let vault    = bytemuck::from_bytes_mut::<EscrowVault>(&mut *data);

                vault.discriminator = ESCROW_VAULT;
                vault.competition   = competition_pubkey;
                vault.depositor     = depositor_pubkey;
                vault.amount        = args.amount;
                vault.bump          = bump;
            }

        } else {
            // 4d: Top-up — verify this vault belongs to this competition + depositor.
            // Checking the stored fields is sufficient after discriminator check:
            // only tyche-escrow can have written them (it owns the account).
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
            } // immutable borrow drops here

            // 4e: Transfer additional bid amount from depositor to vault.
            Transfer {
                from:     accounts.depositor,
                to:       accounts.vault,
                lamports: args.amount,
            }
            .invoke()?;

            // 4f: Accumulate bid amount.
            {
                let mut data = accounts.vault.try_borrow_mut()?;
                let vault    = bytemuck::from_bytes_mut::<EscrowVault>(&mut *data);

                vault.amount = vault.amount
                    .checked_add(args.amount)
                    .ok_or(TycheEscrowError::ArithmeticOverflow)?;
            }
        }

        Ok(())
    }
}
