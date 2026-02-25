# @tyche-protocol/sdk

TypeScript SDK for the Tyche protocol. Provides a high-level client for creating and
managing sealed auctions via the CEE engine, including PER session lifecycle management,
soft-close crank operation, and settlement orchestration.

Built on top of Codama-generated `@solana/kit`-compatible clients. The generated layer
in `clients/generated/ts/` handles all instruction serialization and account
deserialization. This SDK layer handles business logic, routing, and session management.

---

## Installation

```sh
npm install @tyche-protocol/sdk @solana/kit
```

---

## Quick Start: Full Auction Flow

```typescript
import { initTycheClient } from '@tyche-protocol/sdk';
import { createKeyPairSigner, createSolanaRpc } from '@solana/kit';

// Initialize
const signer = await createKeyPairSigner(yourSecretKey);
const client = initTycheClient({
  rpc: createSolanaRpc('https://api.devnet.solana.com'),
  erRpcEndpoint: 'https://devnet.magicblock.app',
  wallet: signer,
});

// 1. Create the auction (Scheduled phase)
const competitionId = await client.auction.create({
  assetMint: address('YourNftMintHere'),
  reservePrice: 5_000_000_000n,     // 5 SOL
  duration: 3600,                    // 1 hour
  softCloseWindow: 120,              // 2-minute window
  softCloseExtension: 120,           // 2-minute extension per trigger
  maxSoftCloses: 10,
  minBidIncrement: 100_000_000n,    // 0.1 SOL
  assetType: 'nft',
});

// 2. Activate (Scheduled -> Active, delegates to PER, starts crank)
await client.session.startSession(competitionId);

// 3. Bid (routes to PER RPC automatically)
await client.auction.bid(competitionId, 7_000_000_000n);  // 7 SOL

// 4. Check status (sealed during active phase)
const status = await client.auction.getStatus(competitionId);
console.log(status.participantCount);   // 4
console.log(status.currentHighBid);     // null (sealed)
console.log(status.reserveMet);         // true

// 5. Finalize (after phase == 'settled')
// Settlement happens automatically via crank.
// Call finalize after detecting settled phase:
await client.auction.finalize(competitionId);
// -- asset transferred to winner
// -- winner vault released to seller
// -- all losing vaults refunded
```

---

## API Reference

### `initTycheClient(config: TycheConfig)`

Initializes the SDK and returns an object with `auction` and `session` clients.

```typescript
interface TycheConfig {
  rpc: Rpc<SolanaRpcApi>;
  erRpcEndpoint: string;   // MagicBlock PER RPC endpoint
  wallet: KeyPairSigner;
}
```

---

### `AuctionClient`

#### `create(config: AuctionConfig): Promise<Address>`

Creates a new auction. Sends `CreateCompetition` (tyche-core) and `CreateAuction`
(tyche-auction). Transfers the asset into program-owned escrow.

Returns the `CompetitionState` PDA address (`competitionId`).

```typescript
interface AuctionConfig {
  assetMint: Address;
  reservePrice: bigint;         // lamports
  duration: number;             // seconds
  softCloseWindow: number;      // seconds
  softCloseExtension: number;   // seconds
  maxSoftCloses: number;
  minBidIncrement: bigint;      // lamports
  assetType: 'nft' | 'game_item';
}
```

#### `bid(competitionId: Address, amount: bigint): Promise<void>`

Places a bid. Routes to the PER RPC endpoint during the active phase. The bid amount
is sealed inside the TEE — not readable on mainnet until settlement.

Throws `BidTooLow` if `amount <= currentHighBid + minBidIncrement`.
Throws `BidBelowReserve` if `amount < reservePrice`.
Throws `InvalidPhase` if the competition is not in the Active phase.

#### `getStatus(competitionId: Address): Promise<AuctionStatus>`

Returns the current public state of the auction. Never exposes sealed fields during
the active phase.

```typescript
interface AuctionStatus {
  phase: 'scheduled' | 'active' | 'closing' | 'settled' | 'cancelled';
  endTime: Date;
  participantCount: number;
  reserveMet: boolean;
  softCloseCount: number;
  // Sealed during active phase, revealed after settlement:
  currentHighBid: bigint | null;
  currentWinner: Address | null;
  // Set after settlement:
  attestationSignature: string | null;
}
```

#### `finalize(competitionId: Address): Promise<void>`

Completes settlement. Sends `FinalizeAuction`, `ReleaseWinner`, and `Refund` for all
losing vaults. Safe to call only when `status.phase == 'settled'`.

---

### `SessionManager`

#### `startSession(competitionId: Address): Promise<void>`

Activates the auction and delegates accounts to the MagicBlock PER. Called internally
by `AuctionClient` — you do not need to call this directly unless you are managing
activation manually.

Calls `ActivateCompetition` on mainnet, then starts `startCrank`.

#### `startCrank(competitionId: Address): void`

Starts the soft-close monitoring crank. Polls every 30 seconds. Calls
`ExtendCompetition` when a bid arrived within the soft-close window. Calls
`CloseCompetition` when `clock >= end_time`. Stops automatically when the phase
transitions to Closing.

You do not need to call this directly — `startSession` calls it.

#### `submitToER(tx: Transaction): Promise<void>`

Submits a transaction to the PER RPC endpoint. Used internally by `AuctionClient.bid`.
Exposed for advanced use cases where you need to submit custom transactions to the PER.

#### `finalizeSession(competitionId: Address): Promise<void>`

Waits for PER undelegation to complete, then calls `SettleCompetition`. Called
automatically by the crank after `CloseCompetition`. You do not need to call this
directly in normal usage.

Polls the `delegation_record` PDA until it no longer exists (proving undelegation),
then reads the final winner and amount from `AuctionState` and sends
`SettleCompetition`.

---

## Listening for Events

```typescript
import { subscribeToTycheEvents } from '@tyche-protocol/sdk';

const unsubscribe = await subscribeToTycheEvents(client.rpc, {
  competition: competitionId,
  onBidPlaced: (event) => {
    console.log('bid placed, participants:', event.participantCount);
    // event.participantCount is all that is exposed
    // no amount, no bidder identity
  },
  onExtended: (event) => {
    console.log('auction extended to', event.newEndTime);
    console.log('total extensions:', event.softCloseCount);
  },
  onSettled: (event) => {
    console.log('winner:', event.winner);
    console.log('final price:', event.finalAmount, 'lamports');
  },
});

// Stop listening:
unsubscribe();
```

---

## Error Handling

All SDK methods throw typed errors that correspond to on-chain program errors:

```typescript
import { TycheError } from '@tyche-protocol/sdk';

try {
  await client.auction.bid(competitionId, 1_000_000n);
} catch (e) {
  if (e instanceof TycheError) {
    switch (e.code) {
      case 'BidTooLow':
        // bid was not high enough
        break;
      case 'BidBelowReserve':
        // bid is below reserve price
        break;
      case 'InvalidPhase':
        // auction not in active phase
        break;
    }
  }
}
```

---

## Generated Client Layer

The files under `clients/generated/ts/` are generated by Codama from the Shank IDL
files. Do not edit them. Regenerate after any instruction or account struct change:

```sh
just idl       # regenerate IDLs from Shank annotations
just generate  # regenerate TypeScript clients from IDLs
```

The SDK in `clients/sdk/src/` imports from the generated layer for all serialization
and deserialization. No manual serialization exists in the SDK layer.
