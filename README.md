# Tyche

Tyche is a competitive price discovery protocol for Solana. Its internal execution
engine — CEE (Competitive Execution Engine) — runs sealed, privacy-preserving
competition for any asset class at real-time speed, settled by trustless on-chain code.

The first application surface is **Conceal**: private auctions for NFTs and in-game
items. Future surfaces include prediction markets and private liquidity markets. All
surfaces run on the same three on-chain programs.

## What CEE Means

Every market answers one question: what is this worth, right now, to the people who
want it most? Existing Solana infrastructure answers it poorly — every bid, position,
and quote is public the moment it is submitted. Sophisticated actors win by surveillance,
not by genuine valuation.

CEE is the engine that fixes this. It runs competition inside a MagicBlock Private
Ephemeral Rollup (PER) secured by Intel TDX hardware. Individual bids and positions are
sealed inside the TEE during the active phase — unreadable by any participant, validator,
or member of the Tyche team. The price discovered is the genuine market price.

Three phases, every competition:

```
COMMIT   participants declare intent (sealed)
COMPETE  engine processes inside TEE at sub-50ms
CLEAR    verifiable outcome, automatic settlement
```

## What Conceal Is

Conceal is the first application built on Tyche/CEE. It runs sealed English auctions
for two asset classes:

- NFT auctions ("Collector Drop") — for creators running private drops
- In-game item auctions ("Rust Undead Market") — for game economies

Both surfaces run on the same three on-chain programs. A game studio integrating CEE
writes their own consumer program and calls the same tyche-core and tyche-escrow via CPI.
Conceal is the reference implementation that proves the engine works.

## Stack

| Layer            | Technology                                    |
|------------------|-----------------------------------------------|
| On-chain         | Rust, Pinocchio, Solana SBF                   |
| IDL generation   | Shank                                         |
| Client codegen   | Codama                                        |
| TypeScript SDK   | @solana/kit                                   |
| Frontend         | Next.js                                       |
| Privacy layer    | MagicBlock Private Ephemeral Rollup, Intel TDX|

Anchor is not used. Every program is written with Pinocchio directly.

## Repository Structure

```
Tyche/
+-- programs/
|   +-- tyche-core/      competition state machine (CEE lifecycle)
|   +-- tyche-escrow/    SOL custody (per-bidder vault PDAs)
|   +-- tyche-auction/   auction semantics, PlaceBid inside CEE
|
+-- crates/
|   +-- tyche-common/    shared constants, macros, delegation CPI helper
|   +-- tyche-cpi/       CPI interface for external programs
|
+-- clients/
|   +-- generated/ts/    Codama-generated TypeScript clients (do not edit)
|   +-- sdk/             @tyche-protocol/sdk hand-written layer
|
+-- app/                 Conceal Next.js frontend
+-- tests/               Rust integration tests
+-- idl/                 Shank-generated IDL JSON files
+-- docs/                Architecture, security, and integration documentation
```

## Prerequisites

- Rust (stable)
- Solana CLI >= 2.1 with SBF toolchain (`solana-install`)
- `cargo-build-sbf` (`cargo install cargo-build-sbf`)
- `shank-cli` (`cargo install shank-cli`)
- Node.js >= 20
- `just` (`cargo install just`)
- MagicBlock local validator (see docs/CEE.md for setup)

## Build

```sh
# Full pipeline: generate IDLs, generate TypeScript clients, build SBF binaries
just all

# Individual steps
just idl        # regenerate IDLs from Shank annotations
just generate   # regenerate TypeScript clients from IDLs
just build      # build SBF binaries only
```

## Test

```sh
# Rust unit and integration tests (host target, no SBF required)
just test-rust

# TypeScript integration tests against a local validator
just test-ts
```

## Running a Local MagicBlock Validator

The delegation and ephemeral rollup functionality requires a local MagicBlock validator
running alongside the standard Solana test validator. See docs/CEE.md for the exact
setup commands.

## Documentation

| Document              | Contents                                              |
|-----------------------|-------------------------------------------------------|
| ARCHITECTURE.md       | Full CEE architecture, phase diagram, account layout  |
| docs/CEE.md           | What CEE is, sealed fields, attestation proof         |
| docs/SECURITY.md      | Threat model, invariants, security checklist          |
| docs/INTEGRATION.md   | How to integrate CEE as a game or NFT developer       |
| programs/*/README.md  | Per-program instructions, accounts, errors            |
| clients/sdk/README.md | TypeScript SDK usage and full auction flow example    |

## Protocol Vision

Conceal is the first vertical. Two more are designed and architecturally accounted for:

- **Prediction markets** — TEE-private positions, competitive commit-reveal oracle
  resolution, fair-launch batch auction for initial odds
- **Liquidity markets** — batch auction execution for token swaps, private LP quotes,
  MEV elimination, RFQ for institutional-size trades

All three verticals share the same CEE engine. The same three on-chain programs handle
the state machine, custody, and settlement for every surface.
