use pinocchio::{
    AccountView, ProgramResult,
    error::ProgramError,
};
use crate::{
    discriminator::AUCTION_STATE,
    error::TycheAuctionError,
    state::auction::AuctionState,
};

// ── Account context 

/// Validated account context for `ProcessUndelegation`.
///
/// MagicBlock calls this with `EXTERNAL_UNDELEGATE_DISCRIMINATOR` when the
/// `AuctionState` account is being undelegated back to mainnet. The program
/// merges the committed buffer data into the live account.
pub struct ProcessUndelegationAccounts<'a> {
    pub auction_state: &'a AccountView,
    pub buffer:        &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for ProcessUndelegationAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [auction_state, buffer, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !auction_state.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(Self { auction_state, buffer })
    }
}

// ── Instruction context 

pub struct ProcessUndelegationInstruction<'a> {
    pub accounts: ProcessUndelegationAccounts<'a>,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for ProcessUndelegationInstruction<'a> {
    type Error = ProgramError;

    fn try_from(
        (accounts, _data): (&'a [AccountView], &'a [u8]),
    ) -> Result<Self, Self::Error> {
        let accounts = ProcessUndelegationAccounts::try_from(accounts)?;
        Ok(Self { accounts })
    }
}

// ── Handler 

impl<'a> ProcessUndelegationInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;

        // 1: Verify the buffer contains a valid AuctionState before merging.
        {
            let data  = accounts.buffer.try_borrow()?;
            let state = bytemuck::try_from_bytes::<AuctionState>(&*data)
                .map_err(|_| TycheAuctionError::InvalidDiscriminator)?;

            if state.discriminator != AUCTION_STATE {
                return Err(TycheAuctionError::InvalidDiscriminator.into());
            }
        }

        // 2: Copy committed buffer data into the live AuctionState account.
        {
            let src  = accounts.buffer.try_borrow()?;
            let mut dst = accounts.auction_state.try_borrow_mut()?;

            if src.len() != AuctionState::LEN || dst.len() != AuctionState::LEN {
                return Err(TycheAuctionError::InvalidDiscriminator.into());
            }

            dst.copy_from_slice(&src);
        }

        Ok(())
    }
}
