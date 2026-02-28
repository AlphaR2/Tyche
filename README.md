# Tyche

Tyche is a competitive price discovery protocol for Solana. Its internal execution
engine — CEE (Competitive Execution Engine) — runs sealed, privacy-preserving
competition for any asset class at real-time speed, settled by trustless on-chain code.

The first application surface is the **Tyche SDK**: a suite of TypeScript tools to run price discovery for any asset class. All surfaces run on the same three on-chain programs.

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

## SDK-Driven Price Discovery

Tyche is built to be integrated. Whether you are running a game economy or an NFT drop, you use the Tyche SDK to call the same `tyche-core` and `tyche-escrow` programs via CPI or direct client calls.

## Stack

| Layer            | Technology                                    |
|------------------|-----------------------------------------------|
| On-chain         | Rust, Pinocchio, Solana SBF                   |
| IDL generation   | Shank                                         |
| Client codegen   | Codama                                        |
| TypeScript SDK   | @solana/kit                                   |
| Realms Plugin    | Plugin for weighted votes based on auction    |
| Privacy layer    | MagicBlock Private Ephemeral Rollup, Intel TDX|

Anchor is not used. Every program is written with Pinocchio directly.

## Repository Structure

```
Tyche/
+-- programs/
|   +-- tyche-core/      competition state machine (CEE lifecycle)
|   +-- tyche-escrow/    SOL custody (per-bidder vault PDAs)
|   +-- tyche-auction/   auction semantics, PlaceBid inside CEE
|   +-- tyche-voter-weight-plugin/  Realms governance integration
|
+-- crates/
|   +-- tyche-common/    shared constants, macros, delegation CPI helper
|   +-- tyche-cpi/       CPI interface for external programs
|
+-- clients/
|   +-- generated/ts/    Codama-generated TypeScript clients (do not edit)
|   +-- sdk/             @tyche-protocol/sdk hand-written layer
|
+-- tests/               Rust integration tests
+-- clients/idls/         Shank-generated IDL JSON files (Codama-compatible)
+-- docs/                Architecture, security, and integration documentation
```

## Prerequisites

- Rust (stable)
- Solana CLI >= 2.1 with SBF toolchain (`solana-install`)
- `cargo-build-sbf` (`cargo install cargo-build-sbf`)
- `shank-cli` (`cargo install shank-cli`)
- Node.js >= 20
- `just` ([Case-sensitive command runner](https://github.com/casey/just)).
  - **Quick Install**: `sudo apt update && sudo apt install just` (Ubuntu 22.04+)
  - **From Cargo**: `cargo install just`
  - **From Snap**: `sudo snap install just`

## Build & Test (Recommended)

Tyche comes with an interactive CLI for the full development lifecycle.

```sh
# Ensure the script is executable
chmod +x scripts/tyche-cli.sh

# Start the interactive UI
./scripts/tyche-cli.sh
```

Within the UI, you can:
- **Build All**: Full IDL -> Client -> SBF pipeline.
- **Selective Build**: Run individual generators or SBF compilation.
- **Interactive Testing**: Run all tests or pick individual programs.
- **Environment Setup**: One-click funding and configuration.

---

## Build (Legacy/Automation)

You can still use `just` or manual commands for automation.
```sh
# Full pipeline: generate IDLs, generate TypeScript clients, build SBF binaries
just all

# Individual steps
just idl        # regenerate IDLs from Shank annotations
just generate   # regenerate TypeScript clients from IDLs
just build      # build SBF binaries only
```

### Manual Build (Without Just)
If you do not have `just` installed, run these commands in order:
```sh
# 1. Generate IDLs
shank idl --crate-root programs/tyche-core    --out-dir clients/idls
shank idl --crate-root programs/tyche-escrow  --out-dir clients/idls
shank idl --crate-root programs/tyche-auction --out-dir clients/idls
shank idl --crate-root programs/tyche-voter-weight-plugin --out-dir clients/idls
npx ts-node scripts/fix-idls.ts

# 2. Generate TypeScript Client
npx ts-node scripts/codama.ts
node scripts/restore-pkg-names.cjs

# 3. Build Rust Programs
cargo build-sbf
```

## Quick Start: CLI UI

The fastest way to verify the codebase is to use the interactive suite:

1. **Permissions**: `chmod +x scripts/tyche-cli.sh`
2. **Launch**: `./scripts/tyche-cli.sh`
3. **Setup**: Select `6` (Setup Test Env)
4. **Build**: Select `1` (Build All)
5. **Test**: Select `7` (Run All Tests)

TypeScript tests run against devnet and require funded keypairs.
**Must be run from a WSL/Linux terminal** — Windows npm cannot install
`@solana/kit` (native esbuild) over UNC paths.

### Environment Setup

Before running tests, configure your environment:

```sh
# 1. Copy the example env file
cp tests/ts/.env.example tests/ts/.env.test

# 2. Edit .env.test and set your values:
#    - RPC_URL: your Helius devnet URL (or leave as public devnet)
#    - AUTHORITY_KEYPAIR / BIDDER1_KEYPAIR / BIDDER2_KEYPAIR: funded keypair paths
#    - TREASURY_ADDRESS: the authority's address (or any devnet address)
#
# The setup script below generates keypairs and fills in the paths automatically.

# 3. One-time bootstrap — generates keypairs, airdrops devnet SOL, writes .env.test
just setup-tests
```

> **Tip**: Sign up at [helius.dev](https://helius.dev) for a free dedicated devnet
> RPC URL. The public endpoint rate-limits airdrop and test transactions.

```sh
# Run all TypeScript tests (programs + SDK)
just test-ts

# Granular test targets
just test-programs            # raw program instruction tests only
just test-sdk                 # SDK wrapper tests only
just test-watch               # watch mode (re-runs on file changes)
just test-file sdk/pdas       # run a specific test file
```

#### MagicBlock PER tests (optional)

Tests that require MagicBlock delegation (ActivateAuction, PlaceBid via PER)
are **skipped by default**.  To enable them, add to `tests/ts/.env.test`:

```sh
MAGICBLOCK_VALIDATOR=1
MAGICBLOCK_VALIDATOR_ADDRESS=LuzXEV3trGF4jQzpRzZaaTB9TqSwLkB7bpKQCQC7BAg
```

#### Test structure

```
tests/ts/
├── setup/
│   ├── env.ts             # RPC, signers, newCompetitionId()
│   ├── airdrop.ts         # retry-airdrop helper
│   ├── helpers.ts         # sendAndConfirm, getBlockhashForAccounts
│   └── global-setup.ts    # loads .env.test before any test file
├── programs/              # raw generated-client instruction tests
│   ├── tyche-core.test.ts
│   ├── tyche-escrow.test.ts
│   ├── tyche-auction.test.ts
│   └── integration.test.ts
└── sdk/                   # SDK wrapper tests (tyche-sdk)
    ├── pdas.test.ts        # PDA derivations (pure computation, no RPC)
    ├── accounts.test.ts    # fetchDecodedCompetition / fetchDecodedAuction
    ├── create-auction.test.ts
    ├── place-bid.test.ts
    └── full-flow.test.ts   # end-to-end lifecycle
```

## Documentation

| Document              | Contents                                              |
|-----------------------|-------------------------------------------------------|
| ARCHITECTURE.md       | Full CEE architecture, phase diagram, account layout  |
| docs/CEE.md           | What CEE is, sealed fields, attestation proof         |
| docs/SECURITY.md      | Threat model, invariants, security checklist          |
| docs/INTEGRATION.md   | How to integrate CEE as a game or NFT developer       |
| programs/*/README.md  | Per-program instructions, accounts, errors            |
| programs/tyche-voter-weight-plugin/README.md | Realms plugin specific documentation          |
| clients/sdk/README.md | TypeScript SDK usage and full auction flow example    |

## Governance (Realms Integration)

Tyche features a native governance integration with **Realms (SPL Governance)**. 

The `tyche-voter-weight-plugin` allows the DAO to recognize SOL deposits in Tyche as voting power. This enables a "Staked SOL" governance model where:
1. **Capital Alignment**: Users who have skin in the game (SOL deposited in auctions) have influence over protocol parameters.
2. **Just-in-Time Weight**: Voting weight is calculated from live escrow balances, requiring users to have an active commitment to vote.
3. **Low Overhead**: Built with Pinocchio for minimal compute costs during vote weight updates.

## Protocol Vision

Tyche is a generalized engine. The SDK supports:

- **Prediction markets** — TEE-private positions, competitive commit-reveal oracle
  resolution, fair-launch batch auction for initial odds
- **Liquidity markets** — batch auction execution for token swaps, private LP quotes,
  MEV elimination, RFQ for institutional-size trades

All three verticals share the same CEE engine. The same three on-chain programs handle
the state machine, custody, and settlement for every surface.
