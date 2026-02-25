# tyche-auction

The Tyche auction consumer. This is the first program built on the CEE engine. It
owns the auction-specific concerns: holding the asset in escrow, processing bids
inside the CEE, enforcing English auction rules, and transferring the asset to the
winner at settlement.

tyche-auction calls `tyche-core` and `tyche-escrow` via CPI. It does not manage
phases directly — it reads `CompetitionState` to verify phase guards and delegates
lifecycle management entirely to `tyche-core`.

A game studio building a custom auction type (Dutch auction, multi-item, sealed-reveal)
would write their own program like this one, using the same `tyche-core` and
`tyche-escrow` through `tyche-cpi`.

---

## What This Program Does

- Initialize `AuctionState` and transfer the asset from the seller to a program-owned
  escrow token account
- Process bids inside the CEE (PlaceBid runs on the MagicBlock PER)
- Enforce English auction rules: minimum bid increment, reserve price
- Transfer the asset to the winner after settlement
- Coordinate with `tyche-escrow` to release the winner's payment and refund losers

## What This Program Does Not Do

- Manage phase transitions — that is `tyche-core`
- Hold SOL — that is `tyche-escrow`
- Handle delegation to the PER directly — `tyche-core` handles the delegation CPI

---

## The PlaceBid Hot Path

`PlaceBid` is the most performance-sensitive instruction in the protocol. It runs on
every bid during the active CEE phase, inside the MagicBlock PER, at sub-50ms.

The instruction does the following in order:

1. Verify `competition.phase == Active`
2. Verify `args.amount >= auction.current_high_bid + auction.min_bid_increment`
3. Verify `args.amount >= competition.reserve_price`
4. Update `auction.current_high_bid` (sealed — only visible inside the TEE)
5. Update `auction.current_winner` (sealed — only visible inside the TEE)
6. Create `ParticipantRecord` if this is the bidder's first bid (on mainnet)
7. Increment `competition.participant_count` if new participant
8. CPI to `tyche-escrow::Deposit` to lock the bid amount in escrow
9. Emit `TycheBidPlaced` (participant count only — no amounts, no identities)

The `PlaceBid` instruction emits no log output that includes bid amounts or bidder
identity. This is a hard protocol guarantee — information leakage from log output
would defeat the sealed-bid privacy model.

---

## Instructions

### CreateAuction

Initializes an `AuctionState` PDA and transfers the asset from the seller's token
account to a program-owned associated token account (the asset escrow).

Accounts:

| Index | Constraint       | Account        | Description                                 |
|-------|------------------|----------------|---------------------------------------------|
| 0     | writable         | auction        | AuctionState PDA, created here              |
| 1     |                  | competition    | CompetitionState PDA (from tyche-core)      |
| 2     | writable         | asset_escrow   | Program-owned ATA for the asset, created    |
| 3     |                  | asset_mint     | The NFT or token mint                       |
| 4     | writable         | seller_ata     | Seller's current token account for asset    |
| 5     | writable, signer | seller         | Asset owner. Stored on AuctionState.        |
| 6     | writable, signer | payer          | Pays rent for AuctionState and asset_escrow |
| 7     |                  | token_program  |                                             |
| 8     |                  | system_program |                                             |

Args: `min_bid_increment: u64`

Guards: `competition.phase == Scheduled`, seller != competition.authority (prevents
seller from also being a bidder)

### PlaceBid

The hot path. Runs inside the MagicBlock PER during the active CEE phase.

Accounts:

| Index | Constraint       | Account            | Description                               |
|-------|------------------|--------------------|-------------------------------------------|
| 0     | writable         | auction            | AuctionState PDA                          |
| 1     | writable         | competition        | CompetitionState PDA                      |
| 2     | writable         | participant_record | ParticipantRecord PDA, created on first bid|
| 3     | writable         | vault              | EscrowVault PDA (tyche-escrow)            |
| 4     | writable, signer | bidder             | The bidding wallet                        |
| 5     | writable, signer | payer              | Pays rent for ParticipantRecord if new    |
| 6     |                  | tyche_escrow       | tyche-escrow program ID (for CPI)         |
| 7     |                  | system_program     |                                           |

Args: `amount: u64` (lamports)

Guards: `competition.phase == Active`,
`amount > auction.current_high_bid + auction.min_bid_increment`,
`amount >= competition.reserve_price`

### FinalizeAuction

Transfers the asset from program-owned escrow to the winner. Then releases the winner's
vault to the seller and refunds all losing vaults via CPI to `tyche-escrow`.

Accounts:

| Index | Constraint | Account         | Description                                     |
|-------|------------|-----------------|-------------------------------------------------|
| 0     | writable   | auction         | AuctionState PDA                                |
| 1     |            | competition     | CompetitionState PDA. Provides winner address.  |
| 2     | writable   | winner_ata      | Winner's token account. Receives the asset.     |
| 3     | writable   | seller          | Receives winning bid lamports from escrow.      |
| 4     | writable   | asset_escrow    | Program-owned ATA holding the asset             |
| 5     |            | asset_mint      | The NFT or token mint                           |
| 6     |            | token_program   |                                                 |
| 7     |            | tyche_escrow    | tyche-escrow program ID (for CPI)               |

Guards: `competition.phase == Settled`, `auction.asset_transferred == false`

Side effects: sets `auction.asset_transferred = true`

---

## Accounts

### AuctionState

PDA seeds: `[b"auction", competition_pubkey]`

| Field             | Type      | Description                                              |
|-------------------|-----------|----------------------------------------------------------|
| discriminator     | [u8; 8]   | Account type tag. Verified on every read.                |
| competition       | [u8; 32]  | Parent CompetitionState pubkey                           |
| asset_mint        | [u8; 32]  | NFT or token mint address                                |
| asset_escrow      | [u8; 32]  | Program-owned token account holding the asset            |
| seller            | [u8; 32]  | Original asset owner. Receives winning bid amount.       |
| min_bid_increment | u64       | Lamports above current_high_bid required for valid bid   |
| current_high_bid  | u64       | SEALED inside CEE/PER during active phase                |
| current_winner    | [u8; 33]  | Option<Pubkey>: byte[0] presence (0=None, 1=Some)        |
|                   |           | SEALED inside CEE/PER during active phase                |
| asset_transferred | bool      | True after asset transferred to winner                   |
| bump              | u8        | Cached PDA bump seed                                     |
| _padding          | [u8; 5]   |                                                          |

---

## Events

| Event                    | Fields                                   | Notes                   |
|--------------------------|------------------------------------------|-------------------------|
| TycheAuctionCreated      | competition, seller, asset_mint          |                         |
| TycheBidPlaced           | competition, participant_count           | No amounts. No identity.|
| TycheAuctionFinalized    | competition, winner                      |                         |

`TycheBidPlaced` intentionally omits bid amounts and bidder identity. This is a
protocol guarantee, not a convenience omission. Any addition of amount or identity
fields to this event would break the privacy model.

---

## Errors

| Error                  | Description                                                |
|------------------------|------------------------------------------------------------|
| InvalidPhase           | Instruction called in wrong phase                          |
| BidTooLow              | Bid <= current_high_bid + min_bid_increment                |
| BidBelowReserve        | Bid < competition.reserve_price                            |
| AssetAlreadyTransferred| FinalizeAuction called when asset_transferred == true      |
| SellerIsBidder         | CreateAuction called with seller == competition authority  |
| ArithmeticOverflow     | Checked arithmetic failed on user-supplied value           |
| InvalidDiscriminator   | Account discriminator does not match expected type         |
| InvalidOwner           | Account not owned by expected program                      |
