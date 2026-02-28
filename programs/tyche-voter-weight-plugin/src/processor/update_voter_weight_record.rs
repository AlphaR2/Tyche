use spl_governance_addin_api::voter_weight::VoterWeightRecord as SplVoterWeightRecord;
use pinocchio::{
    Address, AccountView, ProgramResult,
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
};
use crate::{
    args::update_voter_weight_record::UpdateVoterWeightRecordArgs,
    discriminator::REGISTRAR,
    error::PluginError,
    state::{
        voter_weight_record::{VoterWeightRecord, VOTER_WEIGHT_RECORD_MAX_SIZE},
        registrar::Registrar,
    },
    utils::pda::VOTER_WEIGHT_RECORD_SEED,
};

// ── EscrowVault byte offsets 
const VAULT_COMPETITION_OFFSET: usize = 8;
const VAULT_DEPOSITOR_OFFSET:   usize = 40;
const VAULT_AMOUNT_OFFSET:      usize = 72;
const VAULT_MIN_LEN:            usize = 80;

// ── Account context 

pub struct UpdateVoterWeightRecordAccounts<'a> {
    pub voter_weight_record: &'a AccountView,
    pub registrar:           &'a AccountView,
    pub escrow_vault:        &'a AccountView,
    pub voter_authority:     &'a AccountView,
    pub proposal:            &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for UpdateVoterWeightRecordAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [voter_weight_record, registrar, escrow_vault, voter_authority, proposal, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !voter_weight_record.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !voter_authority.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self { voter_weight_record, registrar, escrow_vault, voter_authority, proposal })
    }
}

// ── Instruction context 

pub struct UpdateVoterWeightRecordInstruction<'a> {
    pub accounts: UpdateVoterWeightRecordAccounts<'a>,
    pub args:     UpdateVoterWeightRecordArgs,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for UpdateVoterWeightRecordInstruction<'a> {
    type Error = ProgramError;

    fn try_from((accounts, data): (&'a [AccountView], &'a [u8])) -> Result<Self, Self::Error> {
        let accounts = UpdateVoterWeightRecordAccounts::try_from(accounts)?;
        let args     = UpdateVoterWeightRecordArgs::load(data)?;
        Ok(Self { accounts, args })
    }
}

// ── Handler 

impl<'a> UpdateVoterWeightRecordInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;
        let args     = &self.args;

        let voter_pubkey    = *accounts.voter_authority.address();
        let proposal_pubkey = *accounts.proposal.address();

        // 1: Load and validate the registrar.
        if unsafe {accounts.registrar.owner() != &crate::ID } {
            return Err(ProgramError::InvalidAccountOwner);
        }

        let escrow_program_id: [u8; 32];
        let competition:       [u8; 32];
        let realm_bytes:       [u8; 32];
        let mint_bytes:        [u8; 32];

        {
            let reg_data = accounts.registrar.try_borrow()?;
            let reg      = bytemuck::try_from_bytes::<Registrar>(&*reg_data)
                .map_err(|_| PluginError::InvalidRegistrar)?;

            if reg.discriminator != REGISTRAR {
                return Err(PluginError::InvalidDiscriminator.into());
            }

            escrow_program_id = reg.tyche_escrow_program_id.to_bytes();
            competition       = reg.competition.to_bytes();
            realm_bytes       = reg.realm.to_bytes();
            mint_bytes        = reg.governing_token_mint.to_bytes();
        }

        // 2: Verify vault ownership.
        if unsafe{ accounts.escrow_vault.owner() != &Address::new_from_array(escrow_program_id) } {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // 3: Read EscrowVault fields.
        let vault_competition: [u8; 32];
        let vault_depositor:   [u8; 32];
        let vault_amount:      u64;

        {
            let vault_data = accounts.escrow_vault.try_borrow()?;
            if vault_data.len() < VAULT_MIN_LEN {
                return Err(PluginError::InvalidVaultData.into());
            }

            vault_competition = vault_data[VAULT_COMPETITION_OFFSET..VAULT_COMPETITION_OFFSET + 32]
                .try_into().unwrap();
            vault_depositor = vault_data[VAULT_DEPOSITOR_OFFSET..VAULT_DEPOSITOR_OFFSET + 32]
                .try_into().unwrap();
            vault_amount = u64::from_le_bytes(
                vault_data[VAULT_AMOUNT_OFFSET..VAULT_AMOUNT_OFFSET + 8].try_into().unwrap()
            );
        }

        // 4: Validations.
        if vault_competition != competition {
            return Err(PluginError::WrongCompetition.into());
        }
        if vault_depositor != *voter_pubkey.as_array() {
            return Err(PluginError::WrongDepositor.into());
        }

        // 5: Verify PDA.
        let (expected_vwr, _) = Address::find_program_address(
            &[VOTER_WEIGHT_RECORD_SEED, &realm_bytes, &mint_bytes, voter_pubkey.as_array()],
            &crate::ID,
        );

        if expected_vwr != *accounts.voter_weight_record.address() {
            return Err(ProgramError::InvalidSeeds);
        }

        if unsafe {accounts.voter_weight_record.owner() != &crate::ID } {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // 6: Write updated record.
        let current_slot = Clock::get()?.slot;
        {
            let mut data = accounts.voter_weight_record.try_borrow_mut()?;
            if data.len() < VOTER_WEIGHT_RECORD_MAX_SIZE {
                return Err(ProgramError::InvalidAccountData);
            }

            let vwr = VoterWeightRecord {
                inner: SplVoterWeightRecord {
                    account_discriminator: spl_governance_addin_api::voter_weight::VoterWeightRecord::ACCOUNT_DISCRIMINATOR,
                    realm: realm_bytes.into(),
                    governing_token_mint: mint_bytes.into(),
                    governing_token_owner: (*voter_pubkey.as_array()).into(),
                    voter_weight: vault_amount,
                    voter_weight_expiry: Some(current_slot),
                    weight_action: Some(args.action.clone()),
                    weight_action_target: Some((*proposal_pubkey.as_array()).into()),
                    reserved: [0u8; 8],
                }
            };

            vwr.write_to(&mut *data);
        }

        Ok(())
    }
}


