use pinocchio::error::ProgramError;

/// Errors returned by `tyche-core` instructions.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TycheCoreError {
    /// The competition is not in the phase required by this instruction.
    InvalidPhase            = 0x1400,

    /// The signer is not the authority that created this competition.
    NotAuthority            = 0x1401,

    /// The signer is a valid signer but is not the authorized crank account.
    /// Only the protocol crank may call `ExtendCompetition` and `CloseCompetition`.
    InvalidCrankAuthority   = 0x140A,

    /// `ActivateCompetition` was called before `start_time`.
    AuctionNotStarted       = 0x1402,

    /// `SettleCompetition` was called before `end_time` has elapsed
    /// and no early-settlement condition has been met.
    AuctionNotExpired       = 0x1403,

    /// `ExtendCompetition` was called but `soft_close_count` has
    /// already reached `max_soft_closes`.
    SoftCloseCapReached     = 0x1404,

    /// `ExtendCompetition` was called but the last bid did not land
    /// inside the soft-close window.
    SoftCloseNotArmed       = 0x1405,

    /// An instruction that requires the account to be undelegated
    /// was called while it is still delegated to the MagicBlock PER.
    NotUndelegated          = 0x1406,

    /// `CancelCompetition` was called but at least one participant
    /// has already placed a bid.
    HasParticipants         = 0x1407,

    /// An arithmetic operation overflowed or underflowed.
    ArithmeticOverflow      = 0x1408,

    /// The account discriminator does not match the expected value.
    InvalidDiscriminator    = 0x1409,

    InvalidAccountData      = 0x1410,

    /// `ExtendCompetition` was called after `end_time` has already elapsed.
    /// The window is closed — call `CloseCompetition` to transition the phase.
    AuctionEnded            = 0x1411,

    /// `RegisterBid` was called via CPI but the `caller_program` account did
    /// not sign. CPI signer propagation must be used — unsigned callers are rejected.
    UnauthorizedCaller      = 0x1412,

    /// `RegisterBid` was called for a first bid but `participant_count` has
    /// already reached `MAX_PARTICIPANTS`. No new bidders are accepted.
    ParticipantCapReached   = 0x1413,
}

impl From<TycheCoreError> for ProgramError {
    #[inline(always)]
    fn from(e: TycheCoreError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
