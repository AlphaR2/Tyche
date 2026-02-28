use pinocchio::{
    Address, AccountView, ProgramResult,
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
};
use crate::{
    discriminator::{MAX_VOTER_WEIGHT_RECORD, REGISTRAR},
    error::PluginError,
    state::{
        registrar::Registrar,
        max_voter_weight_record::{write_max_voter_weight_record, MAX_VOTER_WEIGHT_RECORD_SIZE},
    },
    utils::pda::MAX_VOTER_WEIGHT_RECORD_SEED,
};

// ── Account context 

pub struct UpdateMaxVoterWeightRecordAccounts<'a> {
    pub max_voter_weight_record: &'a AccountView,
    pub registrar:               &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for UpdateMaxVoterWeightRecordAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [max_voter_weight_record, registrar, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !max_voter_weight_record.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(Self { max_voter_weight_record, registrar })
    }
}

// ── Instruction context 

pub struct UpdateMaxVoterWeightRecordInstruction<'a> {
    pub accounts: UpdateMaxVoterWeightRecordAccounts<'a>,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for UpdateMaxVoterWeightRecordInstruction<'a> {
    type Error = ProgramError;

    fn try_from((accounts, _data): (&'a [AccountView], &'a [u8])) -> Result<Self, Self::Error> {
        let accounts = UpdateMaxVoterWeightRecordAccounts::try_from(accounts)?;
        Ok(Self { accounts })
    }
}

// ── Handler 

impl<'a> UpdateMaxVoterWeightRecordInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;

        // 1: Validate registrar.
        if unsafe { accounts.registrar.owner() } != &crate::ID {
            return Err(ProgramError::InvalidAccountOwner);
        }

        let realm_bytes: [u8; 32];
        let mint_bytes:  [u8; 32];

        {
            let reg_data = accounts.registrar.try_borrow()?;
            let reg      = bytemuck::try_from_bytes::<Registrar>(&*reg_data)
                .map_err(|_| PluginError::InvalidRegistrar)?;

            if reg.discriminator != REGISTRAR {
                return Err(PluginError::InvalidDiscriminator.into());
            }

            realm_bytes = reg.realm.to_bytes();
            mint_bytes  = reg.governing_token_mint.to_bytes();
        }

        // 2: Verify max-voter-weight PDA.
        let (expected_mvwr, _) = Address::find_program_address(
            &[&realm_bytes, MAX_VOTER_WEIGHT_RECORD_SEED, &mint_bytes],
            &crate::ID,
        );

        if expected_mvwr != *accounts.max_voter_weight_record.address() {
            return Err(ProgramError::InvalidSeeds);
        }

        // 3: Write record.
        //    max_voter_weight = u64::MAX signals that governance must use absolute
        //    vote-count thresholds rather than percentage-based quorum.
        let current_slot = Clock::get()?.slot;
        {
            let mut data = accounts.max_voter_weight_record.try_borrow_mut()?;
            if data.len() < MAX_VOTER_WEIGHT_RECORD_SIZE {
                return Err(ProgramError::InvalidAccountData);
            }

            write_max_voter_weight_record(
                &mut *data,
                &MAX_VOTER_WEIGHT_RECORD,
                &realm_bytes,
                &mint_bytes,
                u64::MAX,
                Some(current_slot),
            );
        }

        Ok(())
    }
}
