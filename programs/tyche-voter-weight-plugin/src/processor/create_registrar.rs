use pinocchio::{
    Address, AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
};
use pinocchio_system::instructions::CreateAccount;
use crate::{
    args::create_registrar::CreateRegistrarArgs,
    discriminator::{MAX_VOTER_WEIGHT_RECORD, REGISTRAR},
    error::PluginError,
    state::{
        registrar::Registrar,
        max_voter_weight_record::{write_max_voter_weight_record, MAX_VOTER_WEIGHT_RECORD_SIZE},
    },
    utils::pda::{MAX_VOTER_WEIGHT_RECORD_SEED, REGISTRAR_SEED},
};

// ── Account context 
//
// Accounts (7):
//   [0] registrar               writable  PDA to be created
//   [1] max_voter_weight_record writable  PDA to be created alongside registrar
//   [2] realm                   readable
//   [3] governing_token_mint    readable
//   [4] realm_authority         signer
//   [5] payer                   signer + writable
//   [6] system_program

pub struct CreateRegistrarAccounts<'a> {
    pub registrar:               &'a AccountView,
    pub max_voter_weight_record: &'a AccountView,
    pub realm:                   &'a AccountView,
    pub governing_token_mint:    &'a AccountView,
    pub realm_authority:         &'a AccountView,
    pub payer:                   &'a AccountView,
    pub system_program:          &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for CreateRegistrarAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [registrar, max_voter_weight_record, realm, governing_token_mint,
             realm_authority, payer, system_program, ..] = accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !registrar.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !max_voter_weight_record.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !realm_authority.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            registrar,
            max_voter_weight_record,
            realm,
            governing_token_mint,
            realm_authority,
            payer,
            system_program,
        })
    }
}

// ── Instruction context 

pub struct CreateRegistrarInstruction<'a> {
    pub accounts: CreateRegistrarAccounts<'a>,
    pub args:     &'a CreateRegistrarArgs,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for CreateRegistrarInstruction<'a> {
    type Error = ProgramError;

    fn try_from((accounts, data): (&'a [AccountView], &'a [u8])) -> Result<Self, Self::Error> {
        let accounts = CreateRegistrarAccounts::try_from(accounts)?;
        let args     = CreateRegistrarArgs::load(data)?;
        Ok(Self { accounts, args })
    }
}

// ── Handler 

impl<'a> CreateRegistrarInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;
        let args     = self.args;

        let realm_pubkey = *accounts.realm.address();
        let mint_pubkey  = *accounts.governing_token_mint.address();

        let realm_bytes = realm_pubkey.as_array();
        let mint_bytes  = mint_pubkey.as_array();

        // 1: Derive and verify Registrar PDA.
        let (expected_registrar, reg_bump) = Address::find_program_address(
            &[realm_bytes, REGISTRAR_SEED, mint_bytes],
            &crate::ID,
        );

        if expected_registrar != *accounts.registrar.address() {
            return Err(ProgramError::InvalidSeeds);
        }

        if !accounts.registrar.is_data_empty() {
            return Err(PluginError::InvalidRegistrar.into());
        }

        // 2: Derive and verify MaxVoterWeightRecord PDA.
        let (expected_mvwr, mvwr_bump) = Address::find_program_address(
            &[realm_bytes, MAX_VOTER_WEIGHT_RECORD_SEED, mint_bytes],
            &crate::ID,
        );

        if expected_mvwr != *accounts.max_voter_weight_record.address() {
            return Err(ProgramError::InvalidSeeds);
        }

        let rent = Rent::get()?;

        // 3: Allocate Registrar.
        let reg_lamports = rent.try_minimum_balance(Registrar::LEN)?;

        CreateAccount {
            from:     accounts.payer,
            to:       accounts.registrar,
            space:    Registrar::LEN as u64,
            lamports: reg_lamports,
            owner:    &crate::ID,
        }
        .invoke_signed(&[Signer::from(&[
            Seed::from(realm_bytes),
            Seed::from(REGISTRAR_SEED),
            Seed::from(mint_bytes),
            Seed::from(&[reg_bump]),
        ])])?;

        // 4: Write Registrar fields.
        {
            let mut data = accounts.registrar.try_borrow_mut()?;
            let reg      = bytemuck::from_bytes_mut::<Registrar>(&mut *data);

            reg.discriminator           = REGISTRAR;
            reg.governance_program_id   = args.governance_program_id;
            reg.realm                   = realm_pubkey;
            reg.governing_token_mint    = mint_pubkey;
            reg.prev_plugin_program_id  = Address::default();
            reg.has_prev_plugin         = 0;
            reg._pad0                   = [0u8; 7];
            reg.tyche_escrow_program_id = args.tyche_escrow_program;
            reg.competition             = args.competition;
            reg.bump                    = reg_bump;
            reg._pad1                   = [0u8; 7];
            reg.reserved                = [0u8; 128];
        }

        // 5: Allocate MaxVoterWeightRecord alongside the registrar.
        //    This ensures UpdateMaxVoterWeightRecord always has a live account.
        let mvwr_lamports = rent.try_minimum_balance(MAX_VOTER_WEIGHT_RECORD_SIZE)?;

        CreateAccount {
            from:     accounts.payer,
            to:       accounts.max_voter_weight_record,
            space:    MAX_VOTER_WEIGHT_RECORD_SIZE as u64,
            lamports: mvwr_lamports,
            owner:    &crate::ID,
        }
        .invoke_signed(&[Signer::from(&[
            Seed::from(realm_bytes),
            Seed::from(MAX_VOTER_WEIGHT_RECORD_SEED),
            Seed::from(mint_bytes),
            Seed::from(&[mvwr_bump]),
        ])])?;

        // 6: Write initial MaxVoterWeightRecord.
        //    max_voter_weight = u64::MAX — governance uses absolute thresholds.
        //    expiry = None; UpdateMaxVoterWeightRecord stamps the current slot later.
        {
            let mut data = accounts.max_voter_weight_record.try_borrow_mut()?;
            write_max_voter_weight_record(
                &mut *data,
                &MAX_VOTER_WEIGHT_RECORD,
                &realm_pubkey.to_bytes(),
                &mint_pubkey.to_bytes(),
                u64::MAX,
                None,
            );
        }

        Ok(())
    }
}
