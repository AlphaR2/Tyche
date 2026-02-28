# tyche-tests

Integration and unit test suite for the Tyche protocol programs. Tests run on the
host target (not SBF) using `litesvm` for fast, in-process Solana program simulation.

---

## Test Structure

```
tests/src/
+-- lib.rs                   test crate root and shared utilities
+-- tyche_core.rs            unit tests for tyche-core instructions
+-- tyche_escrow.rs          unit tests for tyche-escrow instructions
+-- tyche_auction.rs         unit tests for tyche-auction instructions
+-- integration/
    +-- full_flow.rs         end-to-end integration test (all three programs)
```

### Unit Tests (`tyche_core.rs`, `tyche_escrow.rs`, `tyche_auction.rs`)

Each file tests one program in isolation using litesvm. Covers:

- Happy path for every instruction
- Guard failures (wrong phase, wrong authority, wrong account, etc.)
- Discriminator rejection (wrong account type passed)
- Double-spend protection (releasing/refunding twice)
- Arithmetic edge cases (zero amounts, maximum values)

### Integration Test (`integration/full_flow.rs`)

The full auction lifecycle in one test using litesvm with all three programs loaded.

Sequence:
1. Create competition (tyche-core)
2. Create auction, lock asset in escrow (tyche-auction)
3. Activate competition (tyche-core)
4. Place bids from three different wallets (tyche-auction)
5. Extend competition via soft-close trigger (tyche-core)
6. Close competition (tyche-core)
7. Settle competition (tyche-core) — simulates post-undelegation
8. Finalize auction — asset transfers to winner (tyche-auction)
9. Release winner vault to seller (tyche-escrow via CPI)
10. Refund losing vaults (tyche-escrow via CPI)
11. Verify: winner holds asset, seller received SOL, losers refunded



---

## Running Tests

### Rust Tests (no SBF toolchain required)

```sh
# All unit and integration tests
just test-rust

# Equivalent to:
cargo test -p tyche-tests

# Run a specific test file
cargo test -p tyche-tests tyche_core

# Run the integration test only
cargo test -p tyche-tests integration

# Run with output (see program logs)
cargo test -p tyche-tests -- --nocapture
```

### TypeScript Integration Tests (requires local validator)

```sh
# Start local validator (separate terminal)
solana-test-validator

# Run TypeScript tests
just test-ts

# Equivalent to:
npx tsx tests/integration/full-flow.test.ts
```

The TypeScript integration test runs the full flow against a local validator and tests
MagicBlock PER delegation. Requires the local MagicBlock validator running alongside
the Solana test validator. See `docs/CEE.md` for validator setup instructions.

---

## What Each Test File Covers

### `tyche_core.rs`

| Test                                  | Description                                      |
|---------------------------------------|--------------------------------------------------|
| create_competition_success            | Happy path CreateCompetition                     |
| create_competition_wrong_payer        | Payer is not signer — rejected                   |
| activate_before_start_time            | ActivateCompetition too early — InvalidPhase     |
| extend_beyond_max_soft_closes         | SoftCloseCapReached error                        |
| extend_outside_window                 | SoftCloseNotArmed error                          |
| close_before_expiry                   | AuctionNotExpired error                          |
| close_after_expiry                    | Happy path CloseCompetition                      |
| settle_before_close                   | InvalidPhase error                               |
| cancel_with_participants              | HasParticipants error                            |
| cancel_scheduled_no_participants      | Happy path CancelCompetition                     |
| phase_transitions_in_order           | Full Scheduled -> Active -> Closing -> Settled   |

### `tyche_escrow.rs`

| Test                                  | Description                                      |
|---------------------------------------|--------------------------------------------------|
| deposit_creates_vault                 | First deposit creates EscrowVault PDA            |
| deposit_tops_up_existing              | Second deposit increments vault.amount           |
| release_winner_success                | Happy path ReleaseWinner                         |
| release_winner_wrong_vault            | Non-winner vault — NotWinner error               |
| release_winner_already_released       | VaultAlreadyReleased error                       |
| refund_loser_success                  | Happy path Refund                                |
| refund_winner_vault_when_settled      | IsWinner error                                   |
| refund_already_released               | VaultAlreadyReleased error                       |
| refund_on_cancellation                | Refund works when phase == Cancelled             |
| double_release_attempt                | Second release fails — VaultAlreadyReleased      |

### `tyche_auction.rs`

| Test                                  | Description                                      |
|---------------------------------------|--------------------------------------------------|
| create_auction_locks_asset            | Asset transfers to program escrow                |
| create_auction_seller_is_bidder       | SellerIsBidder error                             |
| place_bid_success                     | Bid accepted, sealed fields updated              |
| place_bid_too_low                     | BidTooLow error                                  |
| place_bid_below_reserve               | BidBelowReserve error                            |
| place_bid_wrong_phase                 | InvalidPhase error                               |
| place_bid_creates_participant         | ParticipantRecord created on first bid           |
| place_bid_increments_participant_count| participant_count increments on new bidder       |
| finalize_transfers_asset              | Asset moves to winner token account              |
| finalize_double_call                  | AssetAlreadyTransferred error                    |
| finalize_wrong_phase                  | InvalidPhase error                               |

### `integration/full_flow.rs`

| Test                                  | Description                                      |
|---------------------------------------|--------------------------------------------------|
| full_auction_lifecycle                | All three programs, complete flow                |
| soft_close_extension_during_flow      | Soft-close triggers mid-auction                  |
| cancellation_with_zero_bids           | Cancel path, all vaults refunded                 |

---

## Shared Utilities (`lib.rs`)

The test crate root exposes utilities used across all test files:

- `create_test_context()` — initializes a litesvm context with all three programs loaded
- `fund_account(ctx, pubkey, lamports)` — airdrops SOL to a test wallet
- `mint_test_nft(ctx, authority)` — mints a test NFT for auction use
- `assert_lamports(ctx, pubkey, expected)` — asserts a wallet's SOL balance
- `assert_token_balance(ctx, ata, expected)` — asserts a token account's balance
