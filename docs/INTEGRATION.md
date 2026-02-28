# Tyche Integration Guide

This guide is for game developers and NFT platform builders who want to integrate
the Tyche CEE engine into their own product. It covers what the SDK exposes, how
PDAs are derived client-side, and a complete working example from auction creation
through settlement.

---

## What You Are Integrating

You are integrating the CEE engine — three on-chain programs that handle the full
lifecycle of a sealed, privacy-preserving auction:

- **tyche-core** — competition state machine (phase transitions, soft-close, settlement)
- **tyche-escrow** — SOL custody (per-bidder vault PDAs, release, refund)
- **tyche-auction** — auction semantics (asset escrow, sealed bid processing, asset transfer)
- **tyche-voter-weight-plugin** — Realms / SPL Governance integration

Your product calls these programs through the `@tyche-protocol/sdk`. You do not call
the programs directly — the SDK handles instruction building, PDA derivation, PER
session routing, and settlement orchestration.

The SDK tests in `tests/ts` provide the reference implementation. You are building your own UI or consumer program on top of the same engine.

---

## Installation

```sh
npm install @tyche-protocol/sdk @solana/kit
```

---

## Initialization

```typescript
import { initTycheClient } from '@tyche-protocol/sdk';
import { createSolanaRpc } from '@solana/kit';

const client = initTycheClient({
  rpc: createSolanaRpc('https://api.devnet.solana.com'),
  erRpcEndpoint: 'https://devnet.magicblock.app',  // MagicBlock PER endpoint
  wallet: yourKeyPairSigner,
});

// client.auction  -- AuctionClient
// client.session  -- SessionManager
```

---

## Creating an Auction

```typescript
import { address } from '@solana/kit';

const competitionId = await client.auction.create({
  assetMint: address('YourNftMintAddressHere'),
  reservePrice: 5_000_000_000n,        // 5 SOL in lamports
  duration: 3600,                       // 1 hour in seconds
  softCloseWindow: 120,                 // 2-minute soft-close window
  softCloseExtension: 120,             // extend by 2 minutes per trigger
  maxSoftCloses: 10,                   // cap at 10 extensions
  minBidIncrement: 100_000_000n,       // 0.1 SOL minimum raise
  assetType: 'nft',                    // 'nft' | 'game_item'
});

// competitionId is the CompetitionState PDA address
// the auction is now in Scheduled phase
// the NFT has been transferred to program-owned escrow
```

## Activating (Starting) the Auction

```typescript
// Transitions Scheduled -> Active
// Delegates CompetitionState and AuctionState to MagicBlock PER
// Starts the soft-close crank
await client.session.startSession(competitionId);

// After this call, bids must route through the PER RPC endpoint
// The SDK handles this routing automatically via client.auction.bid()
```

---

## Placing a Bid

```typescript
// Routes to PER RPC endpoint automatically during active phase
// Bid amount is sealed inside the TEE — not visible on mainnet
await client.auction.bid(competitionId, 7_000_000_000n);  // 7 SOL

// What happens on-chain (inside the PER):
// 1. PlaceBid validates amount > current_high_bid + min_bid_increment
// 2. PlaceBid validates amount >= reserve_price
// 3. AuctionState.current_high_bid updated (sealed)
// 4. AuctionState.current_winner updated (sealed)
// 5. ParticipantRecord created if first bid from this wallet
// 6. CompetitionState.participant_count incremented if new participant
// 7. EscrowVault.amount updated (SOL deposited on mainnet)
```

---

## Checking Auction Status

```typescript
const status = await client.auction.getStatus(competitionId);

// status shape during active phase:
// {
//   phase: 'active',
//   endTime: Date,
//   participantCount: 4,
//   reserveMet: true,       // true if any bid >= reserve_price exists
//   softCloseCount: 2,      // number of extensions so far
//   currentHighBid: null,   // sealed during active phase
//   currentWinner: null,    // sealed during active phase
// }

// status shape after settlement:
// {
//   phase: 'settled',
//   endTime: Date,
//   participantCount: 4,
//   reserveMet: true,
//   softCloseCount: 2,
//   currentHighBid: 9_000_000_000n,    // 9 SOL
//   currentWinner: '7xKp...3mNq',
//   attestationSignature: '...',       // Intel TDX proof
// }
```

---

## Settlement

Settlement happens automatically. The SessionManager crank monitors active auctions
and calls `CloseCompetition` when the timer expires. After PER undelegation completes,
it calls `SettleCompetition`. Your code only needs to call `finalize` to complete
asset transfer and trigger refunds:

```typescript
// Call after phase == 'settled'
// Transfers asset to winner, releases winner escrow to seller,
// refunds all losing vaults
await client.auction.finalize(competitionId);
```

---

## PDAs Your Code Needs to Derive

If you are building a custom UI or reading state directly (without using
`getStatus`), you need to derive these addresses client-side.

All PDA derivations use the program IDs published in `@tyche-protocol/sdk/constants`.

```typescript
import {
  TYCHE_CORE_PROGRAM_ID,
  TYCHE_ESCROW_PROGRAM_ID,
  TYCHE_AUCTION_PROGRAM_ID,
} from '@tyche-protocol/sdk/constants';
import { getProgramDerivedAddress } from '@solana/kit';

// CompetitionState PDA
// seeds: ["competition", authority_pubkey, competition_id_bytes]
const [competitionPda] = await getProgramDerivedAddress({
  programAddress: TYCHE_CORE_PROGRAM_ID,
  seeds: [
    Buffer.from('competition'),
    authority.toBytes(),
    competitionId.toBytes(),
  ],
});

// AuctionState PDA
// seeds: ["auction", competition_pubkey]
const [auctionPda] = await getProgramDerivedAddress({
  programAddress: TYCHE_AUCTION_PROGRAM_ID,
  seeds: [
    Buffer.from('auction'),
    competitionPda.toBytes(),
  ],
});

// EscrowVault PDA (per bidder)
// seeds: ["vault", competition_pubkey, depositor_pubkey]
const [vaultPda] = await getProgramDerivedAddress({
  programAddress: TYCHE_ESCROW_PROGRAM_ID,
  seeds: [
    Buffer.from('vault'),
    competitionPda.toBytes(),
    bidderPubkey.toBytes(),
  ],
});

// ParticipantRecord PDA (per bidder)
// seeds: ["participant", competition_pubkey, participant_pubkey]
const [participantPda] = await getProgramDerivedAddress({
  programAddress: TYCHE_CORE_PROGRAM_ID,
  seeds: [
    Buffer.from('participant'),
    competitionPda.toBytes(),
    bidderPubkey.toBytes(),
  ],
});
```

---

## Building Your Own CEE Consumer (Advanced)

If you want your game to have its own on-chain auction program with custom logic —
for example, a Dutch auction type, or a multi-item auction format — you can build a
program that consumes tyche-core and tyche-escrow directly via CPI, the same way
tyche-auction does.

Add `tyche-cpi` as a dependency:

```toml
[dependencies]
tyche-cpi = { version = "0.1", features = ["core", "escrow"] }
```

Call tyche-core to manage competition lifecycle:

```rust
use tyche_cpi::core::create_competition;

// In your CreateAuction processor:
create_competition(
    ctx.accounts.competition.key(),
    ctx.accounts.authority.key(),
    CreateCompetitionArgs {
        id: args.id,
        asset_type: AssetType::GameItem as u8,
        start_time: clock.unix_timestamp,
        duration: args.duration,
        soft_close_window: args.soft_close_window,
        soft_close_extension: args.soft_close_extension,
        max_soft_closes: args.max_soft_closes,
        reserve_price: args.reserve_price,
    },
)?;
```

Call tyche-escrow to handle custody:

```rust
use tyche_cpi::escrow::deposit;

// In your PlaceBid processor:
deposit(
    ctx.accounts.vault.key(),
    ctx.accounts.bidder.key(),
    ctx.accounts.competition.key(),
    DepositArgs { amount: args.amount },
)?;
```

The CEE phase guarantees (sealed execution, soft-close, settlement flow) come from
tyche-core automatically. You only need to implement your auction-specific logic.

---

## What the SDK Handles for You

| Concern                              | Handled by SDK                          |
|--------------------------------------|-----------------------------------------|
| PDA derivation                       | All PDAs derived internally             |
| Instruction serialization            | Codama-generated builders               |
| PER vs mainnet RPC routing           | SessionManager routes automatically     |
| Soft-close crank                     | SessionManager.startCrank()             |
| Undelegation detection               | SessionManager.finalizeSession()        |
| Settlement sequencing                | AuctionClient.finalize() orchestrates   |
| Refund iteration                     | SDK iterates all losing vaults          |

---

## Error Reference

Errors returned by the programs map to typed error codes in the SDK:

| Error                      | Program         | Meaning                                        |
|----------------------------|-----------------|------------------------------------------------|
| `InvalidPhase`             | tyche-core      | Instruction called in wrong phase              |
| `ArithmeticOverflow`       | tyche-core      | Amount calculation overflowed                  |
| `SoftCloseCapReached`      | tyche-core      | max_soft_closes already reached                |
| `NotUndelegated`           | tyche-core      | SettleCompetition called before undelegation   |
| `BidTooLow`                | tyche-auction   | Bid <= current_high_bid + min_bid_increment    |
| `BidBelowReserve`          | tyche-auction   | Bid < reserve_price                            |
| `AssetAlreadyTransferred`  | tyche-auction   | FinalizeAuction called twice                   |
| `VaultAlreadyReleased`     | tyche-escrow    | Release or refund called on released vault     |
| `NotWinner`                | tyche-escrow    | ReleaseWinner called for non-winner vault      |
| `InvalidDiscriminator`     | all             | Wrong account type passed                      |
| `InvalidOwner`             | all             | Account not owned by expected program          |
