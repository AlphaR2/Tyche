# tyche-core

The Tyche competition state machine. This program owns one responsibility: managing
the lifecycle of a CEE competition from creation through settlement.

tyche-core has no opinion about what is being auctioned, who the participants are,
or how bids are structured. It manages the phase state machine, the soft-close
extension logic, and the delegation into and out of the MagicBlock Private Ephemeral
Rollup. All asset-specific and bid-specific logic lives in `tyche-auction`.

---

## What This Program Does

- Initialize `CompetitionState` accounts in the Scheduled phase
- Delegate `CompetitionState` into the MagicBlock PER on activation
- Run soft-close extension logic via the `ExtendCompetition` instruction
- Transition to Closing when the timer expires
- Settle after PER undelegation completes
- Cancel competitions that received zero bids

## What This Program Does Not Do

- Hold any assets (tokens or NFTs) — that is `tyche-auction`
- Hold any SOL — that is `tyche-escrow`
- Process or validate bids — that is `tyche-auction`
- Release or refund funds — that is `tyche-escrow`

---

## Phase State Machine

```
Scheduled -> Active -> Settling -> Settled
                    -> Cancelled (participant_count == 0 only)
```

| Phase     | Value | Entry Instruction      | Exit Condition                      |
|-----------|-------|------------------------|-------------------------------------|
| Scheduled | 0     | CreateCompetition      | ActivateCompetition called          |
| Active    | 1     | ActivateCompetition    | CloseCompetition (timer expired)    |
| Settling  | 2     | CloseCompetition       | SettleCompetition (undelegated)     |
| Settled   | 3     | SettleCompetition      | Terminal                            |
| Cancelled | 4     | CancelCompetition      | Terminal                            |

---

## Instructions

### CreateCompetition

Initializes a `CompetitionState` PDA in the Scheduled phase.

Accounts:

| Index | Constraint       | Account        | Description                              |
|-------|------------------|----------------|------------------------------------------|
| 0     | writable         | competition    | CompetitionState PDA, created here       |
| 1     | signer           | authority      | Competition creator. Stored on account.  |
| 2     | writable, signer | payer          | Pays rent for CompetitionState           |
| 3     |                  | system_program |                                          |

Args: `id`, `asset_type`, `start_time`, `duration`, `soft_close_window`,
`soft_close_extension`, `max_soft_closes`, `reserve_price`

### ActivateCompetition

Transitions Scheduled -> Active. Delegates `CompetitionState` into the MagicBlock PER
via the delegation program CPI.

Guards: `phase == Scheduled`, `clock.unix_timestamp >= start_time`

Accounts:

| Index | Constraint       | Account             | Description                                    |
|-------|------------------|---------------------|------------------------------------------------|
| 0     | writable         | competition         | CompetitionState PDA                           |
| 1     | signer           | authority           | Must match competition.authority               |
| 2     | writable, signer | payer               | Pays rent for permission account creation      |
| 3     | writable         | permission          | ACL permission PDA (MagicBlock permission program) |
| 4     | writable         | delegation_buffer   | Delegation program buffer PDA                  |
| 5     | writable         | delegation_record   | Delegation program record PDA                  |
| 6     | writable         | delegation_metadata | Delegation program metadata PDA                |
| 7     |                  | delegation_program  | DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh  |
| 8     |                  | permission_program  | MagicBlock ACL permission program              |
| 9     |                  | system_program      |                                                |
| 10    |                  | validator           | TEE validator pubkey                           |

Args: `commit_frequency_ms`

### ExtendCompetition

Extends `end_time` by `soft_close_extension` seconds. Called by the SessionManager
crank when a bid arrives within the soft-close window.

Guards: `phase == Active`, `soft_close_count < max_soft_closes`,
`(end_time - clock.unix_timestamp) < soft_close_window`, caller == stored crank authority

Accounts:

| Index | Constraint | Account          | Description                           |
|-------|------------|------------------|---------------------------------------|
| 0     | writable   | competition      | CompetitionState PDA                  |
| 1     | signer     | crank            | Must match TYCHE_CRANK_PUBKEY         |
| 2     |            | magic_context    | MagicBlock context account            |
| 3     |            | magic_program    | MagicBlock program                    |

### CloseCompetition

Transitions Active -> Settling. Called by the SessionManager crank when the timer expires.

Guards: `phase == Active`, `clock.unix_timestamp >= end_time`

Accounts:

| Index | Constraint | Account          | Description                                           |
|-------|------------|------------------|-------------------------------------------------------|
| 0     | writable   | competition      | CompetitionState PDA                                  |
| 1     | signer     | crank            | Must match TYCHE_CRANK_PUBKEY                         |
| 2     | writable   | permission       | ACL permission PDA — undelegated alongside competition |
| 3     |            | magic_context    | MagicBlock context account                            |
| 4     |            | magic_program    | MagicBlock program                                    |

### SettleCompetition

Transitions Closing -> Settled. Called by the SessionManager after PER undelegation
completes. Accepts the final winner and price from the now-readable AuctionState.

Guards: `phase == Settling`, `delegation_record` PDA for `competition` does not exist

Accounts:

| Index | Constraint | Account           | Description                          |
|-------|------------|-------------------|--------------------------------------|
| 0     | writable   | competition       | CompetitionState PDA                 |
| 1     | signer     | authority         | Must match competition.authority     |
| 2     |            | delegation_record | Must not exist (proves undelegation) |

Args: `settlement_ref: [u8; 32]`

### CancelCompetition

Transitions to Cancelled. Only valid with zero participants.

Guards: `phase == Scheduled` OR (`phase == Active` AND `participant_count == 0`)

Accounts:

| Index | Constraint | Account       | Description                                                        |
|-------|------------|---------------|--------------------------------------------------------------------|
| 0     | writable   | competition   | CompetitionState PDA                                               |
| 1     | signer     | authority     | Must match competition.authority                                   |
| 2     | writable   | permission    | ACL permission PDA — undelegated on Active path, unused on Scheduled |
| 3     |            | magic_context | MagicBlock context account — unused on Scheduled path              |
| 4     |            | magic_program | MagicBlock program — unused on Scheduled path                      |

---

## Accounts

### CompetitionState

PDA seeds: `[b"competition", authority_pubkey, id_bytes]`

| Field                | Type     | Description                                             |
|----------------------|----------|---------------------------------------------------------|
| discriminator        | [u8; 8]  | Account type tag. Verified on every read.               |
| id                   | [u8; 32] | Unique identifier chosen by creator                     |
| authority            | [u8; 32] | Creator pubkey. Also serves as crank authority.         |
| asset_type           | u8       | 0 = NFT, 1 = in-game item                              |
| phase                | u8       | Current phase (see phase table above)                   |
| _padding             | [u8; 6]  |                                                         |
| start_time           | i64      | Earliest unix timestamp at which activation is allowed  |
| end_time             | i64      | Written 0 at creation. Set to clock + duration_secs at activation. Extended by soft-close. |
| soft_close_window    | i64      | Seconds before end_time that arm the soft-close         |
| soft_close_extension | i64      | Seconds added to end_time per soft-close trigger        |
| soft_close_count     | u8       | Number of extensions applied so far                     |
| max_soft_closes      | u8       | Hard cap on total extensions                            |
| _padding             | [u8; 6]  |                                                         |
| reserve_price        | u64      | Minimum winning bid in lamports                         |
| participant_count    | u32      | Unique bidders (increments on first bid per address)    |
| bump                 | u8       | Cached PDA bump seed                                    |
| _padding             | [u8; 3]  |                                                         |
| winner               | [u8; 32] | Zero-initialized. Written by SettleCompetition.         |
| final_amount         | u64      | Zero-initialized. Written by SettleCompetition.         |
| duration_secs        | i64      | Stored at creation. Used by ActivateCompetition to compute end_time. |

### ParticipantRecord

PDA seeds: `[b"participant", competition_pubkey, participant_pubkey]`

Created on the first bid from each address. Tracks per-bidder state.

| Field         | Type     | Description                                             |
|---------------|----------|---------------------------------------------------------|
| discriminator | [u8; 8]  | Account type tag                                        |
| competition   | [u8; 32] | Parent CompetitionState pubkey                          |
| participant   | [u8; 32] | Bidder pubkey                                           |
| is_winner     | bool     | SEALED during active phase. Set at settlement.          |
| last_action   | i64      | Unix timestamp of most recent bid                       |
| bump          | u8       | Cached PDA bump seed                                    |
| _padding      | [u8; 6]  |                                                         |

---

## Errors

| Error                 | Description                                              |
|-----------------------|----------------------------------------------------------|
| InvalidPhase          | Instruction called in wrong phase                        |
| NotAuthority          | Caller does not match stored authority                   |
| AuctionNotStarted     | Activation attempted before start_time                   |
| AuctionNotExpired     | CloseCompetition called before end_time                  |
| SoftCloseCapReached   | ExtendCompetition called at max_soft_closes              |
| SoftCloseNotArmed     | ExtendCompetition called outside soft-close window       |
| NotUndelegated        | SettleCompetition called before PER undelegation         |
| HasParticipants       | CancelCompetition called with participant_count > 0      |
| ArithmeticOverflow    | Checked arithmetic failed on user-supplied value         |
| InvalidDiscriminator  | Account discriminator does not match expected type       |
