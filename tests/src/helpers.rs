//! Shared test utilities for the Tyche protocol test suite.
//!
//! Provides: litesvm setup, funded wallets, instruction helpers,
//! and direct account state manipulation for bypassing the MagicBlock
//! delegation CPI (which is not available in the litesvm environment).

use litesvm::LiteSVM;
use solana_sdk::{
    account::{Account, AccountSharedData},
    clock::Clock,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use tyche_common::{
    constants::{MAX_SOFT_CLOSES, SOFT_CLOSE_EXTENSION_SECS, SOFT_CLOSE_WINDOW_SECS},
    phase::Phase,
    seeds::{AUCTION_SEED, BID_SEED},
};
use tyche_core::{
    instruction_builder::{
        create::build_create_competition,
        initialize_protocol_config::{
            build_initialize_protocol_config, derive_protocol_config_pda,
        },
        register_bid::derive_participant_record_pda,
    },
    state::competition::CompetitionState,
};
use tyche_auction::{
    args::{create_auction::CreateAuctionArgs, place_bid::PlaceBidArgs},
    instruction_builder::{
        cancel_auction::cancel_auction as build_cancel_auction,
        create_auction::create_auction as build_create_auction,
        finalize_auction::finalize_auction as build_finalize_auction,
        place_bid::place_bid as build_place_bid,
    },
};
use tyche_escrow::instruction_builder::{
    deposit::build_deposit, refund::build_refund, release::build_release,
};

// ── Program IDs ───────────────────────────────────────────────────────────────

pub fn core_id()   -> Pubkey { Pubkey::from(*tyche_core::ID.as_array())   }
pub fn escrow_id() -> Pubkey { Pubkey::from(*tyche_escrow::ID.as_array()) }
pub fn auction_id()-> Pubkey { Pubkey::from(*tyche_auction::ID.as_array())}

// Path to compiled SBF programs.
// CARGO_MANIFEST_DIR = {workspace}/tests so /../target/deploy resolves to
// {workspace}/target/deploy — where cargo build-sbf places the .so files.
const DEPLOY_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../target/deploy");

// ── LiteSVM setup ─────────────────────────────────────────────────────────────

/// Create a litesvm context with all three Tyche programs loaded and the clock
/// fixed at genesis (unix_timestamp = 0) so `start_time = 1` always works.
///
/// **Prerequisite**: `just build` (or `cargo build-sbf`) must have been run.
pub fn setup_svm() -> LiteSVM {
    let mut svm = LiteSVM::new();

    // Fix the clock at genesis so time-gated instructions behave predictably.
    svm.set_sysvar(&Clock {
        slot:                    1,
        epoch_start_timestamp:   0,
        epoch:                   0,
        leader_schedule_epoch:   0,
        unix_timestamp:          0,
    });

    let core_so    = format!("{DEPLOY_DIR}/tyche_core.so");
    let escrow_so  = format!("{DEPLOY_DIR}/tyche_escrow.so");
    let auction_so = format!("{DEPLOY_DIR}/tyche_auction.so");

    svm.add_program_from_file(core_id(),    &core_so)
        .unwrap_or_else(|_| panic!("tyche_core.so not found at {core_so} — run `just build` first"));
    svm.add_program_from_file(escrow_id(),  &escrow_so)
        .unwrap_or_else(|_| panic!("tyche_escrow.so not found at {escrow_so} — run `just build` first"));
    svm.add_program_from_file(auction_id(), &auction_so)
        .unwrap_or_else(|_| panic!("tyche_auction.so not found at {auction_so} — run `just build` first"));

    svm
}

// ── Transaction helpers ───────────────────────────────────────────────────────

/// Send a single instruction; panic with logs if it fails.
pub fn send(svm: &mut LiteSVM, ix: Instruction, signers: &[&Keypair]) {
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&signers[0].pubkey()),
        signers,
        blockhash,
    );
    svm.send_transaction(tx)
        .unwrap_or_else(|e| panic!("transaction failed: {:?}", e));
}

/// Send multiple instructions in one transaction; panic with logs if it fails.
pub fn send_all(svm: &mut LiteSVM, ixs: &[Instruction], signers: &[&Keypair]) {
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        ixs,
        Some(&signers[0].pubkey()),
        signers,
        blockhash,
    );
    svm.send_transaction(tx)
        .unwrap_or_else(|e| panic!("transaction failed: {:?}", e));
}

/// Assert that an instruction fails; panic if it unexpectedly succeeds.
pub fn expect_failure(svm: &mut LiteSVM, ix: Instruction, signers: &[&Keypair]) {
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&signers[0].pubkey()),
        signers,
        blockhash,
    );
    assert!(
        svm.send_transaction(tx).is_err(),
        "expected transaction to fail but it succeeded"
    );
}

// ── Account inspection ────────────────────────────────────────────────────────

/// Lamport balance of an account (0 if the account does not exist).
pub fn lamports(svm: &LiteSVM, address: &Pubkey) -> u64 {
    svm.get_account(address).map(|a| a.lamports()).unwrap_or(0)
}

// ── State manipulation (delegation bypass) ────────────────────────────────────

/// Directly set the phase (and optionally `end_time`) on a `CompetitionState`
/// account, bypassing the MagicBlock delegation CPI that is unavailable in
/// litesvm. Used to simulate what `ActivateCompetition` and `CloseCompetition`
/// would do on a live PER.
///
/// `end_time` is only written when transitioning to `Active` or `Settling`;
/// pass `0` to leave it unchanged (e.g. when forcing `Cancelled`).
pub fn force_competition_phase(
    svm: &mut LiteSVM,
    competition: &Pubkey,
    phase: Phase,
    end_time: i64,
) {
    let account = svm
        .get_account(competition)
        .expect("competition account must exist before forcing phase");

    let mut data = account.data().to_vec();
    {
        let state = bytemuck::from_bytes_mut::<CompetitionState>(&mut data);
        state.phase = phase as u8;
        if end_time != 0 {
            state.end_time = end_time;
        }
    }

    let modified = Account {
        lamports:    account.lamports(),
        data,
        owner:       account.owner(),
        executable:  account.executable(),
        rent_epoch:  account.rent_epoch(),
    };
    svm.set_account(*competition, AccountSharedData::from(modified))
        .expect("set_account failed");
}

// ── Protocol setup ────────────────────────────────────────────────────────────

/// Initialize `ProtocolConfig` and return its pubkey.
///
/// Defaults:
/// - `fee_basis_points = 250` (2.5 %)
/// - `max_soft_closes_cap = 5`
/// - `min_reserve_price = 1_000_000` lamports (0.001 SOL)
/// - `min_duration_secs = 60` (1 minute)
pub fn initialize_protocol_config(
    svm:       &mut LiteSVM,
    authority: &Keypair,
    treasury:  &Pubkey,
    crank:     &Pubkey,
) -> Pubkey {
    let (ix, config) = build_initialize_protocol_config(
        &authority.pubkey(),
        &authority.pubkey(), // payer
        &authority.pubkey(), // emergency_authority (same for tests)
        treasury,
        crank,
        250,       // fee_basis_points: 2.5 %
        5,         // max_soft_closes_cap
        1_000_000, // min_reserve_price: 0.001 SOL
        60,        // min_duration_secs: 1 minute
    );
    send(svm, ix, &[authority]);
    config
}

// ── Competition helpers ───────────────────────────────────────────────────────

/// Create a `CompetitionState` in `Scheduled` phase and return its pubkey.
///
/// `id` must be unique per authority. `reserve_price` must be >= 1_000_000
/// (the `min_reserve_price` set by `initialize_protocol_config`).
pub fn create_competition(
    svm:           &mut LiteSVM,
    authority:     &Keypair,
    id:            [u8; 32],
    reserve_price: u64,
) -> Pubkey {
    let (ix, competition) = build_create_competition(
        &authority.pubkey(),
        &authority.pubkey(), // payer
        id,
        tyche_common::asset_type::AssetType::Nft as u8,
        1,                       // start_time = 1 (valid at genesis clock = 0)
        3600,                    // duration_secs: 1 hour
        SOFT_CLOSE_WINDOW_SECS,
        SOFT_CLOSE_EXTENSION_SECS,
        MAX_SOFT_CLOSES,
        reserve_price,
    );
    send(svm, ix, &[authority]);
    competition
}

// ── Auction helpers ───────────────────────────────────────────────────────────

/// Derive the `AuctionState` PDA for a competition.
pub fn derive_auction_pda(competition: &Pubkey) -> Pubkey {
    let (pda, _) = Pubkey::find_program_address(
        &[AUCTION_SEED, &competition.to_bytes()],
        &auction_id(),
    );
    pda
}

/// Create an `AuctionState` for a competition and return its pubkey.
pub fn create_auction(
    svm:               &mut LiteSVM,
    seller:            &Keypair,
    competition:       &Pubkey,
    asset_mint:        &Pubkey,
    min_bid_increment: u64,
) -> Pubkey {
    let auction_state = derive_auction_pda(competition);
    let args = CreateAuctionArgs {
        asset_mint:        pinocchio::Address::new_from_array(asset_mint.to_bytes()),
        min_bid_increment,
    };
    let ix = build_create_auction(
        &auction_state,
        competition,
        &seller.pubkey(),
        &seller.pubkey(), // payer
        args,
    );
    send(svm, ix, &[seller]);
    auction_state
}

/// Cancel an `AuctionState` (only valid when competition is `Cancelled`).
pub fn cancel_auction(
    svm:          &mut LiteSVM,
    seller:       &Keypair,
    auction_state: &Pubkey,
    competition:  &Pubkey,
) {
    let ix = build_cancel_auction(
        auction_state,
        competition,
        &seller.pubkey(),
        &seller.pubkey(), // rent_recipient
    );
    send(svm, ix, &[seller]);
}

// ── Escrow helpers ────────────────────────────────────────────────────────────

/// Deposit `amount` lamports into the bidder's escrow vault and return the
/// vault pubkey. The competition must be in `Active` phase.
pub fn deposit(
    svm:         &mut LiteSVM,
    bidder:      &Keypair,
    competition: &Pubkey,
    amount:      u64,
) -> Pubkey {
    let (ix, vault) = build_deposit(
        &bidder.pubkey(),
        &bidder.pubkey(), // payer
        competition,
        amount,
    );
    send(svm, ix, &[bidder]);
    vault
}

/// Place a bid and return `(bid_record, participant_record)` pubkeys.
/// The competition must be `Active` and the vault must hold >= `amount`.
pub fn place_bid(
    svm:           &mut LiteSVM,
    bidder:        &Keypair,
    competition:   &Pubkey,
    auction_state: &Pubkey,
    vault:         &Pubkey,
    amount:        u64,
) -> (Pubkey, Pubkey) {
    let competition_bytes = competition.to_bytes();
    let bidder_bytes      = bidder.pubkey().to_bytes();

    let (bid_record, _) = Pubkey::find_program_address(
        &[BID_SEED, &competition_bytes, &bidder_bytes],
        &auction_id(),
    );
    let (participant_record, _) =
        derive_participant_record_pda(competition, &bidder.pubkey());

    let ix = build_place_bid(
        auction_state,
        competition,
        &bid_record,
        vault,
        &bidder.pubkey(),
        &bidder.pubkey(), // payer
        &participant_record,
        PlaceBidArgs { amount },
    );
    send(svm, ix, &[bidder]);
    (bid_record, participant_record)
}

/// Finalize an auction after settlement.  `delegation_record` must be a pubkey
/// with zero lamports — in litesvm any unknown pubkey satisfies this.
pub fn finalize_auction(
    svm:               &mut LiteSVM,
    crank:             &Keypair,
    auction_state:     &Pubkey,
    competition:       &Pubkey,
    winner_participant: &Pubkey,
    delegation_record: &Pubkey,
) {
    let (protocol_config, _) = derive_protocol_config_pda();
    let ix = build_finalize_auction(
        auction_state,
        competition,
        winner_participant,
        &crank.pubkey(),
        &protocol_config,
        delegation_record,
    );
    send(svm, ix, &[crank]);
}

/// Release the winner's vault: bid → seller, fee → treasury, rent → winner.
/// The competition must be `Settled` and the vault owner must be IS_WINNER.
pub fn release_vault(
    svm:                &mut LiteSVM,
    crank:              &Keypair,
    winner:             &Pubkey,
    seller:             &Pubkey,
    competition:        &Pubkey,
    participant_record: &Pubkey,
    treasury:           &Pubkey,
) {
    let (ix, _) = build_release(
        winner,
        seller,
        &crank.pubkey(),
        competition,
        participant_record,
        treasury,
    );
    send(svm, ix, &[crank]);
}

/// Refund the full vault balance to a losing (or cancelled) bidder.
pub fn refund_vault(
    svm:         &mut LiteSVM,
    bidder:      &Keypair,
    competition: &Pubkey,
) {
    let (ix, _) = build_refund(&bidder.pubkey(), competition);
    send(svm, ix, &[bidder]);
}
