use pinocchio::error::ProgramError;

/// Errors returned by `tyche-auction` instructions.
///
/// Error codes start at 0x3000 to avoid collisions with `tyche-core` (0x1000+)
/// and `tyche-escrow` (0x2000+).
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TycheAuctionError {
    InvalidDiscriminator = 0x3000,
    InvalidPhase         = 0x3001,
    NotAuthority         = 0x3002,
    AuctionAlreadyExists = 0x3003,
    BidTooLow            = 0x3004,
    InsufficientVault    = 0x3005,
    NoWinner             = 0x3006,
    NotCrank             = 0x3007,
    NotDepositor         = 0x3008,
    NotEscrowProgram     = 0x3009,
    ArithmeticOverflow   = 0x300a,
    InvalidCompetition   = 0x300b,
    InvalidVault         = 0x300c,
    InvalidPda           = 0x300d,
}

impl From<TycheAuctionError> for ProgramError {
    #[inline(always)]
    fn from(e: TycheAuctionError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
