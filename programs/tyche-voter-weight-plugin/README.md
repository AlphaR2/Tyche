# tyche-voter-weight-plugin

An SPL Governance voter-weight plugin that derives voting power from `tyche-escrow` deposits.

This program allows a DAO (Realm) to use SOL deposits in the Tyche protocol as governance voting weight. This enables a "Staked SOL" governance model where user influence is proportional to their economic commitment to the protocol.

---

## What This Program Does

- **Map SOL to Votes**: Translates `EscrowVault.amount` (lamports) into `VoterWeightRecord.voter_weight`.
- **Just-in-Time Weight**: Forces users to update their weight and vote in the same transaction to prevent stale voting.
- **Max Voter Weight**: Calculates the total possible votes based on the total SOL deposited in a protocol competition.
- **Plugin Chaining**: Supports combining voting weight from this plugin with other plugins (e.g., token-based plugins).

## Relationship to SPL Governance

This program implements the [SPL Governance Add-in API](https://github.com/solana-labs/solana-program-library/tree/master/governance/addin-api). It produces:
1.  **`VoterWeightRecord`**: Individual voting power.
2.  **`MaxVoterWeightRecord`**: Total voting power in the system.

---

## Technical Architecture

### Low-Level Efficiency
Built using [Pinocchio](https://github.com/n0fua/pinocchio), a lightweight, no-allocator Solana framework. It uses zero-copy reading to verify `tyche-escrow` accounts without requiring a crate dependency, keeping the binary size minimal and compute costs extremely low.

### Dependency Management (Zeroize Fix)
The program is decoupled from the heavy `solana-program` crate to avoid versioning conflicts (specifically with the `zeroize` crate). By implementing the SPL Governance account structures locally using `Pinocchio` types, it maintains compatibility with Agave/Solana v2.0+ SDKs while fulfilling the interface requirements of SPL Governance.

---

## Instructions

### CreateRegistrar
One-time setup for a Realm/Mint pair. Defines the competition scope and the trusted escrow program.
- **Seeds**: `[realm, b"registrar", governing_token_mint]`

### CreateVoterWeightRecord
Initializes the metadata account for a voter.
- **Seeds**: `[b"voter-weight-record", realm, governing_token_mint, voter]`

### UpdateVoterWeightRecord
The primary interaction. Reads the user's `EscrowVault` from `tyche-escrow` and writes its balance to the `VoterWeightRecord`.
- **Logic**: Verifies the vault is owned by the `tyche-escrow` program and belongs to the correct competition.

### UpdateMaxVoterWeightRecord
Updates the `MaxVoterWeightRecord` with the total lamports deposited in the competition.

---

## Verification & Testing

To test the plugin on devnet:
1. **Build**: `cargo build-sbf`
2. **Deploy**: `solana program deploy target/deploy/tyche_voter_weight_plugin.so`
3. **Integration Tests**:
   ```bash
   cd tests/ts
   npx vitest run programs/tyche-voter-weight-plugin.test.ts
   ```
