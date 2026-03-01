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
    shank idl --crate-root programs/tyche-voter-weight-plugin --out-dir clients/idls
    npx ts-node scripts/fix-idls.ts
    @echo "IDLs written to clients/idls/"

# Generate TypeScript clients from IDLs using Codama, then restore workspace
# package names (Codama resets them to "js-client" on every run).
# Outputs to clients/js/src/generated/.
generate:
    npx ts-node scripts/codama.ts
    node scripts/patch-discriminators.cjs
    node scripts/restore-pkg-names.cjs
    @echo "TypeScript clients written to clients/js/src/generated/"

# ── SDK (tyche-sdk npm package) ────────────────────────────────────────────────
# NOTE: The SDK build requires running from a native WSL terminal.
# Windows npm cannot install esbuild (native binary) over UNC/WSL paths.
# Run these recipes from your WSL shell, NOT from a Windows terminal.

# Install SDK build deps. Run once from WSL terminal: just install-sdk
install-sdk:
    cd packages/sdk && npm install

# Build tyche-sdk: ESM + CJS bundles + TypeScript declarations.
# Output: packages/sdk/dist/
# Requires: just install-sdk (from WSL terminal)
build-sdk:
    cd packages/sdk && npm run build

# Typecheck the SDK without bundling.
typecheck-sdk:
    cd packages/sdk && npm run typecheck

# ── Build ──────────────────────────────────────────────────────────────────────

# Build all three SBF program binaries.
# Output: target/deploy/tyche_core.so, tyche_escrow.so, tyche_auction.so
build:
    cargo build-sbf

# ── Tests (TypeScript, devnet) ─────────────────────────────────────────────────
# NOTE: All TypeScript tests target devnet and MUST run from a WSL terminal.
# Windows npm cannot install @solana/kit (native esbuild dependency) on UNC paths.
#
# First-time setup: just setup-tests
# Add your Helius RPC URL to tests/ts/.env.test for faster, reliable tests.

# Bootstrap: generate keypairs, airdrop devnet SOL, write .env.test, npm install.
# Run once after cloning. Re-run to refresh keypair funding.
setup-tests:
    bash scripts/setup-test-env.sh

# Install TypeScript test dependencies (from WSL terminal).
install-tests:
    cd tests/ts && npm install

# Run all TypeScript tests (programs + SDK) against devnet.
test-ts:
    cd tests/ts && npm test

# Run only the raw program-level tests (no SDK wrapper).
test-programs:
    cd tests/ts && npm run test:programs

# Run only the SDK-level tests.
test-sdk:
    cd tests/ts && npm run test:sdk

# Run TypeScript tests in watch mode (re-runs on file changes).
test-watch:
    cd tests/ts && npm run test:watch

# Run a specific test file. Example: just test-file sdk/pdas
test-file FILE:
    cd tests/ts && npx vitest run --reporter=verbose {{FILE}}

# Default test target — TypeScript devnet tests.
test: test-ts

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
    rm -rf packages/sdk/dist/
