# Tyche justfile — primary dev workflow interface.
# Install: cargo install just
# Usage:   just <recipe>

# Default: full pipeline (idl → generate → build)
default: all

# Full pipeline: generate IDLs, generate TS clients, build SBF binaries.
all: idl generate build

# ── IDL & Client Generation ────────────────────────────────────────────────────

# Generate IDLs from Shank annotations and fix them for Codama compatibility.
# Outputs to clients/idls/. Does NOT require cargo build-sbf.
idl:
    shank idl --crate-root programs/tyche-core    --out-dir clients/idls
    shank idl --crate-root programs/tyche-escrow  --out-dir clients/idls
    shank idl --crate-root programs/tyche-auction --out-dir clients/idls
    npx ts-node scripts/fix-idls.ts
    @echo "IDLs written to clients/idls/"

# Generate TypeScript clients from IDLs using Codama.
# Outputs to clients/js/src/generated/.
generate:
    npx ts-node scripts/codama.ts
    @echo "TypeScript clients written to clients/js/src/generated/"

# ── Build ──────────────────────────────────────────────────────────────────────

# Build all three SBF program binaries.
# Output: target/deploy/tyche_core.so, tyche_escrow.so, tyche_auction.so
build:
    cargo build-sbf

# ── Tests ──────────────────────────────────────────────────────────────────────

# Run Rust unit and integration tests using litesvm (host target).
# Requires programs to have been built first: just build
test-rust:
    cargo test -p tyche-tests

# Run TypeScript integration tests against a local Solana validator.
# Requires: solana-test-validator running in a separate terminal.
# See docs/CEE.md for MagicBlock validator setup.
test-ts:
    npx tsx tests/integration/full-flow.test.ts

# Run all Rust tests, then all TypeScript tests.
test: test-rust test-ts

# ── Deployment ─────────────────────────────────────────────────────────────────

# Deploy all three programs to the local validator.
# Requires: solana-test-validator running.
deploy-localnet:
    solana program deploy target/deploy/tyche_core.so
    solana program deploy target/deploy/tyche_escrow.so
    solana program deploy target/deploy/tyche_auction.so

# ── Housekeeping ───────────────────────────────────────────────────────────────

# Remove all build artifacts.
clean:
    cargo clean
    rm -rf dist/
