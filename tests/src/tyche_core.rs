//! Unit tests for `tyche-core` instructions.
//!
//! Each test is self-contained: it initialises a fresh litesvm context,
//! exercises one instruction or error path, and asserts the expected outcome.
//! Delegation-dependent instructions (`ActivateCompetition`, `CloseCompetition`,
//! `SettleCompetition`) are covered indirectly in `integration.rs`.

use solana_sdk::signature::{Keypair, Signer};
use tyche_core::instruction_builder::{
    cancel::build_cancel_competition,
    create::build_create_competition,
    initialize_protocol_config::derive_protocol_config_pda,
};
use tyche_common::{
    asset_type::AssetType,
    constants::{MAX_SOFT_CLOSES, SOFT_CLOSE_EXTENSION_SECS, SOFT_CLOSE_WINDOW_SECS},
    phase::Phase,
};
use crate::helpers::*;

// ── Helpers ────────────────────────────────────────────────────────────────────

/// Standard competition ID used across tests.
fn cid(n: u8) -> [u8; 32] {
    let mut id = [0u8; 32];
    id[0] = n;
    id
}

/// Common test fixture: svm + authority + treasury + crank + protocol_config.
fn ctx() -> (litesvm::LiteSVM, Keypair, Keypair, Keypair) {
    let mut svm   = setup_svm();
    let authority = Keypair::new();
    let treasury  = Keypair::new();
    let crank     = Keypair::new();

    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();

    initialize_protocol_config(&mut svm, &authority, &treasury.pubkey(), &crank.pubkey());

    (svm, authority, treasury, crank)
}

// ── CreateCompetition ─────────────────────────────────────────────────────────

#[test]
fn create_competition_success() {
    let (mut svm, authority, _, _) = ctx();
    let competition = create_competition(&mut svm, &authority, cid(1), 1_000_000);

    // Account must exist and be owned by tyche-core.
    let account = svm.get_account(&competition).expect("competition account must exist");
    assert_eq!(account.owner(), core_id());
    assert!(!account.data().is_empty());
}

#[test]
fn create_competition_below_min_reserve_price_fails() {
    // min_reserve_price = 1_000_000; supply 999_999 → should fail.
    let (mut svm, authority, _, _) = ctx();
    let (ix, _) = build_create_competition(
        &authority.pubkey(),
        &authority.pubkey(),
        cid(2),
        AssetType::Nft as u8,
        1,    // start_time at genesis
        3600,
        SOFT_CLOSE_WINDOW_SECS,
        SOFT_CLOSE_EXTENSION_SECS,
        MAX_SOFT_CLOSES,
        999_999, // below minimum
    );
    expect_failure(&mut svm, ix, &[&authority]);
}

#[test]
fn create_competition_below_min_duration_fails() {
    // min_duration_secs = 60; supply 59 → should fail.
    let (mut svm, authority, _, _) = ctx();
    let (ix, _) = build_create_competition(
        &authority.pubkey(),
        &authority.pubkey(),
        cid(3),
        AssetType::Nft as u8,
        1,
        59, // below minimum
        SOFT_CLOSE_WINDOW_SECS,
        SOFT_CLOSE_EXTENSION_SECS,
        MAX_SOFT_CLOSES,
        1_000_000,
    );
    expect_failure(&mut svm, ix, &[&authority]);
}

#[test]
fn create_competition_start_time_in_past_fails() {
    // Clock is fixed at unix_timestamp = 0; start_time = -1 is in the past.
    let (mut svm, authority, _, _) = ctx();
    let (ix, _) = build_create_competition(
        &authority.pubkey(),
        &authority.pubkey(),
        cid(4),
        AssetType::Nft as u8,
        -1, // past
        3600,
        SOFT_CLOSE_WINDOW_SECS,
        SOFT_CLOSE_EXTENSION_SECS,
        MAX_SOFT_CLOSES,
        1_000_000,
    );
    expect_failure(&mut svm, ix, &[&authority]);
}

#[test]
fn create_competition_duplicate_id_fails() {
    let (mut svm, authority, _, _) = ctx();
    create_competition(&mut svm, &authority, cid(5), 1_000_000);
    // Second CreateCompetition with the same id and authority → same PDA → already initialized.
    let (ix, _) = build_create_competition(
        &authority.pubkey(),
        &authority.pubkey(),
        cid(5),
        AssetType::Nft as u8,
        1,
        3600,
        SOFT_CLOSE_WINDOW_SECS,
        SOFT_CLOSE_EXTENSION_SECS,
        MAX_SOFT_CLOSES,
        1_000_000,
    );
    expect_failure(&mut svm, ix, &[&authority]);
}

// ── CancelCompetition (Scheduled path) ───────────────────────────────────────

#[test]
fn cancel_scheduled_competition_success() {
    let (mut svm, authority, _, _) = ctx();
    let competition = create_competition(&mut svm, &authority, cid(10), 1_000_000);

    // Scheduled → Cancelled: no delegation CPI needed.
    // Dummy pubkeys for MagicBlock accounts — unused on the Scheduled path.
    let dummy = Keypair::new().pubkey();
    let ix = build_cancel_competition(
        &competition,
        &authority.pubkey(),
        &dummy, // permission (ignored on Scheduled path)
        &dummy, // magic_context (ignored)
        &dummy, // magic_program (ignored)
    );
    send(&mut svm, ix, &[&authority]);

    // Verify the account still exists (lamports were not reclaimed — only phase changed).
    let account = svm.get_account(&competition).expect("competition must still exist after cancel");
    let state = bytemuck::from_bytes::<tyche_core::state::competition::CompetitionState>(account.data());
    assert_eq!(state.phase, Phase::Cancelled as u8, "phase must be Cancelled");
}

#[test]
fn cancel_competition_wrong_authority_fails() {
    let (mut svm, authority, _, _) = ctx();
    let competition = create_competition(&mut svm, &authority, cid(11), 1_000_000);

    let impostor = Keypair::new();
    svm.airdrop(&impostor.pubkey(), 1_000_000_000).unwrap();

    let dummy = Keypair::new().pubkey();
    let ix = build_cancel_competition(
        &competition,
        &impostor.pubkey(), // wrong authority
        &dummy,
        &dummy,
        &dummy,
    );
    expect_failure(&mut svm, ix, &[&impostor]);
}

// ── ProtocolConfig ────────────────────────────────────────────────────────────

#[test]
fn protocol_config_initialized_correctly() {
    let (svm, authority, treasury, crank) = ctx();

    let (config_pda, _) = derive_protocol_config_pda();
    let account = svm.get_account(&config_pda).expect("protocol config must exist");
    let config = bytemuck::from_bytes::<tyche_core::state::protocol_config::ProtocolConfig>(account.data());

    assert_eq!(config.fee_basis_points,  250,       "fee_basis_points");
    assert_eq!(config.max_soft_closes_cap, 5,        "max_soft_closes_cap");
    assert_eq!(config.min_reserve_price, 1_000_000, "min_reserve_price");
    assert_eq!(config.min_duration_secs, 60,         "min_duration_secs");
    assert_eq!(config.authority,         pinocchio::Address::new_from_array(authority.pubkey().to_bytes()));
    assert_eq!(config.treasury,          pinocchio::Address::new_from_array(treasury.pubkey().to_bytes()));
    assert_eq!(config.crank_authority,   pinocchio::Address::new_from_array(crank.pubkey().to_bytes()));
}
