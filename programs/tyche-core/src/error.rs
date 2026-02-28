use pinocchio::error::ProgramError;

/// Errors returned by `tyche-core` instructions.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TycheCoreError {
    InvalidPhase            = 0x1400,
    NotAuthority            = 0x1401,
    InvalidCrankAuthority   = 0x140A,
    AuctionNotStarted       = 0x1402,
    AuctionNotExpired       = 0x1403,
    SoftCloseCapReached     = 0x1404,
    SoftCloseNotArmed       = 0x1405,
    NotUndelegated          = 0x1406,
    HasParticipants         = 0x1407,
    ArithmeticOverflow      = 0x1408,
    InvalidDiscriminator    = 0x1409,
    InvalidAccountData      = 0x1410,
    AuctionEnded            = 0x1411,
    UnauthorizedCaller      = 0x1412,
    ParticipantCapReached   = 0x1413,
    ConfigAlreadyInitialized = 0x1414,
    FeeTooHigh               = 0x1415,
    NotConfigAuthority       = 0x1416,
    ProtocolPaused           = 0x1417,
}

impl From<TycheCoreError> for ProgramError {
    #[inline(always)]
    fn from(e: TycheCoreError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
