# CEE — Competitive Execution Engine

This document explains what CEE is, how it differs from the underlying hardware
mechanism (TEE), which accounts enter the CEE context, which fields are sealed and
why, and what the attestation certificate proves at settlement.

---

## What CEE Stands For

CEE stands for Competitive Execution Engine.

It is the internal name for the execution context in which Tyche competitions are
processed. It is not a brand name, not a product name, and not a user-facing term.
It appears in internal code identifiers — `CeePhase`, `CeeConfig` — because it
precisely names the concept being implemented.

The naming relationship:

- **Tyche** is the protocol. What users and developers interact with.
- **CEE** is the engine inside Tyche. What the on-chain programs implement.
- **TEE** (Trusted Execution Environment) is the hardware mechanism CEE runs on.
- **PER** (Private Ephemeral Rollup) is the MagicBlock infrastructure that hosts CEE.

CEE is to TEE what an auction is to a database: the database is infrastructure,
the auction is the application construct built on it. CEE is the application construct.
TEE and PER are the infrastructure that makes it possible.

---

## How CEE Differs from TEE

TEE is hardware. Specifically, Intel TDX (Trust Domain Extensions) — a CPU feature
that creates isolated memory regions (trust domains) where code executes without the
host OS, hypervisor, or any external observer being able to read the memory contents.

CEE is a protocol concept. It describes the three-phase execution model that Tyche
runs inside the TEE:

```
COMMIT   -- participants submit their values (bids, positions, quotes) to the TEE
COMPETE  -- the engine processes all values according to competition rules
CLEAR    -- the engine produces a verifiable outcome and triggers settlement
```

TEE enforces the hardware-level isolation that makes COMMIT values unreadable. CEE is
the structured protocol that determines what values are committed, how competition is
run, and how outcomes are settled.

A useful analogy: HTTPS uses TLS to encrypt data in transit. TLS is the mechanism.
The secure web application is the product built on it. CEE uses TEE to seal competition
data. TEE is the mechanism. CEE is the product construct.

---

## The MagicBlock PER and How CEE Uses It

MagicBlock's Private Ephemeral Rollup (PER) is a TEE-secured execution environment
that operates as a Solana sidechain session. It can delegate Solana accounts from
mainnet into an ephemeral session, process transactions at sub-50ms speed inside the
TEE, and undelegate accounts back to mainnet with cryptographic proof of honest
execution.

Tyche uses the PER as the CEE execution context:

1. When a competition activates, `ActivateCompetition` calls the MagicBlock delegation
   program CPI to delegate `CompetitionState` and `AuctionState` into a PER session.

2. During the active phase, all `PlaceBid` transactions route to the PER RPC endpoint,
   not to mainnet. The PER processes them inside the Intel TDX enclave.

3. The sealed fields (`current_high_bid`, `current_winner`) update inside the enclave
   on every bid. They are never written to a location readable outside the TEE.

4. When the timer expires, `CloseCompetition` is called. The PER session finalizes.
   Accounts undelegate back to mainnet. Sealed fields are committed to mainnet state
   for the first time — now readable as part of the settlement record.

5. An Intel TDX attestation certificate is published on-chain alongside the settlement
   transaction. It proves that the execution inside the enclave was honest and that the
   final state was produced by the declared program running in an unmodified TEE.

---

## Which Accounts Enter the CEE Context

When `ActivateCompetition` is called, two accounts are delegated to the PER:

| Account            | Program        | Why Delegated                               |
|--------------------|----------------|---------------------------------------------|
| CompetitionState   | tyche-core     | Phase transitions and soft-close happen here|
| AuctionState       | tyche-auction  | Sealed bid tracking happens here            |

These accounts remain readable on mainnet throughout the active phase (Solana accounts
delegated to a PER are readable but not writable on mainnet). Only the PER can write
to them during the active phase.

Accounts that do NOT enter the CEE context:

| Account            | Why Not Delegated                                          |
|--------------------|------------------------------------------------------------|
| EscrowVault        | SOL custody stays on mainnet. Funds never enter the PER.  |
| ParticipantRecord  | Created on mainnet on first bid. Readable throughout.      |

This is a deliberate security boundary. Custody of funds and the competition execution
are separated at the program and account level. A compromise of the PER session cannot
directly affect fund release — the escrow program runs only on mainnet and only
responds to mainnet phase state.

---

## Which Fields Are Sealed and Why

### AuctionState.current_high_bid

The amount of the current highest bid.

Sealed because: if bidders can see the current highest bid, they calibrate their own
bid to the minimum needed to win rather than bidding their true valuation. This is the
whale surveillance attack. Sealing this field forces every bidder to bid based on their
own valuation, not on others' bids. The result is genuine price discovery.

### AuctionState.current_winner

The public key of the current highest bidder.

Sealed because: identity leakage enables targeted shill bidding and collusion. If a
seller can see which wallet is currently winning, an associate can place bids slightly
above that wallet's amount to drive up the price. Sealing the identity prevents this.

### ParticipantRecord.is_winner

Whether this participant is the current winner.

Sealed because: this is derivable from current_winner. It is sealed for consistency —
a participant querying their own record during the active phase cannot determine if they
are winning.

### What Remains Public

The following aggregate metrics are public during the active phase because they do not
enable strategic exploitation:

| Field                              | Why Public                                     |
|------------------------------------|------------------------------------------------|
| CompetitionState.phase             | Participants need to know if auction is active |
| CompetitionState.end_time          | Participants need to know when it closes       |
| CompetitionState.participant_count | Useful signal — indicates competition level   |
| CompetitionState.soft_close_count  | Shows how many extensions have occurred        |
| reserve_price                      | Bidders must know the minimum to participate   |

---

## What the Attestation Certificate Proves

At settlement, an Intel TDX attestation certificate is published on-chain alongside the
`SettleCompetition` transaction. The certificate proves:

1. **Code integrity.** The program that executed inside the TEE is the exact deployed
   program binary. It has not been modified. The hash of the executing code matches the
   expected measurement.

2. **Hardware authenticity.** The execution occurred inside a genuine Intel TDX trust
   domain, not inside a simulated or emulated environment.

3. **State commitment.** The winner and final price published in `SettleCompetition` are
   the values produced by that exact program running in that exact enclave. They have not
   been modified between TEE execution and on-chain commitment.

What the certificate does not prove: that any specific bid was placed, or what any
individual bid amount was. The attestation proves the outcome is honest. It does not
reconstruct the bid history. Individual bids remain sealed permanently — they are not
committed to any on-chain record at settlement.

---

## CEE in Future Verticals

The same CEE model applies to prediction markets and liquidity markets:

**Prediction markets:** Position entry (buying outcome shares) happens inside the PER.
The fields `current_yes_price` and individual position sizes are sealed during the
trading window. Aggregate statistics (total volume, current price) are public.

**Liquidity markets:** Swap orders and LP quotes are sealed inside the PER during each
batch window. The clearing algorithm runs in the TEE. Individual order sizes and LP
strategies are never exposed. The clearing price and aggregate volume are public.

The three-phase model — COMMIT, COMPETE, CLEAR — is identical in each case. Only the
rules of the COMPETE phase differ.
