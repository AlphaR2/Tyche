use pinocchio::{
    AccountView, ProgramResult,
    error::ProgramError,
};
use tyche_common::constants::MAX_FEE_BASIS_POINTS;
use crate::{
    discriminator::PROTOCOL_CONFIG,
    error::TycheCoreError,
    instruction_args::update_protocol_config::UpdateProtocolConfigArgs,
    state::protocol_config::ProtocolConfig,
};

// ── Account context 

/// Validated account context for `UpdateProtocolConfig`.
///
/// Mutates treasury, fee, soft-close cap, reserve-price, and duration-floor
/// in the singleton `ProtocolConfig`. Authority-gated — only
/// `config.authority` may call this.
pub struct UpdateProtocolConfigAccounts<'a> {
    /// `ProtocolConfig` PDA — writable.
    pub protocol_config: &'a AccountView,
    /// Must match `config.authority`.
    pub authority:       &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for UpdateProtocolConfigAccounts<'a> {
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

pub struct UpdateProtocolConfigInstruction<'a> {
    pub accounts: UpdateProtocolConfigAccounts<'a>,
    pub args:     &'a UpdateProtocolConfigArgs,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for UpdateProtocolConfigInstruction<'a> {
    type Error = ProgramError;

    fn try_from((accounts, data): (&'a [AccountView], &'a [u8])) -> Result<Self, Self::Error> {
        let accounts = UpdateProtocolConfigAccounts::try_from(accounts)?;
        let args     = UpdateProtocolConfigArgs::load(data)?;
        Ok(Self { accounts, args })
    }
}

// ── Handler 

impl<'a> UpdateProtocolConfigInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;
        let args     = self.args;

        // 1: Validate fee ceiling — same hard limit as initialization.
        if args.new_fee_basis_points > MAX_FEE_BASIS_POINTS {
            return Err(TycheCoreError::FeeTooHigh.into());
        }

        let mut data = accounts.protocol_config.try_borrow_mut()?;
        let config   = bytemuck::from_bytes_mut::<ProtocolConfig>(&mut *data);

        // 2: Verify discriminator — reject non-ProtocolConfig accounts.
        if config.discriminator != PROTOCOL_CONFIG {
            return Err(TycheCoreError::InvalidDiscriminator.into());
        }

        // 3: Authority check — only config.authority may update parameters.
        if *accounts.authority.address() != config.authority {
            return Err(TycheCoreError::NotConfigAuthority.into());
        }

        // 4: Apply updates.
        config.treasury            = args.new_treasury;
        config.fee_basis_points    = args.new_fee_basis_points;
        config.max_soft_closes_cap = args.new_max_soft_closes_cap;
        config.min_reserve_price   = args.new_min_reserve_price;
        config.min_duration_secs   = args.new_min_duration_secs;

        Ok(())
    }
}
