# Tyche Architecture

This document is the authoritative technical reference for the Tyche protocol
architecture. It covers the CEE execution model, the MagicBlock PER integration,
the phase state machine, account relationships across all three programs, what
information is sealed during the active phase, the settlement flow, and how future
verticals fit into the same engine.

---

## The CEE Execution Model

CEE (Competitive Execution Engine) is the internal name for the execution context in
which Tyche competitions are processed. Every competition — whether an NFT auction, an
in-game item sale, a prediction market, or a liquidity batch — goes through the same
three-phase model.

```
COMMIT
  Participants declare their intent.
  In full sealed-bid mode: cryptographic commitment (hash of value + salt).
  In live auction mode: bid submitted directly to the PER RPC endpoint.
  The commitment enters the TEE. It is not visible on mainnet.

COMPETE
  The engine processes all commitments inside the MagicBlock TEE ephemeral rollup.
  Sub-50ms per operation. No participant can observe another's position.
  Aggregate state (participant count, reserve-met flag, time remaining) is public.
  Individual state (bid amounts, current winner, positions) is sealed.

CLEAR
  The session finalizes. The TEE produces a verifiable outcome.
  Accounts undelegate back to mainnet.
  An Intel TDX attestation certificate is published on-chain.
  Settlement executes automatically via the on-chain escrow program.
  No human intervention required at any step.
```

The relationship between CEE and TEE: TEE (Trusted Execution Environment) is the
hardware and software mechanism — Intel TDX in this case. CEE is the protocol-level
execution model that TEE enables. TEE is the mechanism. CEE is the product construct.

---

## System Layers

```
+----------------------------------------------------------+
|                      TYCHE SDK                           |
|  TypeScript integration layer for:                       |
|  -- Creating sealed-bid competitions                     |
|  -- Managing real-time bid logic                         |
|  -- Orchestrating trustless settlement                   |
+---------------------------+------------------------------+
                            |
                  TypeScript SDK calls
                            |
+---------------------------v------------------------------+
|               @tyche-protocol/sdk                        |
|                                                          |
|  TycheClient                                             |
|    AuctionClient   -- create, bid, finalize, getStatus   |
|    SessionManager  -- PER session lifecycle, crank       |
|                                                          |
|  SessionManager routes PlaceBid to PER RPC endpoint.    |
|  All other instructions go to mainnet RPC.               |
+---------------------------+------------------------------+
                            |
                  on-chain program calls
                            |
+---------------------------v------------------------------+
|              ON-CHAIN PROGRAMS (Pinocchio)               |
|                                                          |
|  tyche-core        tyche-escrow       tyche-auction      |
|  Machine phase     SOL custody        Auction logic      |
|                                                          |
|              tyche-voter-weight-plugin                   |
|              ─────────────────────────                   |
|              Realms / SPL Governance integration         |
|              Reads EscrowVaults to derive VoterWeight    |
+---------------------------+------------------------------+
                            |
            delegate / execute inside / undelegate
                            |
+---------------------------v------------------------------+
|    MAGICBLOCK PRIVATE EPHEMERAL ROLLUP (Intel TDX TEE)  |
|                                                          |
|  During ACTIVE phase:                                    |
|  -- CompetitionState and AuctionState delegated here    |
|  -- PlaceBid transactions route to PER RPC endpoint     |
|  -- current_high_bid and current_winner: SEALED          |
|  -- participant_count and phase: readable on mainnet     |
|  -- soft-close extension logic executes in real-time     |
|                                                          |
|  At close:                                               |
|  -- accounts undelegate back to mainnet                  |
|  -- Intel TDX attestation certificate published          |
|  -- final winner and price become public                 |
+----------------------------------------------------------+
```

---

## Phase State Machine

### CompetitionState Phase Transitions

```
                  CreateCompetition
                        |
                        v
                   SCHEDULED (0)
                        |
          ActivateCompetition (delegates to PER)
                        |
                        v
                    ACTIVE (1) <----+
                        |           |
                        |    ExtendCompetition
                        |    (soft-close crank)
                        |           |
              bid in soft-close window?
              yes ------+
                        |
              clock >= end_time
                        |
              CloseCompetition (crank)
                        |
                        v
                   SETTLING (2)
                        |
         PER undelegation complete
                        |
              SettleCompetition
                        |
                        v
                   SETTLED (3)
                        |
                  FinalizeAuction
                  ReleaseWinner
                  Refund (losers)


  SCHEDULED --CancelCompetition--> CANCELLED (4)
  ACTIVE (participant_count == 0) --CancelCompetition--> CANCELLED (4)
```

### Phase Values (on-chain u8)

| Value | Phase     | Description                                   |
|-------|-----------|-----------------------------------------------|
| 0     | Scheduled | Created, not yet active. Awaiting activation. |
| 1     | Active    | Delegated to PER. Bids routed to PER RPC.     |
| 2     | Settling  | Timer expired. Awaiting PER undelegation.      |
| 3     | Settled   | Final state. Winner determined. Funds moved.  |
| 4     | Cancelled | No bids. All vaults refunded.                 |

---

## Account Relationships

```
CompetitionState (tyche-core)
  PDA seeds: [b"competition", authority, id]
  |
  +-- referenced by --> AuctionState (tyche-auction)
  |                     PDA seeds: [b"auction", competition]
  |                     holds: asset_mint, asset_escrow, seller,
  |                            min_bid_increment, current_high_bid (SEALED),
  |                            current_winner (SEALED)
  |
  +-- referenced by --> EscrowVault (tyche-escrow) [one per bidder]
  |                     PDA seeds: [b"vault", competition, depositor]
  |                     holds: depositor, amount, released flag
  |
  +-- referenced by --> ParticipantRecord (tyche-core) [one per bidder]
                        PDA seeds: [b"participant", competition, participant]
                        holds: is_winner (SEALED during active), last_action
```

### What Is Sealed and What Is Public

During the ACTIVE phase, while CompetitionState and AuctionState are delegated to
the MagicBlock PER:

| Field                              | Visibility    | Location          |
|------------------------------------|---------------|-------------------|
| CompetitionState.phase             | Public        | Mainnet (readable)|
| CompetitionState.end_time          | Public        | Mainnet (readable)|
| CompetitionState.participant_count | Public        | Mainnet (readable)|
| CompetitionState.soft_close_count  | Public        | Mainnet (readable)|
| AuctionState.current_high_bid      | SEALED        | PER TEE only      |
| AuctionState.current_winner        | SEALED        | PER TEE only      |
| ParticipantRecord.is_winner        | SEALED        | PER TEE only      |
| EscrowVault.amount                 | Public        | Mainnet (readable)|

Note: EscrowVault accounts are not delegated to the PER. Bid amounts are visible in
vault balances during the active phase. The identity of the highest bidder and the
amount of the highest bid are what remain sealed. A bidder's total deposited amount
is not hidden — only whether they are winning is hidden.

Post-settlement, all fields become public on mainnet.

---

## The Soft-Close Mechanism

Soft-close prevents sniping by extending the auction end time whenever a bid arrives
within the closing window.

```
Parameters stored on CompetitionState:
  soft_close_window    -- seconds before end_time that arm the mechanism
  soft_close_extension -- seconds added to end_time per trigger
  max_soft_closes      -- hard cap on total extensions
  soft_close_count     -- extensions applied so far

Logic (runs in SessionManager crank, every 30 seconds):
  if phase == Active
  and (end_time - clock.unix_timestamp) < soft_close_window
  and a bid was placed since last check
  and soft_close_count < max_soft_closes:
    call ExtendCompetition
    end_time += soft_close_extension
    soft_close_count += 1
    emit TycheCompetitionExtended

Effect:
  A bidder placing a bid with 45 seconds remaining resets the window.
  There is no "last second" to snipe because the window resets on every bid.
  The auction ends only when a full soft_close_window elapses with no bids.
```

---

## Settlement Flow (End to End)

```
1. CloseCompetition (crank)
   -- phase: Active -> Settling
   -- guard: clock.unix_timestamp >= end_time
   -- emits: TycheCompetitionClosed

2. PER undelegation (MagicBlock infrastructure)
   -- CompetitionState and AuctionState undelegate back to mainnet
   -- sealed fields (current_high_bid, current_winner) become readable
   -- Intel TDX attestation certificate published

3. SettleCompetition (SessionManager)
   -- reads winner and final_amount from now-undelegated AuctionState off-chain, passes as args
   -- phase: Settling -> Settled
   -- guard: delegation_record PDA does not exist (proves full undelegation)
   -- writes winner and final_amount to CompetitionState
   -- emits: TycheCompetitionSettled { winner, final_amount }

4. FinalizeAuction (tyche-auction)
   -- transfers asset from program-owned escrow to winner token account
   -- guard: competition.phase == Settled, asset_transferred == false
   -- sets asset_transferred = true (prevents double transfer)
   -- emits: TycheAuctionFinalized

5. ReleaseWinner (tyche-escrow)
   -- transfers winner's EscrowVault lamports to seller
   -- guard: vault.depositor == winner, vault.released == false
   -- sets vault.released = true (prevents double-spend)

6. Refund x N (tyche-escrow)
   -- for each losing bidder: transfer vault lamports back to depositor
   -- guard: vault.depositor != winner, vault.released == false
   -- sets vault.released = true per vault
```

---

## CPI Relationships Between Programs

tyche-auction calls tyche-core and tyche-escrow via CPI:

```
tyche-auction::PlaceBid
  -- reads CompetitionState (tyche-core) to verify phase == Active
  -- calls tyche-escrow::Deposit CPI to lock bidder SOL
  -- updates AuctionState sealed fields inside PER

tyche-auction::FinalizeAuction
  -- reads CompetitionState (tyche-core) to verify phase == Settled
  -- transfers asset from program-owned ATA to winner
  -- calls tyche-escrow::ReleaseWinner CPI for winner vault
  -- calls tyche-escrow::Refund CPI for each losing vault
```

tyche-core calls the MagicBlock delegation program via CPI (hand-written in
tyche-common):

```
tyche-core::ActivateCompetition
  -- calls MagicBlock permission program: CreatePermission (if not yet created)
  -- calls MagicBlock permission program: DelegatePermission (if not yet delegated)
  -- calls delegation_program::delegate_account CPI
  -- authority is the sole ACL member: only they can read sealed fields inside the TEE
  -- CompetitionState PDA is live inside the TEE after this returns

tyche-core::SettleCompetition
  -- verifies delegation_record PDA does not exist (proves full undelegation)
  -- accepts winner and final_amount as instruction args
  -- writes them to CompetitionState as the canonical on-chain settlement record
  -- transitions phase to Settled
```

---

## The MagicBlock PER Integration

The MagicBlock delegation program address:
`DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh`

Relevant PDAs derived by the delegation program (needed for CPI account lists):

```
delegation_record:
  seeds: [b"delegation", delegated_account_pubkey]
  program: delegation_program

delegation_metadata:
  seeds: [b"delegation-metadata", delegated_account_pubkey]
  program: delegation_program

delegate_buffer:
  seeds: [b"delegate-buffer", delegated_account_pubkey]
  program: delegation_program
```

The `ephemeral_rollups_pinocchio` crate provides Pinocchio-native CPI builders for the
delegation program and the MagicBlock ACL permission program. `ActivateCompetition` uses
`CreatePermissionCpiBuilder`, `DelegatePermissionCpiBuilder`, and `delegate_account`
from this crate directly — no hand-written raw byte construction required.

During the active phase, bid transactions are submitted to the PER RPC endpoint, not to
mainnet. The SessionManager holds both endpoints and routes accordingly:

```
mainnet RPC  -- CreateCompetition, ActivateCompetition, ExtendCompetition,
                CloseCompetition, SettleCompetition, CancelCompetition,
                CreateAuction, FinalizeAuction, Deposit, ReleaseWinner, Refund

PER RPC      -- PlaceBid (all bid transactions during ACTIVE phase)
```

---

## Future Verticals

The CEE engine is not auction-specific. The same CompetitionState phase machine,
EscrowVault custody model, and TEE execution context serve all three planned verticals.

### Prediction Markets (V2)

A new program `tyche-prediction` consumes tyche-core and tyche-escrow the same way
tyche-auction does. CompetitionState manages the market lifecycle. EscrowVault holds
USDC position stakes. The prediction-specific logic (outcome shares, resolver staking,
commit-reveal oracle) lives in tyche-prediction.

Resolver auction: designated resolvers submit commit-reveal answers. The majority answer
is the accepted resolution. Minority resolvers lose stake to majority resolvers. This
is a Schelling-point game — the rational strategy is truthful reporting.

### Liquidity Markets (V3)

A new program `tyche-liquidity` introduces batch auction execution for token swaps.
CompetitionState manages batch windows (e.g. 2-second intervals). During each window,
all swap orders and LP quotes are sealed inside the PER. The clearing algorithm finds
the price at which maximum volume executes. All matched orders in the batch execute at
exactly the clearing price — no price discrimination, no front-running.

LP registration and quote submission are handled by tyche-liquidity. Settlement routes
through tyche-escrow's existing vault model.

---

## Realms Governance Integration

Tyche implements the **SPL Governance Add-in API** to allow SOL deposits to function as voting weight.

### The Voter-Weight Plugin
The `tyche-voter-weight-plugin` acts as an intermediary between `tyche-escrow` and `spl-governance`.

1. **Registrar**: Configured for a specific Realm and Mint. It points to the trusted `tyche-escrow` program and a specific `competition` scope.
2. **VoterWeightRecord**: A standard account expected by Realms. The plugin populates the `voter_weight` field by performing a zero-copy read of the voter's `EscrowVault` in `tyche-escrow`.
3. **Update Flow**:
   - The user calls `UpdateVoterWeightRecord`.
   - The plugin verifies the `EscrowVault` PDA and checks that it belongs to the correct competition.
   - The `vault.amount` is mapped 1:1 to `voter_weight`.
   - The user then votes on Realms, which reads the weight from the record.

This architecture ensures that governance is controlled by those actively participating in the protocol's secondary markets, aligning long-term protocol health with participant interests.
