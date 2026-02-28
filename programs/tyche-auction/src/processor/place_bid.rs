use pinocchio::{
    Address, AccountView, ProgramResult,
    cpi::{self, Seed, Signer},
    error::ProgramError,
    instruction::{InstructionAccount, InstructionView},
    sysvars::{rent::Rent, Sysvar},
};
use pinocchio_system::instructions::CreateAccount;
use tyche_common::{
    phase::Phase,
    seeds::{AUCTION_SEED, BID_SEED},
};
use tyche_core::{
    discriminator::{COMPETITION_STATE, REGISTER_BID},
    state::competition::CompetitionState,
};
use tyche_escrow::state::vault::EscrowVault;
use crate::{
    args::place_bid::PlaceBidArgs,
    discriminator::{AUCTION_STATE, BID_RECORD},
    error::TycheAuctionError,
    state::{auction::AuctionState, bid_record::BidRecord},
};

// ── Account context 

pub struct PlaceBidAccounts<'a> {
    pub auction_state:   &'a AccountView,
    pub competition:     &'a AccountView,
    pub bid_record:      &'a AccountView,
    pub vault:           &'a AccountView,
    pub bidder:          &'a AccountView,
    pub payer:           &'a AccountView,
    pub tyche_core:      &'a AccountView,
    pub participant_record: &'a AccountView,
    pub system_program:  &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for PlaceBidAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [
            auction_state,
            competition,
            bid_record,
            vault,
            bidder,
            payer,
            tyche_core,
            participant_record,
            system_program,
            ..
        ] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !auction_state.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !bid_record.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !bidder.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !payer.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !participant_record.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(Self {
            auction_state,
            competition,
            bid_record,
            vault,
            bidder,
            payer,
            tyche_core,
            participant_record,
            system_program,
        })
    }
}

// ── Instruction context 

pub struct PlaceBidInstruction<'a> {
    pub accounts: PlaceBidAccounts<'a>,
    pub args:     &'a PlaceBidArgs,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for PlaceBidInstruction<'a> {
    type Error = ProgramError;

    fn try_from(
        (accounts, data): (&'a [AccountView], &'a [u8]),
    ) -> Result<Self, Self::Error> {
        let accounts = PlaceBidAccounts::try_from(accounts)?;
        let args     = PlaceBidArgs::load(data)?;
        Ok(Self { accounts, args })
    }
}

// ── Handler 

impl<'a> PlaceBidInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {
        let accounts = &self.accounts;
        let args     = self.args;

        let competition_pubkey = *accounts.competition.address();
        let bidder_pubkey      = *accounts.bidder.address();

        // 1: Verify competition is Active and get reserve_price.
        let reserve_price = {
            let data  = accounts.competition.try_borrow()?;
            let state = bytemuck::try_from_bytes::<CompetitionState>(&*data)
                .map_err(|_| TycheAuctionError::InvalidCompetition)?;

            if state.discriminator != COMPETITION_STATE {
                return Err(TycheAuctionError::InvalidCompetition.into());
            }
            if state.phase != Phase::Active as u8 {
                return Err(TycheAuctionError::InvalidPhase.into());
            }

            state.reserve_price
        };

        // 2: Bid must meet reserve price.
        if args.amount < reserve_price {
            return Err(TycheAuctionError::BidTooLow.into());
        }

        // 3: Verify AuctionState PDA.
        let competition_bytes = competition_pubkey.as_array();
        let (expected_auction, _) = Address::find_program_address(
            &[AUCTION_SEED, competition_bytes],
            &crate::ID,
        );
        if expected_auction.ne(accounts.auction_state.address()) {
            return Err(TycheAuctionError::InvalidPda.into());
        }

        // 4: Read AuctionState — copy fields before mutable borrow.
        let (auction_discriminator, auction_competition, min_bid_increment, current_high_bid) = {
            let data  = accounts.auction_state.try_borrow()?;
            let state = bytemuck::try_from_bytes::<AuctionState>(&*data)
                .map_err(|_| TycheAuctionError::InvalidDiscriminator)?;

            (state.discriminator, state.competition, state.min_bid_increment, state.current_high_bid)
        };

        if auction_discriminator != AUCTION_STATE {
            return Err(TycheAuctionError::InvalidDiscriminator.into());
        }
        if auction_competition != competition_pubkey {
            return Err(TycheAuctionError::InvalidCompetition.into());
        }

        // 5: Bid increment check — new bid must exceed current high by min_bid_increment. We can remove this later for free auctions, but it prevents spam and is a common auction mechanism.
        if current_high_bid > 0 {
            let required = current_high_bid
                .checked_add(min_bid_increment)
                .ok_or(TycheAuctionError::ArithmeticOverflow)?;
            if args.amount < required {
                return Err(TycheAuctionError::BidTooLow.into());
            }
        }

        // 6: Verify vault — depositor == bidder, competition matches, amount >= bid.
        {
            let data  = accounts.vault.try_borrow()?;
            let vault = bytemuck::try_from_bytes::<EscrowVault>(&*data)
                .map_err(|_| TycheAuctionError::InvalidVault)?;

            if vault.discriminator != tyche_escrow::discriminator::ESCROW_VAULT {
                return Err(TycheAuctionError::InvalidDiscriminator.into());
            }
            if vault.depositor != bidder_pubkey {
                return Err(TycheAuctionError::NotDepositor.into());
            }
            if vault.competition != competition_pubkey {
                return Err(TycheAuctionError::InvalidVault.into());
            }
            if vault.amount < args.amount {
                return Err(TycheAuctionError::InsufficientVault.into());
            }
        }

        // 7: Verify BidRecord PDA.
        let bidder_bytes = bidder_pubkey.as_array();
        let (expected_bid_record, bid_bump) = Address::find_program_address(
            &[BID_SEED, competition_bytes, bidder_bytes],
            &crate::ID,
        );
        if expected_bid_record.ne(accounts.bid_record.address()) {
            return Err(TycheAuctionError::InvalidPda.into());
        }

        // 8: First bid — create BidRecord; subsequent bid — update amount.
        let is_first_bid = accounts.bid_record.is_data_empty();
        if is_first_bid {
            let space    = BidRecord::LEN;
            let lamports = Rent::get()?.try_minimum_balance(space)?;

            CreateAccount {
                from:  accounts.payer,
                to:    accounts.bid_record,
                space: space as u64,
                lamports,
                owner: &crate::ID,
            }
            .invoke_signed(&[Signer::from(&[
                Seed::from(BID_SEED),
                Seed::from(competition_bytes),
                Seed::from(bidder_bytes),
                Seed::from(&[bid_bump]),
            ])])?;

            let mut data = accounts.bid_record.try_borrow_mut()?;
            let record   = bytemuck::from_bytes_mut::<BidRecord>(&mut *data);

            record.discriminator = BID_RECORD;
            record.competition   = competition_pubkey;
            record.bidder        = bidder_pubkey;
            record.amount        = args.amount;
            record.bump          = bid_bump;
        } else {
            let mut data = accounts.bid_record.try_borrow_mut()?;
            let record   = bytemuck::from_bytes_mut::<BidRecord>(&mut *data);

            if record.discriminator != BID_RECORD {
                return Err(TycheAuctionError::InvalidDiscriminator.into());
            }
            if record.bidder != bidder_pubkey {
                return Err(TycheAuctionError::NotDepositor.into());
            }

            record.amount = args.amount;
        }

        // 9: CPI to tyche-core RegisterBid.
        // Accounts: competition(w), participant_record(w), bidder(s), payer(s,w), system_program
        let ix = InstructionView {
            program_id: accounts.tyche_core.address(),
            accounts:   &[
                InstructionAccount::writable(accounts.competition.address()),
                InstructionAccount::writable(accounts.participant_record.address()),
                InstructionAccount::readonly_signer(accounts.bidder.address()),
                InstructionAccount::writable_signer(accounts.payer.address()),
                InstructionAccount::readonly(accounts.system_program.address()),
            ],
            data: &REGISTER_BID,
        };

        cpi::invoke::<5>(&ix, &[
            accounts.competition,
            accounts.participant_record,
            accounts.bidder,
            accounts.payer,
            accounts.system_program,
        ])?;

        // 10: Update AuctionState — new high bid or just increment count.
        {
            let mut data  = accounts.auction_state.try_borrow_mut()?;
            let auction   = bytemuck::from_bytes_mut::<AuctionState>(&mut *data);

            if args.amount > auction.current_high_bid {
                auction.current_high_bid = args.amount;
                auction.current_winner   = bidder_pubkey;
            }
            auction.bid_count = auction.bid_count
                .checked_add(1)
                .ok_or(TycheAuctionError::ArithmeticOverflow)?;
        }

        Ok(())
    }
}
