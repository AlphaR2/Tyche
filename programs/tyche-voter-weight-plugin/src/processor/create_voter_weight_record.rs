use spl_governance_addin_api::voter_weight::VoterWeightRecord as SplVoterWeightRecord;
use pinocchio::{
    Address, AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
};
use pinocchio_system::instructions::CreateAccount;
use crate::{
    discriminator::REGISTRAR,
    error::PluginError,
    state::{
        registrar::Registrar,
        voter_weight_record::{VoterWeightRecord, VOTER_WEIGHT_RECORD_MAX_SIZE},
    },
    utils::pda::{REGISTRAR_SEED, VOTER_WEIGHT_RECORD_SEED},
};

// ── Account context 

pub struct CreateVoterWeightRecordAccounts<'a> {
    pub voter_weight_record:  &'a AccountView,
    pub registrar:            &'a AccountView,
    pub realm:                &'a AccountView,
    pub governing_token_mint: &'a AccountView,
    pub voter_authority:      &'a AccountView,
    pub payer:                &'a AccountView,
    pub system_program:       &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for CreateVoterWeightRecordAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [voter_weight_record, registrar, realm, governing_token_mint,
             voter_authority, payer, system_program, ..] = accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !voter_weight_record.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !voter_authority.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            voter_weight_record,
            registrar,
            realm,
            governing_token_mint,
            voter_authority,
            payer,
            system_program,
        })
    }
}

// ── Instruction context 

pub struct CreateVoterWeightRecordInstruction<'a> {
    pub accounts: CreateVoterWeightRecordAccounts<'a>,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for CreateVoterWeightRecordInstruction<'a> {
    type Error = ProgramError;

    fn try_from((accounts, _data): (&'a [AccountView], &'a [u8])) -> Result<Self, Self::Error> {
        let accounts = CreateVoterWeightRecordAccounts::try_from(accounts)?;
        Ok(Self { accounts })
    }
}

// ── Handler 

impl<'a> CreateVoterWeightRecordInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;

        let realm_pubkey  = *accounts.realm.address();
        let mint_pubkey   = *accounts.governing_token_mint.address();
        let voter_pubkey  = *accounts.voter_authority.address();

        let realm_bytes  = realm_pubkey.as_array();
        let mint_bytes   = mint_pubkey.as_array();
        let voter_bytes  = voter_pubkey.as_array();

        // 1: Load and validate the registrar.
        if unsafe{ accounts.registrar.owner() != &crate::ID } {
            return Err(ProgramError::InvalidAccountOwner);
        }

        {
            let reg_data = accounts.registrar.try_borrow()?;
            let reg      = bytemuck::try_from_bytes::<Registrar>(&*reg_data)
                .map_err(|_| PluginError::InvalidRegistrar)?;

            if reg.discriminator != REGISTRAR {
                return Err(PluginError::InvalidDiscriminator.into());
            }

            if &reg.realm.to_bytes() != realm_bytes {
                return Err(ProgramError::InvalidArgument);
            }
            if &reg.governing_token_mint.to_bytes() != mint_bytes {
                return Err(ProgramError::InvalidArgument);
            }

            let (expected_registrar, _) = Address::find_program_address(
                &[realm_bytes, REGISTRAR_SEED, mint_bytes],
                &crate::ID,
            );
            if expected_registrar != *accounts.registrar.address() {
                return Err(ProgramError::InvalidSeeds);
            }
        }

        // 2: Derive and verify VoterWeightRecord PDA.
        let (expected_vwr, bump) = Address::find_program_address(
            &[VOTER_WEIGHT_RECORD_SEED, realm_bytes, mint_bytes, voter_bytes],
            &crate::ID,
        );

        if expected_vwr != *accounts.voter_weight_record.address() {
            return Err(ProgramError::InvalidSeeds);
        }

        // 3: Reject if record already exists.
        if !accounts.voter_weight_record.is_data_empty() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        // 4: Allocate VoterWeightRecord via system program CreateAccount CPI.
        let space    = VOTER_WEIGHT_RECORD_MAX_SIZE;
        let lamports = Rent::get()?.try_minimum_balance(space)?;

        CreateAccount {
            from:     accounts.payer,
            to:       accounts.voter_weight_record,
            space:    space as u64,
            lamports,
            owner:    &crate::ID,
        }
        .invoke_signed(&[Signer::from(&[
            Seed::from(VOTER_WEIGHT_RECORD_SEED),
            Seed::from(realm_bytes),
            Seed::from(mint_bytes),
            Seed::from(voter_bytes),
            Seed::from(&[bump]),
        ])])?;

        // 5: Write an initial expired zero-weight record.
        {
            let mut data = accounts.voter_weight_record.try_borrow_mut()?;
            let vwr      = VoterWeightRecord {
                inner: SplVoterWeightRecord {
                    account_discriminator: spl_governance_addin_api::voter_weight::VoterWeightRecord::ACCOUNT_DISCRIMINATOR,
                    realm: (*realm_bytes).into(),
                    governing_token_mint: (*mint_bytes).into(),
                    governing_token_owner: (*voter_bytes).into(),
                    voter_weight: 0,
                    voter_weight_expiry: Some(0),
                    weight_action: None,
                    weight_action_target: None,
                    reserved: [0u8; 8],
                }
            };
            VoterWeightRecord::write_to(&vwr, &mut *data);
        }

        Ok(())
    }
}


