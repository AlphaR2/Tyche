//! Integration tests: full end-to-end flows spanning all three Tyche programs.
//!
//! These tests cover cross-program coordination paths that are not exercised by
//! the per-program unit test modules:
//!
//! - Competition cancellation with vault refunds (`cancel_and_refund_*`)
//! - `CancelAuction` instruction — closes `AuctionState`, returns rent to seller
//! - No-winner finalization — competition settles with `current_winner == zero`
//!
//! Phase transitions that require MagicBlock delegation (Activate, Close) are
//! bypassed with `force_competition_phase`, the same technique used in the unit
//! test modules.

use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use tyche_common::phase::Phase;
use tyche_core::state::competition::CompetitionState;
use crate::helpers::*;

// ── Shared setup 

struct Ctx {
    svm:         litesvm::LiteSVM,
    authority:   Keypair,
    crank:       Keypair,
    treasury:    Keypair,
    bidder1:     Keypair,
    bidder2:     Keypair,
    competition: Pubkey,
    dummy_mint:  Pubkey,
}

fn make_ctx(id_byte: u8) -> Ctx {
    let mut svm   = setup_svm();
    let authority = Keypair::new();
    let crank     = Keypair::new();
    let treasury  = Keypair::new();
    let bidder1   = Keypair::new();
    let bidder2   = Keypair::new();

    for kp in [&authority, &bidder1, &bidder2] {
        svm.airdrop(&kp.pubkey(), 10_000_000_000).unwrap();
    }

    initialize_protocol_config(&mut svm, &authority, &treasury.pubkey(), &crank.pubkey());

    let mut id = [0u8; 32];
    id[0] = id_byte;
    let competition = create_competition(&mut svm, &authority, id, 1_000_000);
    let dummy_mint  = Keypair::new().pubkey();

    Ctx { svm, authority, crank, treasury, bidder1, bidder2, competition, dummy_mint }
}

// ── Cancel + Refund ───────────────────────────────────────────────────────────

/// Depositors reclaim funds after a competition is cancelled.
///
/// Flow: create → (force) Active → deposit x 2 → (force) Cancelled → refund x 2
#[test]
fn cancel_and_refund_deposits() {
    let mut ctx = make_ctx(50);

    force_competition_phase(&mut ctx.svm, &ctx.competition, Phase::Active, i64::MAX - 1);

    let vault1 = deposit(&mut ctx.svm, &ctx.bidder1, &ctx.competition, 10_000_000);
    let vault2 = deposit(&mut ctx.svm, &ctx.bidder2, &ctx.competition, 20_000_000);

    let before1 = lamports(&ctx.svm, &ctx.bidder1.pubkey());
    let before2 = lamports(&ctx.svm, &ctx.bidder2.pubkey());

    // Cancel competition without delegation CPI.
    force_competition_phase(&mut ctx.svm, &ctx.competition, Phase::Cancelled, 0);

    refund_vault(&mut ctx.svm, &ctx.bidder1, &ctx.competition);
    refund_vault(&mut ctx.svm, &ctx.bidder2, &ctx.competition);

    // Vaults must be closed.
    assert_eq!(lamports(&ctx.svm, &vault1), 0, "vault1 must be drained");
    assert_eq!(lamports(&ctx.svm, &vault2), 0, "vault2 must be drained");

    // Both depositors recover at least their original deposit.
    let after1 = lamports(&ctx.svm, &ctx.bidder1.pubkey());
    let after2 = lamports(&ctx.svm, &ctx.bidder2.pubkey());

    assert!(
        after1 >= before1 + 10_000_000,
        "bidder1 must receive at least the deposited amount back",
    );
    assert!(
        after2 >= before2 + 20_000_000,
        "bidder2 must receive at least the deposited amount back",
    );
}

/// A second refund on an already-drained vault fails (discriminator check).
#[test]
fn double_refund_fails() {
    let mut ctx = make_ctx(51);

    force_competition_phase(&mut ctx.svm, &ctx.competition, Phase::Active, i64::MAX - 1);
    deposit(&mut ctx.svm, &ctx.bidder1, &ctx.competition, 10_000_000);
    force_competition_phase(&mut ctx.svm, &ctx.competition, Phase::Cancelled, 0);

    refund_vault(&mut ctx.svm, &ctx.bidder1, &ctx.competition);

    // Vault data is now zeroed — the second refund must be rejected.
    let (ix, _) = tyche_escrow::instruction_builder::refund::build_refund(
        &ctx.bidder1.pubkey(),
        &ctx.competition,
    );
    expect_failure(&mut ctx.svm, ix, &[&ctx.bidder1]);
}

/// Refund must fail on an Active competition (neither Cancelled nor Settled).
#[test]
fn refund_on_active_competition_fails() {
    let mut ctx = make_ctx(52);

    force_competition_phase(&mut ctx.svm, &ctx.competition, Phase::Active, i64::MAX - 1);
    deposit(&mut ctx.svm, &ctx.bidder1, &ctx.competition, 5_000_000);

    let (ix, _) = tyche_escrow::instruction_builder::refund::build_refund(
        &ctx.bidder1.pubkey(),
        &ctx.competition,
    );
    expect_failure(&mut ctx.svm, ix, &[&ctx.bidder1]);
}

// ── CancelAuction ─────────────────────────────────────────────────────────────

/// `CancelAuction` closes `AuctionState` and returns rent to the seller once
/// the underlying competition is cancelled.
#[test]
fn cancel_auction_after_cancel_competition() {
    let mut ctx = make_ctx(53);

    let auction_state = create_auction(
        &mut ctx.svm,
        &ctx.authority,
        &ctx.competition,
        &ctx.dummy_mint,
        1_000_000,
    );

    assert!(
        ctx.svm.get_account(&auction_state).is_some(),
        "auction_state must exist before cancel",
    );

    let seller_before = lamports(&ctx.svm, &ctx.authority.pubkey());

    force_competition_phase(&mut ctx.svm, &ctx.competition, Phase::Cancelled, 0);

    cancel_auction(&mut ctx.svm, &ctx.authority, &auction_state, &ctx.competition);

    // AuctionState must be closed (zero lamports).
    assert_eq!(
        lamports(&ctx.svm, &auction_state),
        0,
        "auction_state lamports must be zero after CancelAuction",
    );
    // Seller must have received the rent back.
    let seller_after = lamports(&ctx.svm, &ctx.authority.pubkey());
    assert!(seller_after > seller_before, "seller must receive auction_state rent back");
}

/// `CancelAuction` is rejected when the competition is still Active.
#[test]
fn cancel_auction_on_active_competition_fails() {
    let mut ctx = make_ctx(54);

    let auction_state = create_auction(
        &mut ctx.svm,
        &ctx.authority,
        &ctx.competition,
        &ctx.dummy_mint,
        1_000_000,
    );

    force_competition_phase(&mut ctx.svm, &ctx.competition, Phase::Active, i64::MAX - 1);

    let ix = tyche_auction::instruction_builder::cancel_auction::cancel_auction(
        &auction_state,
        &ctx.competition,
        &ctx.authority.pubkey(),
        &ctx.authority.pubkey(),
    );
    expect_failure(&mut ctx.svm, ix, &[&ctx.authority]);
}

// ── No-winner finalization ────────────────────────────────────────────────────

/// When no bids are placed, the auction finalizes with `current_winner == zero`.
/// `FinalizeAuction` CPIs to `SettleCompetition`, which transitions the competition
/// to `Settled` without writing `IS_WINNER` on any `ParticipantRecord`.
///
/// Flow: create → create auction → (force) Settling → finalize (winner = zero)
#[test]
fn no_winner_finalization_settles_competition() {
    let mut ctx = make_ctx(55);

    let auction_state = create_auction(
        &mut ctx.svm,
        &ctx.authority,
        &ctx.competition,
        &ctx.dummy_mint,
        1_000_000,
    );

    // Jump straight to Settling — no bids means AuctionState.current_winner is zero.
    force_competition_phase(&mut ctx.svm, &ctx.competition, Phase::Settling, 0);

    // Pubkey::default() == all-zeros == Address::default(); the handler skips PDA
    // verification when auction_state.current_winner == zero.
    let delegation_record = Keypair::new().pubkey();
    let zero_winner       = Pubkey::default();

    finalize_auction(
        &mut ctx.svm,
        &ctx.crank,
        &auction_state,
        &ctx.competition,
        &zero_winner,
        &delegation_record,
    );

    // Competition must be Settled.
    let account = ctx.svm
        .get_account(&ctx.competition)
        .expect("competition must still exist after no-winner finalization");
    let state = bytemuck::from_bytes::<CompetitionState>(account.data());
    assert_eq!(
        state.phase,
        Phase::Settled as u8,
        "competition must be Settled after no-winner finalization",
    );
}

/// A depositor who never placed a bid has no `ParticipantRecord`. The Settled
/// refund path requires a valid record, so they must use the Cancelled path.
/// This test documents that the Cancelled-path refund works without a record.
#[test]
fn no_bid_depositor_refunds_on_cancelled_path() {
    let mut ctx = make_ctx(56);

    force_competition_phase(&mut ctx.svm, &ctx.competition, Phase::Active, i64::MAX - 1);
    let vault  = deposit(&mut ctx.svm, &ctx.bidder1, &ctx.competition, 8_000_000);
    let before = lamports(&ctx.svm, &ctx.bidder1.pubkey());

    // Force Cancelled — skips the participant record check in the Refund handler.
    force_competition_phase(&mut ctx.svm, &ctx.competition, Phase::Cancelled, 0);

    refund_vault(&mut ctx.svm, &ctx.bidder1, &ctx.competition);

    assert_eq!(lamports(&ctx.svm, &vault), 0, "vault must be drained");
    let after = lamports(&ctx.svm, &ctx.bidder1.pubkey());
    assert!(
        after >= before + 8_000_000,
        "depositor must receive at least their deposit back",
    );
}
