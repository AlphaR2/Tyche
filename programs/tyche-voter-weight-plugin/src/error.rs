use pinocchio::error::ProgramError;

/// Errors returned by `tyche-voter-weight-plugin` instructions.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginError {
    /// The vault's `competition` field does not match the registrar's configured competition.
    WrongCompetition    = 0x3100,

    /// The vault's `depositor` field does not match the signing voter.
    WrongDepositor      = 0x3101,

    /// The registrar account has an unexpected discriminator or cannot be cast as `Registrar`.
    InvalidRegistrar    = 0x3102,

    /// The vault account data is too short to contain a valid `EscrowVault`.
    InvalidVaultData    = 0x3103,

    /// The account discriminator does not match the expected value.
    InvalidDiscriminator = 0x3104,
}

impl From<PluginError> for ProgramError {
    #[inline(always)]
    fn from(e: PluginError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
