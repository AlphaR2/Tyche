#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# scripts/setup-realms.sh
#
# Deploy the tyche-voter-weight-plugin and register it with a SPL Governance
# Realm.  Covers the two steps that must happen on-chain before voting works:
#
#   1. Deploy the plugin binary (or verify it is already deployed).
#   2. Call CreateRegistrar on the plugin to bind it to a (Realm, Mint) pair.
#
# Prerequisites:
#   - solana CLI ≥ 2.0 installed and on PATH
#   - cargo build-sbf installed (cargo-build-sbf)
#   - Node.js ≥ 18 (with npx) for the CreateRegistrar step
#   - A funded payer keypair at PAYER_KEYPAIR (defaults to ~/.config/solana/id.json)
#   - Run from the repository root inside a WSL/Linux terminal
#
# Usage:
#   # Step 1 only — build & deploy the plugin, then stop.
#   bash scripts/setup-realms.sh --deploy-only
#
#   # Full setup — deploy + CreateRegistrar (all params required):
#   bash scripts/setup-realms.sh \
#     --realm          <REALM_ADDRESS>       \
#     --mint           <GOVERNING_MINT>      \
#     --competition    <COMPETITION_ADDRESS> \
#     --payer-keypair  ~/.config/solana/id.json
#
# Environment overrides:
#   RPC_URL          HTTP RPC endpoint  (default: https://api.devnet.solana.com)
#   PAYER_KEYPAIR    Path to payer keypair JSON
#   SKIP_BUILD       Set to 1 to skip cargo build-sbf (assumes .so already exists)
# ─────────────────────────────────────────────────────────────────────────────

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

PLUGIN_PROGRAM_ID="TYGwvLsQWTNgwQcuP4sREXHVinz14WG9caEZecbKTVg"
PLUGIN_SO="${REPO_ROOT}/target/deploy/tyche_voter_weight_plugin.so"
REALMS_PROGRAM_ID="GovER5Lthms3bLBqWub97yVrMmEogzX7xNjdXpPPCVZw"
TYCHE_ESCROW_PROGRAM_ID="TYEhGGkbujScDqPK1KTKCRu9cjVzjBH2Yf9Jb5L5Xtk"

RPC_URL="${RPC_URL:-https://api.devnet.solana.com}"
PAYER_KEYPAIR="${PAYER_KEYPAIR:-${HOME}/.config/solana/id.json}"

# ── Argument parsing ──────────────────────────────────────────────────────────

REALM=""
MINT=""
COMPETITION=""
DEPLOY_ONLY=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --realm)           REALM="$2";         shift 2 ;;
    --mint)            MINT="$2";          shift 2 ;;
    --competition)     COMPETITION="$2";   shift 2 ;;
    --payer-keypair)   PAYER_KEYPAIR="$2"; shift 2 ;;
    --deploy-only)     DEPLOY_ONLY=1;      shift   ;;
    *) echo "[ERROR] Unknown flag: $1"; exit 1 ;;
  esac
done

# ── Helpers ───────────────────────────────────────────────────────────────────

ok()   { echo -e "\033[32m[DONE]\033[0m $*"; }
info() { echo -e "\033[34m[INFO]\033[0m $*"; }
warn() { echo -e "\033[33m[WARN]\033[0m $*"; }
err()  { echo -e "\033[31m[ERROR]\033[0m $*"; exit 1; }

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " Tyche Voter Weight Plugin — Realms Setup"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
info "Plugin program ID : ${PLUGIN_PROGRAM_ID}"
info "RPC               : ${RPC_URL}"
info "Payer keypair     : ${PAYER_KEYPAIR}"
echo ""

# ── Step 1: Build ─────────────────────────────────────────────────────────────

if [[ "${SKIP_BUILD:-0}" == "1" ]]; then
  info "Skipping build (SKIP_BUILD=1)."
elif [[ -f "${PLUGIN_SO}" ]]; then
  info "Found existing build: ${PLUGIN_SO}"
  info "Set SKIP_BUILD=0 and re-run to force rebuild."
else
  echo "Building tyche-voter-weight-plugin (cargo build-sbf)..."
  cargo build-sbf -p tyche-voter-weight-plugin
  ok "Build complete: ${PLUGIN_SO}"
fi

echo ""

# ── Step 2: Deploy ────────────────────────────────────────────────────────────

PAYER_ADDRESS=$(solana-keygen pubkey "${PAYER_KEYPAIR}")
info "Payer address: ${PAYER_ADDRESS}"

BALANCE_RAW=$(solana balance "${PAYER_ADDRESS}" --url "${RPC_URL}" 2>/dev/null | awk '{print $1}')
info "Payer balance: ${BALANCE_RAW} SOL"

echo ""
echo "Deploying plugin to devnet..."
solana program deploy \
  "${PLUGIN_SO}" \
  --program-id "${REPO_ROOT}/target/deploy/tyche_voter_weight_plugin-keypair.json" \
  --keypair    "${PAYER_KEYPAIR}" \
  --url        "${RPC_URL}"

ok "Plugin deployed at: ${PLUGIN_PROGRAM_ID}"
echo ""

# ── Step 3: Realms UI instructions ───────────────────────────────────────────

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " How to create a Realm with the voter-weight plugin"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo " Option A — Realms UI (app.realms.today)"
echo "   1. Click  Create DAO"
echo "   2. Choose  Multi-Signature Wallet  or  Community Token DAO"
echo "   3. Open  Advanced settings"
echo "   4. Under  Community voter weight plugin,  paste:"
echo "      ${PLUGIN_PROGRAM_ID}"
echo "   5. Complete the Realm creation flow and confirm."
echo ""
echo " Option B — programmatic (spl-governance CLI or TypeScript)"
echo "   Pass --use-community-voter-weight-addin to CreateRealm"
echo "   with voter_weight_addin = ${PLUGIN_PROGRAM_ID}"
echo ""
echo " After the Realm is created, note the Realm address and governing"
echo " token mint, then run this script again with:"
echo ""
echo "   bash scripts/setup-realms.sh \\"
echo "     --realm       <REALM_ADDRESS>       \\"
echo "     --mint        <GOVERNING_MINT>      \\"
echo "     --competition <COMPETITION_ADDRESS> \\"
echo "     --payer-keypair ~/.config/solana/id.json"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

[[ "${DEPLOY_ONLY}" == "1" ]] && exit 0

# ── Step 4: CreateRegistrar ───────────────────────────────────────────────────

[[ -z "${REALM}" ]]       && err "Missing --realm.  See usage at the top of this script."
[[ -z "${MINT}" ]]        && err "Missing --mint."
[[ -z "${COMPETITION}" ]] && err "Missing --competition."

echo "Creating Registrar for Realm ${REALM}..."
echo ""

# Write a one-shot TypeScript helper and run it with tsx/ts-node.
# Uses the same @solana/kit v2 stack as the integration tests.
HELPER_TS="$(mktemp /tmp/create-registrar-XXXXXX.ts)"

cat > "${HELPER_TS}" <<'TSEOF'
import {
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  createKeyPairSignerFromBytes,
  getProgramDerivedAddress,
  getAddressEncoder,
  AccountRole,
  SYSTEM_PROGRAM_ADDRESS,
  type Address,
  type Instruction,
  type TransactionSigner,
  pipe,
  createTransactionMessage,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  appendTransactionMessageInstructions,
  signTransactionMessageWithSigners,
  sendAndConfirmTransactionFactory,
  getSignatureFromTransaction,
} from '@solana/kit';
import { readFileSync } from 'fs';

const [, , rpcUrl, payerKeypath, realmAddr, mintAddr, competitionAddr] = process.argv;

const PLUGIN_PROGRAM_ID   = 'TYGwvLsQWTNgwQcuP4sREXHVinz14WG9caEZecbKTVg' as Address;
const REALMS_PROGRAM_ID   = 'GovER5Lthms3bLBqWub97yVrMmEogzX7xNjdXpPPCVZw' as Address;
const TYCHE_ESCROW_ID     = 'TYEhGGkbujScDqPK1KTKCRu9cjVzjBH2Yf9Jb5L5Xtk' as Address;

const DISC_CREATE_REGISTRAR = new Uint8Array([132, 235, 36, 49, 139, 66, 202, 69]);
const SEED_REGISTRAR             = new TextEncoder().encode('registrar');
const SEED_MAX_VOTER_WEIGHT_RECORD = new TextEncoder().encode('max-voter-weight-record');

const enc = getAddressEncoder();

const rpc     = createSolanaRpc(rpcUrl);
const rpcWs   = createSolanaRpcSubscriptions(rpcUrl.replace('https://', 'wss://').replace('http://', 'ws://'));

async function main() {
  const keyBytes = JSON.parse(readFileSync(payerKeypath!, 'utf-8')) as number[];
  const payer    = await createKeyPairSignerFromBytes(new Uint8Array(keyBytes));

  const realm       = realmAddr  as Address;
  const mint        = mintAddr   as Address;
  const competition = competitionAddr as Address;

  const [registrar]            = await getProgramDerivedAddress({
    programAddress: PLUGIN_PROGRAM_ID,
    seeds: [enc.encode(realm), SEED_REGISTRAR, enc.encode(mint)],
  });

  const [maxVoterWeightRecord] = await getProgramDerivedAddress({
    programAddress: PLUGIN_PROGRAM_ID,
    seeds: [enc.encode(realm), SEED_MAX_VOTER_WEIGHT_RECORD, enc.encode(mint)],
  });

  const data = new Uint8Array(8 + 96);
  data.set(DISC_CREATE_REGISTRAR, 0);
  data.set(enc.encode(REALMS_PROGRAM_ID),  8);
  data.set(enc.encode(competition),       40);
  data.set(enc.encode(TYCHE_ESCROW_ID),   72);

  const ix: Instruction = {
    programAddress: PLUGIN_PROGRAM_ID,
    accounts: [
      { address: registrar,            role: AccountRole.WRITABLE },
      { address: maxVoterWeightRecord, role: AccountRole.WRITABLE },
      { address: realm,                role: AccountRole.READONLY },
      { address: mint,                 role: AccountRole.READONLY },
      { address: payer.address,        role: AccountRole.READONLY_SIGNER, signer: payer },
      { address: payer.address,        role: AccountRole.WRITABLE_SIGNER,  signer: payer },
      { address: SYSTEM_PROGRAM_ADDRESS, role: AccountRole.READONLY },
    ],
    data,
  } as Instruction;

  const { value: latestBlockhash } = await rpc
    .getLatestBlockhash({ commitment: 'confirmed' })
    .send();

  const tx = await pipe(
    createTransactionMessage({ version: 0 }),
    (m) => setTransactionMessageFeePayerSigner(payer, m),
    (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
    (m) => appendTransactionMessageInstructions([ix], m),
    (m) => signTransactionMessageWithSigners(m),
  );

  const sendAndConfirm = sendAndConfirmTransactionFactory({ rpc, rpcSubscriptions: rpcWs });
  await sendAndConfirm(tx, { commitment: 'confirmed' });

  const sig = getSignatureFromTransaction(tx);
  console.log('Registrar    :', registrar);
  console.log('MaxVWR       :', maxVoterWeightRecord);
  console.log('Transaction  :', sig);
}

main().catch((e) => { console.error(e); process.exit(1); });
TSEOF

# Run the helper — prefer tsx (faster), fall back to ts-node.
TS_RUNNER="npx tsx"
if ! npx tsx --version &>/dev/null 2>&1; then
  TS_RUNNER="npx ts-node --esm"
fi

(cd "${REPO_ROOT}/tests/ts" && ${TS_RUNNER} "${HELPER_TS}" \
  "${RPC_URL}" "${PAYER_KEYPAIR}" "${REALM}" "${MINT}" "${COMPETITION}")

rm -f "${HELPER_TS}"

echo ""
ok "Registrar created.  The voter-weight plugin is now active for this Realm."
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " Next steps for voters:"
echo "   1. Each voter calls CreateVoterWeightRecord once (per-voter init)."
echo "   2. Before casting a vote, call UpdateVoterWeightRecord in the same tx."
echo "   3. Pass the VoterWeightRecord address to CastVote."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
