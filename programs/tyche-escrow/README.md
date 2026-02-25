# tyche-escrow

The Tyche SOL custody program. This program owns one responsibility: holding bidder
SOL deposits safely and releasing them to the correct destination after settlement.

tyche-escrow is intentionally separated from `tyche-core` and `tyche-auction`. Custody
logic and competition logic have different security boundaries. A vulnerability in the
state machine must not be able to affect fund release, and auditors can reason about
each program in isolation. This separation is a deliberate architectural constraint,
not an optimization.

---

## What This Program Does

- Hold bidder SOL in per-bidder PDA vaults (`EscrowVault`)
- Accept top-up deposits when a bidder raises their own bid
- Release the winner's vault to the seller after settlement
- Refund all losing vaults after settlement
- Refund all vaults on cancellation

## What This Program Does Not Do

- Know anything about auction state, bid amounts, or winners directly — it reads
  `CompetitionState` from `tyche-core` to verify phase before any transfer
- Hold any assets (tokens or NFTs) — that is `tyche-auction`
- Manage phase transitions — that is `tyche-core`

---

## Instructions

### Deposit

Creates an `EscrowVault` PDA on the bidder's first deposit. On subsequent deposits
from the same bidder, tops up the existing vault. This allows a bidder to raise their
own bid incrementally without losing their prior deposit.

Accounts:

| Index | Constraint       | Account        | Description                                  |
|-------|------------------|----------------|----------------------------------------------|
| 0     | writable         | vault          | EscrowVault PDA. Created if not exists.      |
| 1     | writable, signer | depositor      | The bidder. Sole authorized withdrawer.      |
| 2     |                  | competition    | CompetitionState PDA. Verifies phase.        |
| 3     | writable, signer | payer          | Pays rent on first deposit (usually depositor)|
| 4     |                  | system_program |                                              |

Args: `amount: u64` (lamports)

Guards: `competition.phase == Active` (bids only accepted during active phase)

### ReleaseWinner

Transfers the winner's vault lamports to the seller. Called by `tyche-auction` via CPI
as part of `FinalizeAuction`, after the winner has received the asset.

Accounts:

| Index | Constraint | Account     | Description                                         |
|-------|------------|-------------|-----------------------------------------------------|
| 0     | writable   | vault       | The winner's EscrowVault PDA                        |
| 1     | writable   | seller      | Destination. Receives the winning bid lamports.     |
| 2     |            | competition | CompetitionState PDA. Verifies phase and winner.    |
| 3     | signer     | authority   | Must match competition.authority                    |

Guards: `competition.phase == Settled`, `vault.depositor == competition.winner`,
`vault.released == false`

Side effects: sets `vault.released = true`, closes vault account (lamports to authority)

### Refund

Returns a losing bidder's lamports. Also called for all vaults on cancellation.

Accounts:

| Index | Constraint | Account     | Description                                       |
|-------|------------|-------------|---------------------------------------------------|
| 0     | writable   | vault       | The losing bidder's EscrowVault PDA               |
| 1     | writable   | depositor   | Destination. Must match vault.depositor.          |
| 2     |            | competition | CompetitionState PDA. Verifies phase.             |
| 3     | signer     | authority   | Must match competition.authority                  |

Guards: `competition.phase == Settled OR Cancelled`, `vault.released == false`,
if Settled: `vault.depositor != competition.winner`

Side effects: sets `vault.released = true`, closes vault account (lamports to authority)

---

## Accounts

### EscrowVault

PDA seeds: `[b"vault", competition_pubkey, depositor_pubkey]`

One vault per `(competition, depositor)` pair. A bidder who raises their bid tops up
the same vault — they do not create multiple vaults.

| Field         | Type     | Description                                                   |
|---------------|----------|---------------------------------------------------------------|
| discriminator | [u8; 8]  | Account type tag. Verified on every read.                     |
| competition   | [u8; 32] | Parent CompetitionState pubkey                                |
| depositor     | [u8; 32] | Bidder pubkey. Sole authorized withdrawer.                    |
| amount        | u64      | Current lamport balance held in escrow                        |
| released      | bool     | True after release or refund. Prevents double-spend.          |
| bump          | u8       | Cached PDA bump seed                                          |
| _padding      | [u8; 6]  |                                                               |

The `released` flag is the primary double-spend protection. It is read before every
fund transfer and set to `true` before the function returns. Because Solana transactions
are atomic, setting the flag and transferring funds cannot be separated by any
interleaving transaction.

---

## Errors

| Error                | Description                                                  |
|----------------------|--------------------------------------------------------------|
| InvalidPhase         | Deposit called outside Active phase                          |
| VaultAlreadyReleased | Release or refund called on a vault where released == true   |
| NotWinner            | ReleaseWinner called for a vault that is not the winner      |
| IsWinner             | Refund called for the winning vault (when phase is Settled)  |
| DepositorMismatch    | Refund destination does not match vault.depositor            |
| ArithmeticOverflow   | Checked arithmetic failed on deposit amount                  |
| InvalidDiscriminator | Account discriminator does not match expected type           |
| InvalidOwner         | Account not owned by tyche-escrow                            |
