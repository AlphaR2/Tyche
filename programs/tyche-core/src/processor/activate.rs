use pinocchio::{
    AccountView, ProgramResult,
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    cpi::{Seed, Signer},
};
use tyche_common::{phase::Phase, seeds::COMPETITION_SEED};
use crate::{
    instruction_args::activate::ActivateCompetitionArgs,
    discriminator::COMPETITION_STATE,
    error::TycheCoreError,
    state::competition::CompetitionState,
};

use ephemeral_rollups_pinocchio::{
    
    acl::{
        CreatePermissionCpiBuilder,
        DelegatePermissionCpiBuilder,
        Member,
        MemberFlags,
        MembersArgs
    }, instruction::delegate_account, types::DelegateConfig,
};

// Layer 1 — account validation: destructure raw slice, enforce writability and signer presence.
/// Validated account context for `ActivateCompetition`.
pub struct ActivateCompetitionAccounts<'a> {
    pub competition: &'a AccountView,
    pub authority:   &'a AccountView,
    // delegation program accounts
    //payer can be same as authority, making it open in scenrios where users fund via a relayer etc 
    pub payer:               &'a AccountView,
    pub permission:          &'a AccountView,
    pub delegation_buffer:   &'a AccountView,
    pub delegation_record:   &'a AccountView,
    pub delegation_metadata: &'a AccountView,
    pub delegation_program:  &'a AccountView,
    pub permission_program:  &'a AccountView,
    pub system_program:      &'a AccountView,
    pub validator:           &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for ActivateCompetitionAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [
            competition, 
            authority, 
            payer, 
            permission, 
            delegation_buffer, 
            delegation_record, 
            delegation_metadata, 
            delegation_program, 
            permission_program, 
            system_program, 
            validator, 
             _
            ] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);

            };

        // competition must be writable
        if !competition.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
         if competition.is_data_empty() {
            return Err(ProgramError::UninitializedAccount);
        }

        // authority must sign — checked again against state.authority in handler
        if !authority.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // payer must sign and be writable — funds permission account creation
        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !payer.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(Self { competition, authority, payer, permission, delegation_buffer, delegation_record, delegation_metadata, delegation_program, permission_program, system_program, validator })
    }
}

// Layer 2 — instruction context: bundle validated accounts with zero-copy args; entrypoint calls TryFrom.
/// Instruction context for `ActivateCompetition`.
pub struct ActivateCompetitionInstruction<'a> {
    pub accounts: ActivateCompetitionAccounts<'a>,
    pub args:     &'a ActivateCompetitionArgs,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for ActivateCompetitionInstruction<'a> {
    type Error = ProgramError;

    fn try_from((accounts, data): (&'a [AccountView], &'a [u8])) -> Result<Self, Self::Error> {
        let accounts = ActivateCompetitionAccounts::try_from(accounts)?;
        let args     = ActivateCompetitionArgs::load(data)?;
        Ok(Self { accounts, args })
    }
}

// Layer 3 — execution: state-level checks (discriminator → phase → authority → time) then mutations.
impl<'a> ActivateCompetitionInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;
        let args    = self.args;

    let (authority_bytes, id_bytes, bump) = {
        let mut data = accounts.competition.try_borrow_mut()?;
        let state    = bytemuck::from_bytes_mut::<CompetitionState>(&mut *data);
            // 1: Verify discriminator — reject accounts not initialized by this program.
        if state.discriminator != COMPETITION_STATE {
            return Err(TycheCoreError::InvalidDiscriminator.into());
        }

        // 2: Phase gate — only Scheduled competitions can be activated.
        if state.phase != Phase::Scheduled as u8 {
            return Err(TycheCoreError::InvalidPhase.into());
        }

        // 3: Authority check — only the creator may activate.
        if state.authority != *accounts.authority.address() {
            return Err(TycheCoreError::NotAuthority.into());
        }

        // 4: Time gate — activation only valid at or after start_time.
        let clock = Clock::get()?;
        if clock.unix_timestamp < state.start_time {
            return Err(TycheCoreError::AuctionNotStarted.into());
        }

        // 5: Compute end_time relative to actual activation moment, not scheduled start_time.
        // This guarantees the competition window is always exactly duration_secs long.
        let end_time = clock.unix_timestamp
            .checked_add(state.duration_secs)
            .ok_or(TycheCoreError::ArithmeticOverflow)?;

        // 6: Transition to Active.
        state.phase    = Phase::Active as u8;
        state.end_time = end_time;

        //Extract values needed by CPIs before borrow drops

        let authority_bytes = state.authority;
        let id_bytes        = state.id;
        let bump            = state.bump;

        (authority_bytes, id_bytes, bump)
    };  // mutable borrow on competition drops here — safe to pass to CPIs below

        //get the byte arrays for these 
        let authority_ref = authority_bytes.as_array();
        let id_ref        = id_bytes.as_array();
        let bump_binding = [bump];

        let seeds_array : [Seed; 4] = [
            Seed::from(COMPETITION_SEED),
            Seed::from(authority_ref),
            Seed::from(id_ref),
            Seed::from(&bump_binding),
        ];
        let signer = Signer::from(&seeds_array);

        // 7: Create permission for CompetitionState inside the TEE.
        // Members list defines who can read this account while it is delegated.
        // Authority is the only member(authority is creator of competition) — they can read competition state inside the enclave.
        // Bidders cannot read sealed fields (those live on AuctionState with zero members).

        if accounts.permission.lamports() == 0 {
            //set the members array and add authority
            let members_array = [ Member {
                flags: MemberFlags::default(),
                pubkey: *accounts.authority.address(),
            }];

            let memebers_args = MembersArgs{
                members: Some(&members_array),
            };

            CreatePermissionCpiBuilder::new(
                accounts.competition,
                accounts.permission,
                accounts.payer,
                accounts.system_program,
                accounts.permission_program.address(),
            ).members(memebers_args)
            .seeds(&[COMPETITION_SEED, authority_ref, id_ref])
            .bump(bump)
            .invoke()?;
        }

        // 8: Delegate permission to TEE validator.
        // Permission must be delegated before the account itself can be delegated.
        // Guard checks permission is still owned by permission program (not yet delegated).

        //we can do unsafe here because we are sure the permissions account exists and we can check .owner() ---- if accounts.permission.lamports() == 0 is the initial check
            if unsafe {accounts.permission.owner()} == accounts.permission_program.address() {
                DelegatePermissionCpiBuilder::new(
                accounts.payer,
                accounts.payer,
                accounts.competition,
                accounts.permission,
                accounts.system_program,
                accounts.permission_program,
                accounts.delegation_buffer,
                accounts.delegation_record,
                accounts.delegation_metadata,
                accounts.delegation_program,
                accounts.validator,
                accounts.permission_program.address(),
                )
                .signer_seeds(signer)
                .invoke()?;
                
            }

            // 9: Delegate CompetitionState to the PER.
        // After this CPI returns, competition is live inside the TEE.
        // commit_frequency_ms controls how often the PER snapshots state to mainnet.

        let delegate_config = DelegateConfig{
            validator: Some(*accounts.validator.address()),
            commit_frequency_ms: args.commit_frequency_ms,
            ..Default::default()
        };

        delegate_account(
            &[
                accounts.payer,
                accounts.competition,
                accounts.delegation_program,  // owner_program slot
                accounts.delegation_buffer,
                accounts.delegation_record,
                accounts.delegation_metadata,
                accounts.system_program,
            ],
            &[COMPETITION_SEED, authority_ref, id_ref],
            bump, 
            delegate_config
        )?;

        Ok(())
    }
}
