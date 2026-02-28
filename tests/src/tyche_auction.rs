//! Unit tests for `tyche-auction` instructions.

use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use tyche_common::{phase::Phase, seeds::AUCTION_SEED};
use tyche_auction::{
    args::place_bid::PlaceBidArgs,
    instruction_builder::place_bid::place_bid as build_place_bid,
    state::auction::AuctionState,
};
use tyche_common::seeds::BID_SEED;
use tyche_core::instruction_builder::register_bid::derive_participant_record_pda;
use crate::helpers::*;

// ── Test context ──────────────────────────────────────────────────────────────

struct Ctx {
    svm:           litesvm::LiteSVM,
    seller:        Keypair,
    crank:         Keypair,
    treasury:      Keypair,
    bidder1:       Keypair,
    bidder2:       Keypair,
    competition:   Pubkey,
    auction_state: Pubkey,
    dummy_mint:    Pubkey,
}

fn setup() -> Ctx {
    let mut svm  = setup_svm();
    let seller   = Keypair::new();
    let crank    = Keypair::new();
    let treasury = Keypair::new();
    let bidder1  = Keypair::new();
    let bidder2  = Keypair::new();

    for kp in [&seller, &bidder1, &bidder2] {
        svm.airdrop(&kp.pubkey(), 10_000_000_000).unwrap();
    }

    initialize_protocol_config(&mut svm, &seller, &treasury.pubkey(), &crank.pubkey());

    let mut id = [0u8; 32]; id[0] = 40;
    let competition = create_competition(&mut svm, &seller, id, 1_000_000);
    let dummy_mint  = Keypair::new().pubkey();
    let auction_state = create_auction(&mut svm, &seller, &competition, &dummy_mint, 1_000_000);

    Ctx { svm, seller, crank, treasury, bidder1, bidder2, competition, auction_state, dummy_mint }
}

// ── CreateAuction ─────────────────────────────────────────────────────────────

#[test]
fn create_auction_success() {
    let ctx = setup();
    let account = ctx.svm.get_account(&ctx.auction_state).expect("auction must exist");
    assert_eq!(account.owner(), auction_id());

    let state = bytemuck::from_bytes::<AuctionState>(account.data());
    assert_eq!(
        state.competition,
        pinocchio::Address::new_from_array(ctx.competition.to_bytes()),
    );
    assert_eq!(state.min_bid_increment, 1_000_000);
    assert_eq!(state.bid_count, 0);
    assert_eq!(state.current_high_bid, 0);
}

#[test]
fn create_auction_wrong_authority_fails() {
    let mut ctx = setup();
    let impostor = Keypair::new();
    ctx.svm.airdrop(&impostor.pubkey(), 1_000_000_000).unwrap();

    let mut id = [0u8; 32]; id[0] = 41;
    let competition2 = create_competition(&mut ctx.svm, &ctx.seller, id, 1_000_000);

    let (pda, _) = Pubkey::find_program_address(
        &[AUCTION_SEED, &competition2.to_bytes()],
        &auction_id(),
    );
    let args = tyche_auction::args::create_auction::CreateAuctionArgs {
        asset_mint:        pinocchio::Address::new_from_array(Keypair::new().pubkey().to_bytes()),
        min_bid_increment: 1_000_000,
    };
    let ix = tyche_auction::instruction_builder::create_auction::create_auction(
        &pda,
        &competition2,
        &impostor.pubkey(), // not the competition authority
        &impostor.pubkey(),
        args,
    );
    expect_failure(&mut ctx.svm, ix, &[&impostor]);
}

// ── PlaceBid ──────────────────────────────────────────────────────────────────

#[test]
fn place_bid_success() {
    let mut ctx = setup();
    force_competition_phase(&mut ctx.svm, &ctx.competition, Phase::Active, i64::MAX - 1);

    let vault = deposit(&mut ctx.svm, &ctx.bidder1, &ctx.competition, 10_000_000);
    place_bid(&mut ctx.svm, &ctx.bidder1, &ctx.competition, &ctx.auction_state, &vault, 5_000_000);

    let account = ctx.svm.get_account(&ctx.auction_state).unwrap();
    let state = bytemuck::from_bytes::<AuctionState>(account.data());
    assert_eq!(state.current_high_bid, 5_000_000, "current_high_bid must update");
    assert_eq!(state.bid_count, 1, "bid_count must increment");
    assert_eq!(
        state.current_winner,
        pinocchio::Address::new_from_array(ctx.bidder1.pubkey().to_bytes()),
    );
}

#[test]
fn place_bid_below_reserve_fails() {
    let mut ctx = setup();
    force_competition_phase(&mut ctx.svm, &ctx.competition, Phase::Active, i64::MAX - 1);
    let vault = deposit(&mut ctx.svm, &ctx.bidder1, &ctx.competition, 10_000_000);

    let competition_bytes = ctx.competition.to_bytes();
    let bidder_bytes      = ctx.bidder1.pubkey().to_bytes();
    let (bid_record, _) = Pubkey::find_program_address(
        &[BID_SEED, &competition_bytes, &bidder_bytes],
        &auction_id(),
    );
    let (participant_record, _) =
        derive_participant_record_pda(&ctx.competition, &ctx.bidder1.pubkey());

    // reserve_price = 1_000_000; bid 500_000 → BidTooLow.
    let ix = build_place_bid(
        &ctx.auction_state,
        &ctx.competition,
        &bid_record,
        &vault,
        &ctx.bidder1.pubkey(),
        &ctx.bidder1.pubkey(),
        &participant_record,
        PlaceBidArgs { amount: 500_000 },
    );
    expect_failure(&mut ctx.svm, ix, &[&ctx.bidder1]);
}

#[test]
fn place_bid_below_min_increment_fails() {
    let mut ctx = setup();
    force_competition_phase(&mut ctx.svm, &ctx.competition, Phase::Active, i64::MAX - 1);

    // Bidder1 places a valid first bid.
    let vault1 = deposit(&mut ctx.svm, &ctx.bidder1, &ctx.competition, 50_000_000);
    place_bid(&mut ctx.svm, &ctx.bidder1, &ctx.competition, &ctx.auction_state, &vault1, 10_000_000);

    // Bidder2 tries to outbid by less than min_bid_increment (1_000_000).
    // current_high_bid = 10_000_000; must exceed 11_000_000 to succeed.
    let vault2 = deposit(&mut ctx.svm, &ctx.bidder2, &ctx.competition, 50_000_000);

    let competition_bytes = ctx.competition.to_bytes();
    let bidder_bytes      = ctx.bidder2.pubkey().to_bytes();
    let (bid_record, _) = Pubkey::find_program_address(
        &[BID_SEED, &competition_bytes, &bidder_bytes],
        &auction_id(),
    );
    let (participant_record, _) =
        derive_participant_record_pda(&ctx.competition, &ctx.bidder2.pubkey());

    // 10_500_000 < 10_000_000 + 1_000_000 → BidTooLow.
    let ix = build_place_bid(
        &ctx.auction_state,
        &ctx.competition,
        &bid_record,
        &vault2,
        &ctx.bidder2.pubkey(),
        &ctx.bidder2.pubkey(),
        &participant_record,
        PlaceBidArgs { amount: 10_500_000 },
    );
    expect_failure(&mut ctx.svm, ix, &[&ctx.bidder2]);
}

#[test]
fn place_bid_insufficient_vault_fails() {
    let mut ctx = setup();
    force_competition_phase(&mut ctx.svm, &ctx.competition, Phase::Active, i64::MAX - 1);

    // Deposit 5M but try to bid 10M → InsufficientVault.
    let vault = deposit(&mut ctx.svm, &ctx.bidder1, &ctx.competition, 5_000_000);

    let competition_bytes = ctx.competition.to_bytes();
    let bidder_bytes      = ctx.bidder1.pubkey().to_bytes();
    let (bid_record, _) = Pubkey::find_program_address(
        &[BID_SEED, &competition_bytes, &bidder_bytes],
        &auction_id(),
    );
    let (participant_record, _) =
        derive_participant_record_pda(&ctx.competition, &ctx.bidder1.pubkey());

    let ix = build_place_bid(
        &ctx.auction_state,
        &ctx.competition,
        &bid_record,
        &vault,
        &ctx.bidder1.pubkey(),
        &ctx.bidder1.pubkey(),
        &participant_record,
        PlaceBidArgs { amount: 10_000_000 },
    );
    expect_failure(&mut ctx.svm, ix, &[&ctx.bidder1]);
}

#[test]
fn place_bid_on_non_active_competition_fails() {
    let mut ctx = setup();
    // Competition is still Scheduled — deposit and bid must both fail.
    // (We test the bid gate specifically.)
    // Force Active for deposit only, then force back to Scheduled for bid.
    force_competition_phase(&mut ctx.svm, &ctx.competition, Phase::Active, i64::MAX - 1);
    let vault = deposit(&mut ctx.svm, &ctx.bidder1, &ctx.competition, 10_000_000);
    force_competition_phase(&mut ctx.svm, &ctx.competition, Phase::Scheduled, 0);

    let competition_bytes = ctx.competition.to_bytes();
    let bidder_bytes      = ctx.bidder1.pubkey().to_bytes();
    let (bid_record, _) = Pubkey::find_program_address(
        &[BID_SEED, &competition_bytes, &bidder_bytes],
        &auction_id(),
    );
    let (participant_record, _) =
        derive_participant_record_pda(&ctx.competition, &ctx.bidder1.pubkey());

    let ix = build_place_bid(
        &ctx.auction_state,
        &ctx.competition,
        &bid_record,
        &vault,
        &ctx.bidder1.pubkey(),
        &ctx.bidder1.pubkey(),
        &participant_record,
        PlaceBidArgs { amount: 5_000_000 },
    );
    expect_failure(&mut ctx.svm, ix, &[&ctx.bidder1]);
}

#[test]
fn winner_updates_on_higher_bid() {
    let mut ctx = setup();
    force_competition_phase(&mut ctx.svm, &ctx.competition, Phase::Active, i64::MAX - 1);

    // Bidder1 bids 10M.
    let vault1 = deposit(&mut ctx.svm, &ctx.bidder1, &ctx.competition, 50_000_000);
    place_bid(&mut ctx.svm, &ctx.bidder1, &ctx.competition, &ctx.auction_state, &vault1, 10_000_000);

    // Bidder2 bids 11M (10M + 1M min increment) — becomes new winner.
    let vault2 = deposit(&mut ctx.svm, &ctx.bidder2, &ctx.competition, 50_000_000);
    place_bid(&mut ctx.svm, &ctx.bidder2, &ctx.competition, &ctx.auction_state, &vault2, 11_000_000);

    let account = ctx.svm.get_account(&ctx.auction_state).unwrap();
    let state = bytemuck::from_bytes::<AuctionState>(account.data());
    assert_eq!(state.current_high_bid, 11_000_000);
    assert_eq!(state.bid_count, 2);
    assert_eq!(
        state.current_winner,
        pinocchio::Address::new_from_array(ctx.bidder2.pubkey().to_bytes()),
        "winner must switch to higher bidder",
    );
}

#[test]
fn bid_count_increments_on_every_bid() {
    let mut ctx = setup();
    force_competition_phase(&mut ctx.svm, &ctx.competition, Phase::Active, i64::MAX - 1);

    // Same bidder places three increasing bids.
    let vault = deposit(&mut ctx.svm, &ctx.bidder1, &ctx.competition, 100_000_000);
    place_bid(&mut ctx.svm, &ctx.bidder1, &ctx.competition, &ctx.auction_state, &vault, 5_000_000);
    place_bid(&mut ctx.svm, &ctx.bidder1, &ctx.competition, &ctx.auction_state, &vault, 10_000_000);
    place_bid(&mut ctx.svm, &ctx.bidder1, &ctx.competition, &ctx.auction_state, &vault, 15_000_000);

    let account = ctx.svm.get_account(&ctx.auction_state).unwrap();
    let state = bytemuck::from_bytes::<AuctionState>(account.data());
    assert_eq!(state.bid_count, 3, "bid_count must increment on each call including repeats");
}
