# Tyche Security

This document covers the threat model, the specific vulnerabilities the Tyche
architecture guards against, the invariants the programs enforce, the security
properties of the CEE/PER integration, and the checklist run before deployment.

---

## Threat Model

Tyche processes SOL custody and asset transfers for arbitrary participants. The threat
model assumes:

- **Adversarial participants.** Any bidder may attempt to double-spend, grief others,
  or extract value through timing attacks.
- **Adversarial sellers.** A seller may attempt to shill bid, cancel after seeing bids,
  or interfere with the settlement flow.
- **Adversarial cranks.** The SessionManager crank is a trusted operator for
  ExtendCompetition and CloseCompetition. It must not be replaceable by an arbitrary
  caller.
- **Compromised PER session.** The TEE provides hardware guarantees, but the program
  must be correct in isolation — the sealed fields must be handled safely even if the
  PER infrastructure behaved unexpectedly.
- **No trust in the Tyche team.** The programs must enforce all invariants without
  relying on off-chain operator correctness. Fund release must be gated entirely on
  on-chain state.

---

## Vulnerabilities This Architecture Guards Against

### Bot Sniping

Public Solana auctions close at a deterministic block. Bots monitor the mempool, detect
the auction close window, and submit a higher bid in the same or next slot — faster than
any human can respond.

Tyche defense: the soft-close mechanism mathematically prevents this. Any bid placed
within `soft_close_window` seconds of `end_time` extends the auction by
`soft_close_extension` seconds. This runs via `ExtendCompetition` which is called by the
SessionManager crank. There is no "final slot" to snipe — the window resets on every bid.
The `max_soft_closes` field caps the maximum total extension to prevent infinite loops.

### Whale Surveillance

In public auctions, a buyer who would pay 100 SOL watches for a bid of 60 SOL and then
bids 61 SOL. The seller loses 39 SOL of genuine buyer valuation.

Tyche defense: `current_high_bid` is sealed inside the TEE during the active phase.
Bidders cannot calibrate their bid to anyone else's. Each bidder bids their own
valuation. The price discovered reflects genuine market demand.

### Shill Bidding

Sellers or associates submit fake bids to inflate the apparent price against genuine
buyers. With public bid history the pattern may be visible but is unprovable.

Tyche defense: `current_winner` is sealed. A shill bidder cannot target a specific
genuine bidder without knowing who is winning. Additionally, `CreateAuction` must
enforce that the seller's pubkey cannot be used as a bidder pubkey in the same
competition. This is checked in the `CreateAuction` processor against
`CompetitionState.authority`.

### Double-Spend on Escrow

A malicious participant attempts to withdraw funds twice — once via `ReleaseWinner` and
once via `Refund`, or to trigger `Refund` on a winning vault.

Tyche defense: `EscrowVault.released` is read before every transfer and set to `true`
before the function returns. Both `ReleaseWinner` and `Refund` check this flag. Because
Solana transactions are atomic, setting the flag and transferring funds cannot be
separated. A vault that has been released cannot be released again.

### Asset Double-Transfer

The winning asset is transferred twice via two calls to `FinalizeAuction`.

Tyche defense: `AuctionState.asset_transferred` is set to `true` in `FinalizeAuction`
before the token transfer instruction. The guard at the top of `FinalizeAuction` reads
this flag and returns `AssetAlreadyTransferred` if it is set.

### Unauthorized Phase Transitions

An arbitrary caller submits `CloseCompetition` before the timer expires, or calls
`ExtendCompetition` with no bid having occurred.

Tyche defense:
- `CloseCompetition` guards: `phase == Active`, `clock.unix_timestamp >= end_time`.
  No exception. The crank cannot close early.
- `ExtendCompetition` guards: `phase == Active`, `soft_close_count < max_soft_closes`,
  `(end_time - clock.unix_timestamp) < soft_close_window`, and the caller must be the
  authorized crank authority stored on `CompetitionState`. Arbitrary callers are
  rejected.
- `SettleCompetition` guards: `phase == Closing`, and the `delegation_record` PDA for
  `CompetitionState` must not exist (proving full undelegation). Settling before
  undelegation is impossible.

### Account Substitution

A caller passes a different account in place of the expected PDA — for example, passing
a fake `CompetitionState` to trick `ReleaseWinner` into releasing funds for a fake
settled competition.

Tyche defense: every instruction that reads a PDA account verifies the account
discriminator in the first 8 bytes. A fake account with a wrong discriminator is
rejected immediately. The PDA derivation is also verified by recomputing the expected
address from seeds and checking it matches the provided account key.

### Arithmetic Overflow on Bid Amounts

A bid amount overflows when added to the minimum increment, causing incorrect
comparisons.

Tyche defense: all arithmetic on user-supplied values uses checked arithmetic variants
(`checked_add`, `checked_sub`, `checked_mul`). Any overflow returns
`ArithmeticOverflow` error and the transaction is rejected. No bare arithmetic
operators on user-supplied values anywhere in the program crates.

### Sealed Field Leakage via Logs

A `PlaceBid` processor logs the bid amount, leaking it from the TEE session to the
public transaction log.

Tyche defense: `PlaceBid` must emit no log output that includes the bid amount or
bidder identity. The `TycheBidPlaced` event includes only `competition` and
`participant_count` — no amounts, no addresses. All log calls in `bid.rs` must be
audited before every deployment.

---

## Program Invariants

The following invariants must hold at all times. Any code change that could violate
these invariants must include an explicit justification in a comment.

### tyche-core invariants

1. `CompetitionState.phase` transitions are strictly one-directional and gated.
   No transition can skip a phase. No transition can go backward.

2. `CancelCompetition` is only callable when `participant_count == 0`. A competition
   with any bids cannot be cancelled — funds are already locked in vaults.

3. `ActivateCompetition` can only be called once per competition. Once the account
   is delegated to the PER, `phase == Active` and the delegate guard prevents
   re-activation.

4. `ExtendCompetition` is callable only by the designated crank authority stored in
   `CompetitionState.authority`. Storing the crank authority on-chain ensures no
   off-chain operator change can bypass the on-chain check.

5. `soft_close_count` never exceeds `max_soft_closes`. The guard is checked before
   incrementing and the instruction returns an error if the cap is reached.

### tyche-escrow invariants

1. `EscrowVault.released` starts `false` and transitions to `true` exactly once,
   via either `ReleaseWinner` or `Refund`. It never returns to `false`.

2. `ReleaseWinner` is callable only when `CompetitionState.phase == Settled` and
   `vault.depositor == winner`. The winner is read from the settled
   `CompetitionState`, not from any caller argument.

3. `Refund` is callable only when phase is `Settled` or `Cancelled`, and only for
   vaults whose `depositor` is not the competition winner (when Settled).

4. The lamport transfer amount equals exactly `vault.amount`. No partial releases.
   The vault account is closed (lamports zeroed, account reclaimed) after release.

### tyche-auction invariants

1. `current_high_bid` only increases. A bid that does not exceed
   `current_high_bid + min_bid_increment` is rejected. A bid exactly equal to
   `current_high_bid + min_bid_increment` is rejected — the increment is strictly
   greater than.

2. `asset_transferred` starts `false` and transitions to `true` exactly once, via
   `FinalizeAuction`. The asset escrow account is closed after transfer.

3. The seller stored on `AuctionState` cannot be the same pubkey as any bidder.
   This is enforced at `CreateAuction` by comparing the seller key against
   `CompetitionState.authority` and rejecting if they differ from expected.

---

## Security Checklist (Pre-Deployment)

Run this checklist against all three programs before any devnet or mainnet deployment.

### Fund Custody

- [ ] Every fund-moving instruction recomputes and verifies PDA derivation or checks
      the cached bump against a recomputed address.
- [ ] `EscrowVault.released` is read before the transfer in both `ReleaseWinner` and
      `Refund`. It is set to `true` before the function returns (not after).
- [ ] All arithmetic on user-supplied values (`amount`, `min_bid_increment`) uses
      `checked_*` methods. Search for bare `+`, `-`, `*` on user-supplied `u64` values
      and verify none exist.
- [ ] No instruction can transfer more lamports than the vault holds.

### State Machine

- [ ] Phase transition guards are present in every instruction that changes phase.
- [ ] `ExtendCompetition` rejects callers whose key does not match the stored
      `crank_authority`. Verify this check is the first guard, before any state reads.
- [ ] `SettleCompetition` checks that the `delegation_record` PDA does not exist before
      accepting the winner and amount arguments.
- [ ] `CancelCompetition` checks `participant_count == 0` when phase is Active.

### Account Validation

- [ ] Every instruction that reads a PDA account checks the first 8 bytes against the
      expected discriminator.
- [ ] Every PDA account check recomputes the expected address from seeds and verifies
      it matches the provided account key.
- [ ] `PlaceBid` verifies `amount > current_high_bid + min_bid_increment` (strictly
      greater than, not greater than or equal to).
- [ ] `CreateAuction` verifies the seller is not the same pubkey as
      `CompetitionState.authority` (or whichever field identifies the expected bidder
      exclusion).

### Information Leakage

- [ ] All log statements in `tyche-auction/src/processor/bid.rs` are reviewed. None
      include bid amount, bidder pubkey, or current winner.
- [ ] `TycheBidPlaced` event fields: only `competition: Pubkey` and
      `participant_count: u32`. No other fields.
- [ ] No other event emitted during the active phase includes sealed field values.

### CPI Safety

- [ ] All CPI calls to tyche-escrow from tyche-auction pass the correct program ID.
- [ ] The delegation CPI in tyche-common matches the current delegation program IDL.
      Verify discriminator values (Delegate = 0, Undelegate = 3) against the deployed
      program before each deployment.
- [ ] `invoke_signed` calls use the correct PDA seeds for signing.
