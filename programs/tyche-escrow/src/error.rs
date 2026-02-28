use pinocchio::error::ProgramError;

/// Errors returned by `tyche-escrow` instructions.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TycheEscrowError {
    /// The competition or vault is not in the phase required by this instruction.
    InvalidPhase            = 0x2400,

    /// The signer is a valid signer but is not the authorized crank account.
    /// Only the protocol crank may call `Release`.
    InvalidCrankAuthority   = 0x2401,

    /// The account discriminator does not match the expected value.
    InvalidDiscriminator    = 0x2402,

    /// An arithmetic operation overflowed or underflowed.
    ArithmeticOverflow      = 0x2403,

    /// `Release` was called but the depositor is not the competition winner.
    /// Winners are identified by `ParticipantRecord::is_winner == IS_WINNER`.
    NotWinner               = 0x2404,

    /// `Refund` was called but the depositor won the competition.
    /// Winners must use `Release`, not `Refund`.
    WinnerCannotRefund      = 0x2405,

    /// `Deposit` was called with a zero amount.
    InvalidAmount           = 0x2406,

    /// The `treasury` account passed to `Release` does not match
    /// `ProtocolConfig::treasury`.
    InvalidTreasury         = 0x2407,

    /// The `protocol_config` account passed to `Release` has an unexpected
    /// discriminator or cannot be deserialized as `ProtocolConfig`.
    InvalidProtocolConfig   = 0x2408,
}

impl From<TycheEscrowError> for ProgramError {
    #[inline(always)]
    fn from(e: TycheEscrowError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
