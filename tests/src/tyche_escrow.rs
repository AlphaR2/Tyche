//! Unit tests for `tyche-escrow` instructions.
//!
//! The Deposit, Refund, and Release processors all require the competition to
//! be in a specific phase.  Phase transitions that require MagicBlock delegation
//! are bypassed via `force_competition_phase`.

use solana_sdk::signature::{Keypair, Signer};
use tyche_common::phase::Phase;
use tyche_escrow::state::vault::EscrowVault;
use crate::helpers::*;

// ── Test context ──────────────────────────────────────────────────────────────

struct Ctx {
    svm:         litesvm::LiteSVM,
    authority:   Keypair, // seller / competition creator
    crank:       Keypair,
    treasury:    Keypair,
    bidder1:     Keypair,
    bidder2:     Keypair,
    competition: solana_sdk::pubkey::Pubkey,
}

fn setup() -> Ctx {
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

    let mut id = [0u8; 32]; id[0] = 20;
    let competition = create_competition(&mut svm, &authority, id, 1_000_000);

    // Bypass delegation: set competition to Active so deposits are accepted.
    force_competition_phase(&mut svm, &competition, Phase::Active, i64::MAX - 1);

    Ctx { svm, authority, crank, treasury, bidder1, bidder2, competition }
}

// ── Deposit ───────────────────────────────────────────────────────────────────

#[test]
fn deposit_creates_vault() {
    let mut ctx = setup();
    let vault = deposit(&mut ctx.svm, &ctx.bidder1, &ctx.competition, 5_000_000);

    let account = ctx.svm.get_account(&vault).expect("vault must exist");
    assert_eq!(account.owner(), escrow_id());

    let state = bytemuck::from_bytes::<EscrowVault>(account.data());
    assert_eq!(state.amount, 5_000_000, "vault.amount must equal deposited amount");
    assert_eq!(
        state.depositor,
        pinocchio::Address::new_from_array(ctx.bidder1.pubkey().to_bytes()),
    );
    assert_eq!(
        state.competition,
        pinocchio::Address::new_from_array(ctx.competition.to_bytes()),
    );
}

#[test]
fn deposit_tops_up_existing_vault() {
    let mut ctx = setup();
    deposit(&mut ctx.svm, &ctx.bidder1, &ctx.competition, 5_000_000);
    let vault = deposit(&mut ctx.svm, &ctx.bidder1, &ctx.competition, 3_000_000);

    let account = ctx.svm.get_account(&vault).expect("vault must exist");
    let state = bytemuck::from_bytes::<EscrowVault>(account.data());
    assert_eq!(state.amount, 8_000_000, "vault.amount must accumulate across deposits");
}

#[test]
fn deposit_zero_amount_fails() {
    let mut ctx = setup();
    let (ix, _) = tyche_escrow::instruction_builder::deposit::build_deposit(
        &ctx.bidder1.pubkey(),
        &ctx.bidder1.pubkey(),
        &ctx.competition,
        0, // zero rejected
    );
    expect_failure(&mut ctx.svm, ix, &[&ctx.bidder1]);
}

#[test]
fn deposit_on_non_active_competition_fails() {
    let mut svm   = setup_svm();
    let authority = Keypair::new();
    let crank     = Keypair::new();
    let treasury  = Keypair::new();
    let bidder    = Keypair::new();

    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&bidder.pubkey(),    10_000_000_000).unwrap();

    initialize_protocol_config(&mut svm, &authority, &treasury.pubkey(), &crank.pubkey());

    let mut id = [0u8; 32]; id[0] = 21;
    let competition = create_competition(&mut svm, &authority, id, 1_000_000);
    // Competition is still Scheduled — deposit must fail.

    let (ix, _) = tyche_escrow::instruction_builder::deposit::build_deposit(
        &bidder.pubkey(),
        &bidder.pubkey(),
        &competition,
        1_000_000,
    );
    expect_failure(&mut svm, ix, &[&bidder]);
}

// ── Refund (Cancelled path) ───────────────────────────────────────────────────

#[test]
fn refund_on_cancellation() {
    let mut ctx = setup();

    // Bidder1 deposits while Active.
    let vault = deposit(&mut ctx.svm, &ctx.bidder1, &ctx.competition, 10_000_000);
    let before = lamports(&ctx.svm, &ctx.bidder1.pubkey());

    // Move competition to Cancelled.
    force_competition_phase(&mut ctx.svm, &ctx.competition, Phase::Cancelled, 0);

    refund_vault(&mut ctx.svm, &ctx.bidder1, &ctx.competition);

    // Vault must be drained.
    assert_eq!(lamports(&ctx.svm, &vault), 0, "vault must be drained after refund");

    // Bidder1 must have received at least the deposit amount back.
    let after = lamports(&ctx.svm, &ctx.bidder1.pubkey());
    assert!(after > before, "bidder must receive lamports back");
    assert!(after >= before + 10_000_000, "bidder must receive full deposit back");
}

#[test]
fn refund_on_active_competition_fails() {
    let mut ctx = setup();
    deposit(&mut ctx.svm, &ctx.bidder1, &ctx.competition, 10_000_000);
    // Competition is still Active — refund must fail.
    let (ix, _) = tyche_escrow::instruction_builder::refund::build_refund(
        &ctx.bidder1.pubkey(),
        &ctx.competition,
    );
    expect_failure(&mut ctx.svm, ix, &[&ctx.bidder1]);
}

// ── Release ───────────────────────────────────────────────────────────────────

/// Helper that sets up a fully settled competition with one winner and one loser,
/// returning (winner_participant_record, winner_vault, loser_vault).
fn setup_settled(
    svm:       &mut litesvm::LiteSVM,
    authority: &Keypair,
    crank:     &Keypair,
    treasury:  &Keypair,
    winner:    &Keypair,
    loser:     &Keypair,
    competition: &solana_sdk::pubkey::Pubkey,
) -> (solana_sdk::pubkey::Pubkey, solana_sdk::pubkey::Pubkey, solana_sdk::pubkey::Pubkey) {
    // Auction must exist before placing bids.
    let dummy_mint = Keypair::new().pubkey();
    let auction_state = create_auction(svm, authority, competition, &dummy_mint, 1_000_000);

    // Deposits (Active phase already set by caller).
    let winner_vault = deposit(svm, winner, competition, 51_000_000);
    let loser_vault  = deposit(svm, loser,  competition, 50_000_000);

    // Bids — winner outbids loser.
    let (_, winner_participant) =
        place_bid(svm, winner, competition, &auction_state, &winner_vault, 51_000_000);
    place_bid(svm, loser, competition, &auction_state, &loser_vault, 50_000_000);

    // Transition to Settling then finalize (CPIs to SettleCompetition).
    force_competition_phase(svm, competition, Phase::Settling, 0);
    let delegation_record = Keypair::new().pubkey(); // zero lamports → undelegation proof
    finalize_auction(svm, crank, &auction_state, competition, &winner_participant, &delegation_record);

    // Competition is now Settled; winner_participant IS_WINNER is set.
    (winner_participant, winner_vault, loser_vault)
}

#[test]
fn release_winner_vault_success() {
    let mut svm   = setup_svm();
    let authority = Keypair::new();
    let crank     = Keypair::new();
    let treasury  = Keypair::new();
    let winner    = Keypair::new();
    let loser     = Keypair::new();

    for kp in [&authority, &winner, &loser] {
        svm.airdrop(&kp.pubkey(), 10_000_000_000).unwrap();
    }

    initialize_protocol_config(&mut svm, &authority, &treasury.pubkey(), &crank.pubkey());

    let mut id = [0u8; 32]; id[0] = 30;
    let competition = create_competition(&mut svm, &authority, id, 1_000_000);
    force_competition_phase(&mut svm, &competition, Phase::Active, i64::MAX - 1);

    let (winner_participant, winner_vault, _) =
        setup_settled(&mut svm, &authority, &crank, &treasury, &winner, &loser, &competition);

    let seller_before   = lamports(&svm, &authority.pubkey());
    let treasury_before = lamports(&svm, &treasury.pubkey());

    release_vault(
        &mut svm,
        &crank,
        &winner.pubkey(),
        &authority.pubkey(),
        &competition,
        &winner_participant,
        &treasury.pubkey(),
    );

    // Vault must be drained.
    assert_eq!(lamports(&svm, &winner_vault), 0, "vault drained after release");

    // Seller must have received the net bid (51M * 97.5% = 49.725M).
    let seller_after = lamports(&svm, &authority.pubkey());
    assert!(seller_after > seller_before, "seller balance must increase");
    assert_eq!(seller_after - seller_before, 49_725_000, "seller receives bid minus 2.5% fee");

    // Treasury must have received the protocol fee (51M * 2.5% = 1.275M).
    let treasury_after = lamports(&svm, &treasury.pubkey());
    assert_eq!(treasury_after - treasury_before, 1_275_000, "treasury receives 2.5% fee");
}

#[test]
fn release_wrong_vault_non_winner_fails() {
    let mut svm   = setup_svm();
    let authority = Keypair::new();
    let crank     = Keypair::new();
    let treasury  = Keypair::new();
    let winner    = Keypair::new();
    let loser     = Keypair::new();

    for kp in [&authority, &winner, &loser] {
        svm.airdrop(&kp.pubkey(), 10_000_000_000).unwrap();
    }

    initialize_protocol_config(&mut svm, &authority, &treasury.pubkey(), &crank.pubkey());

    let mut id = [0u8; 32]; id[0] = 31;
    let competition = create_competition(&mut svm, &authority, id, 1_000_000);
    force_competition_phase(&mut svm, &competition, Phase::Active, i64::MAX - 1);

    let (_, _, loser_vault) =
        setup_settled(&mut svm, &authority, &crank, &treasury, &winner, &loser, &competition);

    // Try to Release the loser's vault → NOT winner → should fail.
    let (loser_participant, _) =
        tyche_core::instruction_builder::register_bid::derive_participant_record_pda(
            &competition, &loser.pubkey(),
        );
    let (ix, _) = tyche_escrow::instruction_builder::release::build_release(
        &loser.pubkey(),
        &authority.pubkey(),
        &crank.pubkey(),
        &competition,
        &loser_participant,
        &treasury.pubkey(),
    );
    drop(loser_vault);
    expect_failure(&mut svm, ix, &[&crank]);
}

#[test]
fn refund_loser_after_settlement() {
    let mut svm   = setup_svm();
    let authority = Keypair::new();
    let crank     = Keypair::new();
    let treasury  = Keypair::new();
    let winner    = Keypair::new();
    let loser     = Keypair::new();

    for kp in [&authority, &winner, &loser] {
        svm.airdrop(&kp.pubkey(), 10_000_000_000).unwrap();
    }

    initialize_protocol_config(&mut svm, &authority, &treasury.pubkey(), &crank.pubkey());

    let mut id = [0u8; 32]; id[0] = 32;
    let competition = create_competition(&mut svm, &authority, id, 1_000_000);
    force_competition_phase(&mut svm, &competition, Phase::Active, i64::MAX - 1);

    let (_, _, loser_vault) =
        setup_settled(&mut svm, &authority, &crank, &treasury, &winner, &loser, &competition);

    let loser_before = lamports(&svm, &loser.pubkey());
    refund_vault(&mut svm, &loser, &competition);

    assert_eq!(lamports(&svm, &loser_vault), 0, "loser vault drained after refund");
    let loser_after = lamports(&svm, &loser.pubkey());
    assert!(loser_after > loser_before, "loser must receive lamports back");
    assert!(loser_after >= loser_before + 50_000_000, "loser gets full deposit back");
}

#[test]
fn refund_winner_after_settlement_fails() {
    let mut svm   = setup_svm();
    let authority = Keypair::new();
    let crank     = Keypair::new();
    let treasury  = Keypair::new();
    let winner    = Keypair::new();
    let loser     = Keypair::new();

    for kp in [&authority, &winner, &loser] {
        svm.airdrop(&kp.pubkey(), 10_000_000_000).unwrap();
    }

    initialize_protocol_config(&mut svm, &authority, &treasury.pubkey(), &crank.pubkey());

    let mut id = [0u8; 32]; id[0] = 33;
    let competition = create_competition(&mut svm, &authority, id, 1_000_000);
    force_competition_phase(&mut svm, &competition, Phase::Active, i64::MAX - 1);

    setup_settled(&mut svm, &authority, &crank, &treasury, &winner, &loser, &competition);

    // Winner tries to refund → WinnerCannotRefund.
    let (ix, _) = tyche_escrow::instruction_builder::refund::build_refund(
        &winner.pubkey(),
        &competition,
    );
    expect_failure(&mut svm, ix, &[&winner]);
}
