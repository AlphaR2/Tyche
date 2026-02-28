# Tyche — TODO

All open work items, tracked in one place.
Items prefixed with a file path came from inline source comments.

---

## IDL & Client Generation

- [ ] **Replace Shank with Codama-native IDL generation**
  Currently using `shank idl` + `scripts/fix-idls.ts` as a post-processor because Shank
  emits the wrong discriminant format (`u8` scalar) and cannot resolve the `Address` type.
  Migrate to a Codama-first approach (e.g. `@codama/renderers-rust` or a custom visitor)
  so the IDLs are generated correctly in one step with no fix script needed.

- [ ] **Write `scripts/codama.ts` and install `@codama` packages**
  The client generation step (Step 7 of the IDL fix plan) is not yet implemented.
  Create `scripts/codama.ts`, install `@codama/nodes-from-anchor`,
  `@codama/renderers-js`, `@codama/visitors-core`, and wire into `npm run generate`.
  Output goes to `clients/tyche-core/src`, `clients/tyche-escrow/src`, `clients/tyche-auction/src`.

- [ ] **Add `@codama` packages to `package.json` devDependencies**
  Required before `scripts/codama.ts` can run. Packages:
  `@codama/nodes-from-anchor`, `@codama/renderers-js`, `@codama/visitors-core`.

- [ ] **Test the full `npm run generate` pipeline end-to-end**
  Confirm: `shank idl` → `fix-idls.ts` → `codama.ts` produces clean TypeScript clients
  with correct discriminators (8-byte arrays), no `Address defined` refs, and
  `fetchProtocolConfig` available from the generated core client.

---

## Governance / ProtocolConfig
> Source: `programs/tyche-core/src/state/protocol_config.rs`

- [ ] Add `pause_flags: u16` to `ProtocolConfig` to gate individual instruction families
  (e.g. pause deposits, pause new competitions) without a full program upgrade.

- [ ] Add `pending_authority: Address` for two-step authority transfer.
  Current single-step transfer lets a typo permanently brick the config.

- [ ] Add timelock fields for sensitive parameter changes so on-chain
  governance changes cannot take effect immediately.

---

## UpdateProtocolConfig
> Source: `programs/tyche-core/src/instruction_args/update_protocol_config.rs`

- [ ] Enforce 48-hour timelock on `new_fee_basis_points` changes.

- [ ] Enforce 48-hour timelock on `new_min_reserve_price` changes.

- [ ] Emit a change-log event for off-chain indexers when protocol config is updated.

---

## UpdateCrankAuthority
> Source: `programs/tyche-core/src/instruction_args/update_crank_authority.rs`

- [ ] Require `new_crank_authority` to co-sign the rotation instruction.
  Prevents rotating to a key you do not control, which would permanently lock
  all crank-gated instructions.

---

## Security

- [ ] Audit all `set_lamports` call sites — confirm total lamports out equals
  total lamports in for every instruction that moves funds.
  Priority: `Release`, `Refund`, `Deposit`.

- [ ] Verify `EscrowVault` PDA seeds produce a unique address per
  `(competition, depositor)` pair and cannot collide with other PDAs.

- [ ] Add signer check that `crank == ProtocolConfig::crank_authority`
  consistently across all three programs (core `SettleCompetition`,
  escrow `Release`, auction `FinalizeAuction`).

---

## Testing

- [ ] Write integration tests for the full auction happy path:
  `CreateAuction` → `ActivateAuction` → `PlaceBid` → `FinalizeAuction`
  → `SettleCompetition` (via CPI) → `Release`.

- [ ] Write a test for the `Refund` path (non-winner after `Settled`).

- [ ] Write a test for the `Refund` path after `Cancelled`.

- [ ] Write negative tests:
  - Non-crank calls `Release` / `FinalizeAuction`
  - Non-winner calls `Release`
  - Winner calls `Refund`
  - `SettleCompetition` called with wrong delegation_record (non-zero lamports)
  - `ActivateCompetition` called before `min_duration_secs` elapses

- [ ] Verify arithmetic overflow guards in `Release` fee calculation
  with `fee_basis_points = 10_000` and `vault.amount = u64::MAX`.

---

## Program Architecture

- [ ] Implement `pause_flags` checks in each processor once the field is
  added to `ProtocolConfig` (blocked on governance TODO above).

- [ ] Investigate replacing per-program `target/` directories with a
  single workspace-level build to reduce disk usage and CI time.

- [ ] Decide whether `Cargo.lock` should be committed — it is currently
  committed; confirm this is intentional for reproducible SBF builds.

---

## Infrastructure / DX

- [ ] Add a `Makefile` or `justfile` with common dev commands:
  `make build`, `make test`, `make generate`, `make deploy-localnet`.

- [ ] Set up CI (GitHub Actions) with:
  - `cargo check` on every PR
  - `cargo test` on every PR
  - `npm run generate` dry-run to catch IDL drift

- [ ] Pin `shank-cli` version in CI so IDL output is reproducible.
