# tyche-common

Shared library crate for the Tyche protocol. Contains constants, validation macros,
PDA derivation utilities, and the hand-written MagicBlock delegation CPI helper.

This crate has no entrypoint and is never deployed directly. It is a compile-time
dependency of all three program crates: `tyche-core`, `tyche-escrow`, and
`tyche-auction`.

---

## What This Crate Provides

### Constants (`src/constants.rs`)

Protocol-wide constants used across all three programs:

| Constant                  | Value / Description                                            |
|---------------------------|----------------------------------------------------------------|
| `DLP_ID`                  | MagicBlock delegation program address                          |
| `SYSTEM_PROGRAM_ID`       | Solana system program address                                  |
| `COMPETITION_SEED`        | PDA seed prefix for CompetitionState (`b"competition"`)        |
| `PARTICIPANT_SEED`        | PDA seed prefix for ParticipantRecord (`b"participant"`)       |
| `VAULT_SEED`              | PDA seed prefix for EscrowVault (`b"vault"`)                   |
| `AUCTION_SEED`            | PDA seed prefix for AuctionState (`b"auction"`)                |
| `DELEGATION_SEED`         | MagicBlock delegation program seed (`b"delegation"`)           |
| `DELEGATION_METADATA_SEED`| MagicBlock delegation metadata seed (`b"delegation-metadata"`) |

Centralizing these prevents silent mismatches between programs. If a seed changes,
it changes in one place and all three programs pick it up on recompile.

### Validation Macros (`src/macros.rs`)

Standard validation primitives used in all three programs:

| Macro               | Description                                                     |
|---------------------|-----------------------------------------------------------------|
| `require!`          | Assert a boolean condition, return error if false               |
| `require_signer!`   | Assert account.is_signer(), return MissingRequiredSignature     |
| `require_writable!` | Assert account.is_writable(), return InvalidAccountData         |
| `require_eq_keys!`  | Assert two [u8; 32] keys are equal, return given error          |
| `require_owned_by!` | Assert account.owner() matches program_id, return IllegalOwner  |

All macros log the failed condition using `pinocchio_log::log!` before returning.
This produces debuggable output in the program log without using heap allocation.

Using macros rather than inline conditions provides a single auditable location for
validation behavior. Every guard in every processor uses these macros — the pattern
is uniform across all three programs.

### PDA Utilities (`src/pda_utils.rs`)

Shared PDA derivation functions for PDAs that span program boundaries. Programs that
need to verify a PDA owned by another program use these functions to recompute the
expected address from seeds.

### MagicBlock Delegation CPI (`src/cpi/delegation.rs`)

Hand-written CPI helpers for the MagicBlock delegation program.

**Why hand-written:** The MagicBlock delegation program uses `solana-program` types
in its published instruction builders. Pinocchio uses its own `Instruction` type.
These are incompatible at the type level — you cannot directly use the delegation
program's builders in a Pinocchio crate without pulling in `solana-program`, which
would add significant binary size and CU overhead.

The hand-written helper constructs delegation and undelegation instructions from raw
bytes using Pinocchio's `Instruction` type, matching the delegation program's
discriminator and argument format exactly (discriminator 0 = Delegate, discriminator
3 = Undelegate, borsh-serialized args, all little-endian u64).

This helper is written once here and imported by all three programs. It does not need
to change unless the MagicBlock delegation program changes its public interface.

**Functions:**

- `cpi_delegate(...)` — delegates a PDA account into a MagicBlock PER session
- `cpi_undelegate(...)` — undelegates a PDA account back from the PER to mainnet

---

## Which Programs Use What

| Module             | Used by                                          |
|--------------------|--------------------------------------------------|
| constants.rs       | tyche-core, tyche-escrow, tyche-auction          |
| macros.rs          | tyche-core, tyche-escrow, tyche-auction          |
| pda_utils.rs       | tyche-core, tyche-escrow, tyche-auction          |
| cpi/delegation.rs  | tyche-core (ActivateCompetition calls Delegate)  |

---

## Adding to a Program

`tyche-common` is declared as a workspace dependency and referenced in each program's
`Cargo.toml`:

```toml
[dependencies]
tyche-common = { workspace = true }
```

Macros are imported via the crate root:

```rust
use tyche_common::{require, require_signer, require_eq_keys};
```

Constants are imported directly:

```rust
use tyche_common::constants::{COMPETITION_SEED, VAULT_SEED, DLP_ID};
```

The delegation CPI helper is imported from its submodule:

```rust
use tyche_common::cpi::delegation::cpi_delegate;
```
