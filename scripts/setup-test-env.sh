#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# scripts/setup-test-env.sh
#
# One-command bootstrap for the Tyche TypeScript test suite.
#
# What it does:
#   1. Generates fresh test keypairs (authority, bidder1, bidder2, crank).
#   2. Airdrops SOL to each keypair on devnet.
#   3. Writes tests/ts/.env.test with the correct paths.
#   4. Installs npm dependencies inside tests/ts/.
#
# Prerequisites:
#   - solana CLI installed and on PATH
#   - Node.js ≥ 18 (with npm) installed
#   - Run from the repository root inside a WSL/Linux terminal
#
# Usage:
#   bash scripts/setup-test-env.sh [TREASURY_ADDRESS]
#
# TREASURY_ADDRESS defaults to the authority keypair address if not provided.
# ─────────────────────────────────────────────────────────────────────────────

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KEYS_DIR="${HOME}/.config/solana/tyche-test"
ENV_FILE="${REPO_ROOT}/tests/ts/.env.test"
TESTS_DIR="${REPO_ROOT}/tests/ts"

# If .env.test already has a custom RPC_URL, use it (preserves Helius keys etc.)
# Priority: environment variable > existing .env.test > public devnet fallback
if [[ -z "${RPC_URL:-}" ]] && [[ -f "${ENV_FILE}" ]]; then
  _saved_rpc=$(grep '^RPC_URL=' "${ENV_FILE}" 2>/dev/null | head -1 | cut -d= -f2-)
  [[ -n "${_saved_rpc}" ]] && export RPC_URL="${_saved_rpc}"
fi
RPC_URL="${RPC_URL:-https://api.devnet.solana.com}"

# Auto-derive WS URL (https:// → wss://) unless explicitly set or already in .env.test
if [[ -z "${RPC_WS_URL:-}" ]] && [[ -f "${ENV_FILE}" ]]; then
  _saved_ws=$(grep '^RPC_WS_URL=' "${ENV_FILE}" 2>/dev/null | head -1 | cut -d= -f2-)
  [[ -n "${_saved_ws}" ]] && export RPC_WS_URL="${_saved_ws}"
fi
RPC_WS_URL="${RPC_WS_URL:-${RPC_URL/https:\/\//wss:\/\/}}"
RPC_WS_URL="${RPC_WS_URL/http:\/\//ws:\/\/}"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " Tyche test environment setup"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# ── 1. Create keypair directory ───────────────────────────────────────────────
mkdir -p "${KEYS_DIR}"

# ── 2. Generate keypairs (skip if already exist) ──────────────────────────────
generate_keypair() {
  local path="$1"
  local name="$2"
  if [[ -f "${path}" ]]; then
    echo "  ✓ ${name} keypair already exists: ${path}"
  else
    solana-keygen new -o "${path}" --no-bip39-passphrase --silent
    echo "  ✓ Generated ${name} keypair: ${path}"
  fi
}

generate_keypair "${KEYS_DIR}/authority.json"  "authority"
generate_keypair "${KEYS_DIR}/bidder1.json"    "bidder1"
generate_keypair "${KEYS_DIR}/bidder2.json"    "bidder2"
generate_keypair "${KEYS_DIR}/crank.json"      "crank"

# ── 3. Resolve addresses ──────────────────────────────────────────────────────
AUTHORITY_ADDRESS=$(solana-keygen pubkey "${KEYS_DIR}/authority.json")
BIDDER1_ADDRESS=$(solana-keygen pubkey   "${KEYS_DIR}/bidder1.json")
BIDDER2_ADDRESS=$(solana-keygen pubkey   "${KEYS_DIR}/bidder2.json")
TREASURY_ADDRESS="${1:-${AUTHORITY_ADDRESS}}"

echo ""
echo "  Authority : ${AUTHORITY_ADDRESS}"
echo "  Bidder 1  : ${BIDDER1_ADDRESS}"
echo "  Bidder 2  : ${BIDDER2_ADDRESS}"
echo "  Treasury  : ${TREASURY_ADDRESS}"

# ── 4. Check balances + airdrop if empty ─────────────────────────────────────

# Fetch balance from devnet; prints the SOL amount or "ERROR" if unreachable.
get_balance() {
  local address="$1"
  local raw
  raw=$(solana balance "${address}" --url "${RPC_URL}" 2>&1) || { echo "ERROR"; return; }
  # Output is "X.XXXXXXXXX SOL" — extract the number
  echo "${raw}" | awk '{print $1}'
}

echo ""
echo "Checking devnet balances..."
AUTHORITY_BAL=$(get_balance "${AUTHORITY_ADDRESS}")
BIDDER1_BAL=$(get_balance   "${BIDDER1_ADDRESS}")
BIDDER2_BAL=$(get_balance   "${BIDDER2_ADDRESS}")

echo "  Authority : ${AUTHORITY_ADDRESS}  →  ${AUTHORITY_BAL} SOL"
echo "  Bidder 1  : ${BIDDER1_ADDRESS}  →  ${BIDDER1_BAL} SOL"
echo "  Bidder 2  : ${BIDDER2_ADDRESS}  →  ${BIDDER2_BAL} SOL"

airdrop_if_needed() {
  local address="$1"
  local name="$2"
  local balance="$3"

  if [[ "${balance}" == "ERROR" ]]; then
    echo "  ⚠  ${name}: could not fetch balance (RPC error) — skipping airdrop"
    return
  fi

  # Only airdrop when balance is exactly 0
  if awk "BEGIN {exit !($balance > 0)}"; then
    echo "  ✓ ${name} has ${balance} SOL — skipping airdrop"
    return
  fi

  echo "  ⬇ Airdropping 2 SOL to ${name} (${address})..."
  solana airdrop 2 "${address}" --url "${RPC_URL}" || {
    echo "  ⚠  Airdrop failed (rate-limited?). Retrying in 5 s..."
    sleep 5
    solana airdrop 1 "${address}" --url "${RPC_URL}" || echo "  ⚠  Airdrop still failed — fund manually: ${address}"
  }
}

echo ""
echo "Airdropping to empty accounts..."
airdrop_if_needed "${AUTHORITY_ADDRESS}" "authority" "${AUTHORITY_BAL}"
airdrop_if_needed "${BIDDER1_ADDRESS}"   "bidder1"   "${BIDDER1_BAL}"
airdrop_if_needed "${BIDDER2_ADDRESS}"   "bidder2"   "${BIDDER2_BAL}"

# ── 5. Write .env.test ────────────────────────────────────────────────────────
echo ""
echo "Writing ${ENV_FILE}..."

cat > "${ENV_FILE}" <<EOF
# Auto-generated by scripts/setup-test-env.sh — do not commit.

# HTTP RPC endpoint (replace with your Helius URL for faster, reliable testing)
RPC_URL=${RPC_URL}
# WebSocket endpoint (auto-derived from RPC_URL — override if using a different WS host)
RPC_WS_URL=${RPC_WS_URL}

AUTHORITY_KEYPAIR=${KEYS_DIR}/authority.json
BIDDER1_KEYPAIR=${KEYS_DIR}/bidder1.json
BIDDER2_KEYPAIR=${KEYS_DIR}/bidder2.json
CRANK_KEYPAIR=${KEYS_DIR}/crank.json

TREASURY_ADDRESS=${TREASURY_ADDRESS}

# Uncomment to enable MagicBlock PER tests:
# MAGICBLOCK_VALIDATOR=1
# MAGICBLOCK_VALIDATOR_ADDRESS=LuzXEV3trGF4jQzpRzZaaTB9TqSwLkB7bpKQCQC7BAg
EOF

echo "  ✓ Written"

# ── 6. Install npm dependencies ───────────────────────────────────────────────

# Ensure we use the Linux-native Node.js from nvm, not the Windows npm on PATH.
# Windows npm cannot install native addons (esbuild) over WSL filesystem paths.
export NVM_DIR="${HOME}/.nvm"
if [[ -s "${NVM_DIR}/nvm.sh" ]]; then
  # shellcheck source=/dev/null
  \. "${NVM_DIR}/nvm.sh"
  # Activate the default alias; fall back to the newest installed version.
  nvm use default 2>/dev/null || nvm use node 2>/dev/null || true
fi

# Sanity-check: warn loudly if npm still resolves to the Windows binary.
NPM_BIN="$(command -v npm 2>/dev/null || true)"
if [[ "${NPM_BIN}" == /mnt/c/* ]]; then
  echo ""
  echo "  ⚠  npm is resolving to Windows: ${NPM_BIN}"
  echo "  ⚠  Run  nvm alias default 24 && nvm use default  in your WSL terminal,"
  echo "  ⚠  then re-run this script.  Skipping npm install to avoid a hang."
  echo ""
else
  echo ""
  echo "Installing test npm dependencies (${TESTS_DIR})..."
  echo "  Using npm: ${NPM_BIN}"
  (cd "${TESTS_DIR}" && npm install)
  echo "  ✓ Done"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " Setup complete. Run tests with:"
echo ""
echo "   just test-ts                  # all TypeScript tests"
echo "   just test-programs            # program-level tests only"
echo "   just test-sdk                 # SDK tests only"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
