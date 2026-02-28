use pinocchio::{
    AccountView, ProgramResult,
    error::ProgramError,
};
use crate::{
    discriminator::PROTOCOL_CONFIG,
    error::TycheCoreError,
    instruction_args::update_crank_authority::UpdateCrankAuthorityArgs,
    state::protocol_config::ProtocolConfig,
};

// ── Account context 

/// Validated account context for `UpdateCrankAuthority`.
///
/// Replaces `config.crank_authority` with a new keypair. Kept separate from
/// `UpdateProtocolConfig` to allow narrower key delegation — an ops keypair
/// can rotate the crank without touching fee or treasury parameters.
pub struct UpdateCrankAuthorityAccounts<'a> {
    /// `ProtocolConfig` PDA — writable.
    pub protocol_config: &'a AccountView,
    /// Must match `config.authority`.
    pub authority:       &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for UpdateCrankAuthorityAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [protocol_config, authority, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !protocol_config.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        if !authority.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self { protocol_config, authority })
    }
}

// ── Instruction context 

pub struct UpdateCrankAuthorityInstruction<'a> {
    pub accounts: UpdateCrankAuthorityAccounts<'a>,
    pub args:     &'a UpdateCrankAuthorityArgs,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for UpdateCrankAuthorityInstruction<'a> {
    type Error = ProgramError;

    fn try_from((accounts, data): (&'a [AccountView], &'a [u8])) -> Result<Self, Self::Error> {
        let accounts = UpdateCrankAuthorityAccounts::try_from(accounts)?;
        let args     = UpdateCrankAuthorityArgs::load(data)?;
        Ok(Self { accounts, args })
    }
}

// ── Handler 

impl<'a> UpdateCrankAuthorityInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;
        let args     = self.args;

        let mut data = accounts.protocol_config.try_borrow_mut()?;
        let config   = bytemuck::from_bytes_mut::<ProtocolConfig>(&mut *data);

        // 1: Verify discriminator — reject non-ProtocolConfig accounts.
        if config.discriminator != PROTOCOL_CONFIG {
            return Err(TycheCoreError::InvalidDiscriminator.into());
        }

        // 2: Authority check — only config.authority may rotate the crank.
        if *accounts.authority.address() != config.authority {
            return Err(TycheCoreError::NotConfigAuthority.into());
        }

        // 3: Apply the new crank authority.
        config.crank_authority = args.new_crank_authority;

        Ok(())
    }
}
