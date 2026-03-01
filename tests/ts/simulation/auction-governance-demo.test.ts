
/**
 * Tyche Auction + Governance Simulation Demo
 *
 * Walks through the full Tyche sealed-bid auction lifecycle alongside the
 * SPL Governance voter-weight plugin. Each SOL deposit made via a bid
 * automatically becomes that bidder's governance voting power.
 *
 * ╔══════════════════════════════════════════════════════════════════╗
 * ║  Phase 0  Preflight       — verify wallets are funded           ║
 * ║  Phase 1  Governance      — registrar + per-voter VWR setup     ║
 * ║  Phase 2  Create Auction  — competition + auction on devnet     ║
 * ║  Phase 3  Activate    *   — delegate both accounts to PER       ║
 * ║  Phase 4  Bid         *   — Alice 0.350 SOL, Bob 0.150 SOL      ║
 * ║  Phase 5  Voting Power*   — UpdateVoterWeightRecord + leaderboard║
 * ╚══════════════════════════════════════════════════════════════════╝
 *
 *  * = requires MAGICBLOCK_VALIDATOR=1 in .env.test
 *
 * All wallets must be pre-funded on devnet.
 * If a wallet has insufficient SOL the phase will throw with the address
 * and exact amount needed — fund manually then re-run.
 *
 * Run (base phases, no MagicBlock):
 *   npx vitest run tests/ts/simulation/
 *
 * Run (full lifecycle):
 *   MAGICBLOCK_VALIDATOR=1 npx vitest run tests/ts/simulation/
 */

import { describe, it, expect, beforeAll } from 'vitest';
import {
  getProgramDerivedAddress,
  getAddressEncoder,
  generateKeyPairSigner,
  type Address,
  type Instruction,
  type TransactionSigner,
  AccountRole,
} from '@solana/kit';
import { SYSTEM_PROGRAM_ADDRESS } from '@solana-program/system';

import {
  rpc,
  authority as authorityPromise,
  bidder1   as bidder1Promise,
  bidder2   as bidder2Promise,
  newCompetitionId,
} from '../setup/env.js';
import {
  sendAndConfirm,
  sendAndConfirmWithBlockhash,
  getBlockhashForAccounts,
  accountExists,
} from '../setup/helpers.js';
import {
  buildCreateAuctionTransaction,
  buildActivateAuctionTransaction,
  buildPlaceBidTransaction,
  fetchDecodedCompetition,
  fetchDecodedAuction,
  getCompetitionStatePda,
  getAuctionStatePda,
  getEscrowVaultPda,
  getDelegationBufferPda,
  getDelegationRecordPda,
  getDelegationMetadataPda,
  getPermissionPda,
  MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS,
  MAGICBLOCK_PERMISSION_PROGRAM_ADDRESS,
  type MagicBlockActivateCompetitionAccounts,
  type MagicBlockDelegationAccounts,
} from 'tyche-sdk';

// ── Simulation parameters ─────────────────────────────────────────────────────

const ALICE_BID     = 250_000_000n;  // 0.250 SOL
const BOB_BID       = 150_000_000n;  // 0.150 SOL
const RESERVE_PRICE =  50_000_000n;  // 0.050 SOL  (below both bids)
const MIN_INCREMENT =  10_000_000n;  // 0.010 SOL
const DURATION_SECS = 3_600n;        // 1 hour
const COMMIT_MS     = 1_000;         // MagicBlock commit frequency (1 s)

const DUMMY_MINT = '11111111111111111111111111111111' as Address;

const DEVNET_VALIDATOR = (
  process.env['MAGICBLOCK_VALIDATOR_ADDRESS'] ?? 'LuzXEV3trGF4jQzpRzZaaTB9TqSwLkB7bpKQCQC7BAg'
) as Address;

const hasMagicBlock = Boolean(process.env['MAGICBLOCK_VALIDATOR']);

// ── Plugin constants ──────────────────────────────────────────────────────────

const PLUGIN_PROGRAM_ID =
  'TYGwvLsQWTNgwQcuP4sREXHVinz14WG9caEZecbKTVg' as Address;

const TYCHE_ESCROW_PROGRAM_ID =
  'TYEhGGkbujScDqPK1KTKCRu9cjVzjBH2Yf9Jb5L5Xtk' as Address;

const REALMS_PROGRAM_ID =
  'GovER5Lthms3bLBqWub97yVrMmEogzX7xNjdXpPPCVZw' as Address;

// Discriminators: SHA256("global:<name>")[0..8]
const DISC_CREATE_REGISTRAR               = new Uint8Array([132, 235,  36,  49, 139,  66, 202,  69]);
const DISC_CREATE_VOTER_WEIGHT_RECORD     = new Uint8Array([184, 249, 133, 178,  88, 152, 250, 186]);
const DISC_UPDATE_VOTER_WEIGHT_RECORD     = new Uint8Array([ 45, 185,   3,  36, 109, 190, 115, 169]);
const DISC_UPDATE_MAX_VOTER_WEIGHT_RECORD = new Uint8Array([103, 175, 201, 251,   2,   9, 251, 179]);

const SEED_REGISTRAR               = new TextEncoder().encode('registrar');
const SEED_VOTER_WEIGHT_RECORD     = new TextEncoder().encode('voter-weight-record');
const SEED_MAX_VOTER_WEIGHT_RECORD = new TextEncoder().encode('max-voter-weight-record');

const enc = getAddressEncoder();

// ── Logging utilities ─────────────────────────────────────────────────────────

const W = 68; // column width inside box borders

function banner(title: string): void {
  const bar = '═'.repeat(W);
  console.log(`\n╔${bar}╗`);
  console.log(`║  ${title.padEnd(W - 2)}║`);
  console.log(`╚${bar}╝\n`);
}

function section(title: string): void {
  console.log(`  ┌─ ${title}`);
}

function field(label: string, value: string, last = false): void {
  const prefix = last ? '  └─' : '  ├─';
  console.log(`${prefix} ${label.padEnd(28)} ${value}`);
}

function info(text: string): void {
  console.log(`  │  ${text}`);
}

function blank(): void {
  console.log('');
}

function toSol(lamports: bigint): string {
  return `${(Number(lamports) / 1e9).toFixed(3)} SOL  (${lamports.toLocaleString()} lamports)`;
}

function short(a: Address): string {
  return `${a.slice(0, 6)}…${a.slice(-4)}`;
}

function explorerUrl(sig: string): string {
  return `https://explorer.solana.com/tx/${sig}?cluster=devnet`;
}

// ── Plugin PDA helpers (not yet in tyche-sdk) ─────────────────────────────────

async function getRegistrarPda(realm: Address, mint: Address) {
  return getProgramDerivedAddress({
    programAddress: PLUGIN_PROGRAM_ID,
    seeds: [enc.encode(realm), SEED_REGISTRAR, enc.encode(mint)],
  });
}

async function getVoterWeightRecordPda(
  realm: Address,
  mint:  Address,
  voter: Address,
) {
  return getProgramDerivedAddress({
    programAddress: PLUGIN_PROGRAM_ID,
    seeds: [SEED_VOTER_WEIGHT_RECORD, enc.encode(realm), enc.encode(mint), enc.encode(voter)],
  });
}

async function getMaxVoterWeightRecordPda(realm: Address, mint: Address) {
  return getProgramDerivedAddress({
    programAddress: PLUGIN_PROGRAM_ID,
    seeds: [enc.encode(realm), SEED_MAX_VOTER_WEIGHT_RECORD, enc.encode(mint)],
  });
}

// ── Plugin instruction builders (no generated client yet) ────────────────────

/**
 * CreateRegistrar — one-time setup per (realm, mint) pair.
 * Allocates Registrar PDA (stores competition + escrow program) and the
 * MaxVoterWeightRecord PDA (initialised with u64::MAX weight).
 *
 * Data: 8 (disc) + 32 (governance_program_id) + 32 (competition) + 32 (tyche_escrow_program)
 */
function buildCreateRegistrarIx(args: {
  registrar:            Address;
  maxVoterWeightRecord: Address;
  realm:                Address;
  governingTokenMint:   Address;
  realmAuthority:       TransactionSigner;
  payer:                TransactionSigner;
  governanceProgramId:  Address;
  competition:          Address;
  tycheEscrowProgram:   Address;
}): Instruction {
  const data = new Uint8Array(8 + 96);
  data.set(DISC_CREATE_REGISTRAR, 0);
  data.set(enc.encode(args.governanceProgramId), 8);
  data.set(enc.encode(args.competition),         40);
  data.set(enc.encode(args.tycheEscrowProgram),  72);

  return {
    programAddress: PLUGIN_PROGRAM_ID,
    accounts: [
      { address: args.registrar,              role: AccountRole.WRITABLE },
      { address: args.maxVoterWeightRecord,   role: AccountRole.WRITABLE },
      { address: args.realm,                  role: AccountRole.READONLY },
      { address: args.governingTokenMint,     role: AccountRole.READONLY },
      { address: args.realmAuthority.address, role: AccountRole.READONLY_SIGNER, signer: args.realmAuthority },
      { address: args.payer.address,          role: AccountRole.WRITABLE_SIGNER, signer: args.payer },
      { address: SYSTEM_PROGRAM_ADDRESS,      role: AccountRole.READONLY },
    ],
    data,
  } as Instruction;
}

/**
 * CreateVoterWeightRecord — initialises a per-voter VWR with weight = 0.
 * Called once per bidder before they can vote.
 *
 * Data: 8 (disc only)
 */
function buildCreateVoterWeightRecordIx(args: {
  voterWeightRecord:  Address;
  registrar:          Address;
  voter:              TransactionSigner;
  payer:              TransactionSigner;
  realm:              Address;
  governingTokenMint: Address;
}): Instruction {
  return {
    programAddress: PLUGIN_PROGRAM_ID,
    accounts: [
      { address: args.voterWeightRecord,  role: AccountRole.WRITABLE },
      { address: args.registrar,          role: AccountRole.READONLY },
      { address: args.realm,              role: AccountRole.READONLY },
      { address: args.governingTokenMint, role: AccountRole.READONLY },
      { address: args.voter.address,      role: AccountRole.READONLY_SIGNER, signer: args.voter },
      { address: args.payer.address,      role: AccountRole.WRITABLE_SIGNER, signer: args.payer },
      { address: SYSTEM_PROGRAM_ADDRESS,  role: AccountRole.READONLY },
    ],
    data: DISC_CREATE_VOTER_WEIGHT_RECORD,
  } as Instruction;
}

/**
 * UpdateVoterWeightRecord — reads EscrowVault.amount → writes to VWR.
 * voter_weight     = EscrowVault.amount (lamports deposited)
 * voter_weight_expiry = Some(current_slot)  — forces same-tx voting
 * weight_action    = Some(CastVote)
 *
 * Data: 8 (disc) + 1 (action byte: 0 = CastVote)
 */
function buildUpdateVoterWeightRecordIx(args: {
  voterWeightRecord: Address;
  registrar:         Address;
  escrowVault:       Address;
  voter:             TransactionSigner;
  proposal:          Address;
  action?:           number;
}): Instruction {
  const data = new Uint8Array(9);
  data.set(DISC_UPDATE_VOTER_WEIGHT_RECORD, 0);
  data[8] = args.action ?? 0; // 0 = CastVote

  return {
    programAddress: PLUGIN_PROGRAM_ID,
    accounts: [
      { address: args.voterWeightRecord, role: AccountRole.WRITABLE },
      { address: args.registrar,         role: AccountRole.READONLY },
      { address: args.escrowVault,       role: AccountRole.READONLY },
      { address: args.voter.address,     role: AccountRole.READONLY_SIGNER, signer: args.voter },
      { address: args.proposal,          role: AccountRole.READONLY },
    ],
    data,
  } as Instruction;
}

/**
 * UpdateMaxVoterWeightRecord — stamps the current slot onto MaxVWR.
 * Marks the maximum possible voting weight as "fresh" for this slot.
 *
 * Data: 8 (disc only)
 */
function buildUpdateMaxVoterWeightRecordIx(args: {
  maxVoterWeightRecord: Address;
  registrar:            Address;
}): Instruction {
  return {
    programAddress: PLUGIN_PROGRAM_ID,
    accounts: [
      { address: args.maxVoterWeightRecord, role: AccountRole.WRITABLE },
      { address: args.registrar,            role: AccountRole.READONLY },
    ],
    data: DISC_UPDATE_MAX_VOTER_WEIGHT_RECORD,
  } as Instruction;
}

// ── Account helpers ───────────────────────────────────────────────────────────

/**
 * Read voter_weight (u64 LE at offset 104) from a VoterWeightRecord.
 * Layout mirrors spl_governance_addin_api::voter_weight::VoterWeightRecord.
 */
async function readVoterWeight(vwrAddress: Address): Promise<bigint> {
  const { value } = await rpc
    .getAccountInfo(vwrAddress, { encoding: 'base64', commitment: 'confirmed' })
    .send();
  if (!value) return 0n;
  const buf = Buffer.from(value.data[0], 'base64');
  if (buf.length < 112) return 0n;
  return buf.readBigUInt64LE(104);
}

/**
 * Read EscrowVault.amount (u64 LE at offset 72).
 * Layout: disc(8) + competition(32) + depositor(32) + amount(8)
 */
async function readVaultAmount(address: Address): Promise<bigint> {
  const { value } = await rpc
    .getAccountInfo(address, { encoding: 'base64', commitment: 'confirmed' })
    .send();
  if (!value) throw new Error(`EscrowVault not found: ${address}`);
  const buf = Buffer.from(value.data[0], 'base64');
  if (buf.length < 80) throw new Error(`EscrowVault too small: ${buf.length} bytes`);
  return buf.readBigUInt64LE(72);
}

/**
 * Read VoterWeightRecord slot expiry and action bytes for display.
 * Returns { slot, action } where action 0 = CastVote.
 */
async function readVwrMeta(vwrAddress: Address): Promise<{ slot: bigint; action: number }> {
  const { value } = await rpc
    .getAccountInfo(vwrAddress, { encoding: 'base64', commitment: 'confirmed' })
    .send();
  if (!value) return { slot: 0n, action: 0 };
  const buf = Buffer.from(value.data[0], 'base64');
  const slot   = buf.length >= 121 && buf[112] === 1 ? buf.readBigUInt64LE(113) : 0n;
  const action = buf.length >= 123 && buf[121] === 1 ? buf[122]! : 0;
  return { slot, action };
}

// ── Wallet funding check ──────────────────────────────────────────────────────

/**
 * Verifies a wallet has at least `minLamports`. If not, throws a descriptive
 * error with the address and the airdrop command to fund it.
 */
async function checkFunded(
  name:        string,
  address:     Address,
  minLamports: bigint,
): Promise<bigint> {
  const { value: balance } = await rpc
    .getBalance(address, { commitment: 'confirmed' })
    .send();

  if (balance < minLamports) {
    const need = ((Number(minLamports - balance) / 1e9) + 0.001).toFixed(3);
    const bar  = '═'.repeat(50);
    throw new Error(
      `\n\n  ╔${bar}╗\n` +
      `  ║  WALLET NEEDS FUNDING${' '.repeat(28)}║\n` +
      `  ╚${bar}╝\n\n` +
      `  Wallet:  ${name}\n` +
      `  Address: ${address}\n` +
      `  Missing: ${need} SOL\n\n` +
      `  Fund via CLI:\n` +
      `    solana airdrop ${need} ${address} --url devnet\n\n` +
      `  Fund via browser:\n` +
      `    https://faucet.solana.com\n`,
    );
  }

  return balance;
}

// ── PER commit poll ───────────────────────────────────────────────────────────

/**
 * Polls every 2 s until `address` appears on mainnet (committed from PER).
 * MagicBlock commits delegated state every `commitFrequencyMs` (here: 1 s).
 */
async function pollForAccount(
  address:   Address,
  label:     string,
  timeoutMs: number = 30_000,
): Promise<void> {
  console.log(`  ├─ Polling for ${label} to commit from PER to mainnet...`);
  const deadline = Date.now() + timeoutMs;
  let attempt    = 0;

  while (Date.now() < deadline) {
    attempt++;
    if (await accountExists(address)) {
      console.log(`  ├─ ✓ ${label} visible on mainnet  (attempt ${attempt})`);
      return;
    }
    process.stdout.write(`  │  attempt ${attempt} — not yet committed...\r`);
    await new Promise(r => setTimeout(r, 2_000));
  }

  throw new Error(`${label} (${address}) not committed to mainnet within ${timeoutMs} ms`);
}

// ── Shared simulation state ───────────────────────────────────────────────────

let authority: TransactionSigner;
let alice:     TransactionSigner;   // bidder1
let bob:       TransactionSigner;   // bidder2

let competitionId:      bigint;
let competitionAddress: Address;
let auctionAddress:     Address;

// Governance
let realm:                Address;
let mint:                 Address;
let registrar:            Address;
let maxVoterWeightRecord: Address;
let aliceVwr:             Address;
let bobVwr:               Address;

// EscrowVaults — created on PER by PlaceBid CPI into tyche-escrow::Deposit
let aliceVault: Address;
let bobVault:   Address;

// MagicBlock delegation PDAs
let compBuffer:   Address;
let compRecord:   Address;
let compMetadata: Address;
let aucBuffer:    Address;
let aucRecord:    Address;
let aucMetadata:  Address;
let permission:   Address;

// Auction start time — shared so Phase 3 can wait for it to pass
let startTime: bigint;

// Dummy governance proposal address (placeholder for weight_action_target)
let proposal: Address;

// ═════════════════════════════════════════════════════════════════════════════
//  SIMULATION
// ═════════════════════════════════════════════════════════════════════════════

describe('Tyche Auction + Governance Simulation', () => {

  // ── Global beforeAll ─────────────────────────────────────────────────────

  beforeAll(async () => {
    [authority, alice, bob] = await Promise.all([
      authorityPromise,
      bidder1Promise,
      bidder2Promise,
    ]);

    // Competition — derive PDA from authority + fresh ID so we know the address
    // before the on-chain create transaction.
    competitionId      = newCompetitionId();
    [competitionAddress] = await getCompetitionStatePda(authority.address, competitionId);
    [auctionAddress]     = await getAuctionStatePda(competitionAddress);

    // Governance keys — fresh per run to avoid "already initialised" conflicts.
    realm = (await generateKeyPairSigner()).address;
    mint  = (await generateKeyPairSigner()).address;

    [registrar]            = await getRegistrarPda(realm, mint);
    [maxVoterWeightRecord] = await getMaxVoterWeightRecordPda(realm, mint);
    [aliceVwr]             = await getVoterWeightRecordPda(realm, mint, alice.address);
    [bobVwr]               = await getVoterWeightRecordPda(realm, mint, bob.address);

    // EscrowVault PDAs — seeded [competition, depositor] by tyche-escrow.
    [aliceVault] = await getEscrowVaultPda(competitionAddress, alice.address);
    [bobVault]   = await getEscrowVaultPda(competitionAddress, bob.address);

    // MagicBlock delegation PDAs — one buffer/record/metadata per delegated account.
    [compBuffer]   = await getDelegationBufferPda(competitionAddress);
    [compRecord]   = await getDelegationRecordPda(competitionAddress);
    [compMetadata] = await getDelegationMetadataPda(competitionAddress);
    [aucBuffer]    = await getDelegationBufferPda(auctionAddress);
    [aucRecord]    = await getDelegationRecordPda(auctionAddress);
    [aucMetadata]  = await getDelegationMetadataPda(auctionAddress);
    [permission]   = await getPermissionPda(competitionAddress);

    // Dummy proposal — the governance target for CastVote weight action.
    proposal = (await generateKeyPairSigner()).address;
  });

  // ══════════════════════════════════════════════════════════════════════════
  // PHASE 0 — Preflight
  // ══════════════════════════════════════════════════════════════════════════

  it('[Phase 0] Preflight — verify wallets are funded', async () => {
    banner('PHASE 0 — PREFLIGHT');

    console.log('  Required balances:');
    console.log('    Authority   ≥ 0.100 SOL  (account creation + rent)');
    console.log('    Alice       ≥ 0.400 SOL  (bid 0.350 + rent buffer)');
    console.log('    Bob         ≥ 0.200 SOL  (bid 0.150 + rent buffer)\n');

    const [authBal, aliceBal, bobBal] = await Promise.all([
      checkFunded('Authority', authority.address, 100_000_000n),
      checkFunded('Alice (bidder1)', alice.address, 400_000_000n),
      checkFunded('Bob   (bidder2)', bob.address,   200_000_000n),
    ]);

    section('Participants');
    field('Authority',       `${authority.address}`);
    field('',                `balance: ${toSol(authBal)}`);
    field('Alice (bidder1)', `${alice.address}`);
    field('',                `balance: ${toSol(aliceBal)}`);
    field('Bob   (bidder2)', `${bob.address}`);
    field('',                `balance: ${toSol(bobBal)}`, true);

    blank();
    section('Competition (pre-computed PDAs)');
    field('Competition ID',   competitionId.toString());
    field('Competition PDA',  competitionAddress);
    field('AuctionState PDA', auctionAddress);
    field('Alice Vault PDA',  aliceVault);
    field('Bob Vault PDA',    bobVault, true);

    blank();
    section('Governance (fresh keys — no conflicts)');
    field('Realm',                  realm);
    field('Mint',                   mint);
    field('Registrar PDA',          registrar);
    field('MaxVoterWeightRecord',   maxVoterWeightRecord);
    field('Alice VoterWeightRecord', aliceVwr);
    field('Bob VoterWeightRecord',  bobVwr, true);

    blank();
    section('Auction parameters');
    field('Reserve price',    toSol(RESERVE_PRICE));
    field('Min bid increment', toSol(MIN_INCREMENT));
    field('Duration',         `${DURATION_SECS}s  (1 hour)`);
    field('Alice bid',        toSol(ALICE_BID));
    field('Bob bid',          toSol(BOB_BID), true);

    if (!hasMagicBlock) {
      console.log('\n  ┌─ MagicBlock status');
      console.log('  │  MAGICBLOCK_VALIDATOR is not set in .env.test');
      console.log('  │  Phases 3–5 (activate / bid / governance voting power) will be skipped.');
      console.log('  │  Phases 0–2 demonstrate governance setup and auction creation.');
      console.log('  └─ Add MAGICBLOCK_VALIDATOR=1 to .env.test for the full lifecycle.\n');
    } else {
      blank();
      section('MagicBlock PER');
      field('Validator',  DEVNET_VALIDATOR);
      field('Permission PDA', permission);
      field('Comp delegation buffer',   compBuffer);
      field('Comp delegation record',   compRecord);
      field('Comp delegation metadata', compMetadata);
      field('Auction delegation buffer',   aucBuffer);
      field('Auction delegation record',   aucRecord);
      field('Auction delegation metadata', aucMetadata, true);
      console.log('\n  ✓ MagicBlock configured — full lifecycle will run.\n');
    }
  });

  // ══════════════════════════════════════════════════════════════════════════
  // PHASE 1 — Governance setup
  // ══════════════════════════════════════════════════════════════════════════

  it('[Phase 1.1] Governance — CreateRegistrar', async () => {
    banner('PHASE 1 — GOVERNANCE SETUP');

    console.log('  The tyche-voter-weight-plugin implements the SPL Governance Add-in API.');
    console.log('  It derives each voter\'s weight from their SOL deposited in a Tyche');
    console.log('  EscrowVault, creating a direct link between economic stake and voting power.');
    blank();
    console.log('  Architecture:');
    console.log('    EscrowVault.amount (lamports)  ──▶  VoterWeightRecord.voter_weight');
    console.log('    1 lamport deposited            ──▶  1 vote');
    console.log('    weight expires after 1 slot    ──▶  UpdateVoterWeightRecord must be');
    console.log('                                        atomic with CastVote\n');

    section('CreateRegistrar');
    info('One Registrar per (realm, mint) pair. Stores:');
    info('  • competition address  — which vault balances count as voting power');
    info('  • tyche-escrow program — for vault ownership verification');
    info('  • governance program   — for SPL Governance compatibility\n');
    field('Realm',              realm);
    field('Mint',               mint);
    field('Plugin program',     PLUGIN_PROGRAM_ID);
    field('Competition',        competitionAddress);
    field('Escrow program',     TYCHE_ESCROW_PROGRAM_ID);
    field('Governance program', REALMS_PROGRAM_ID, true);

    const sig = await sendAndConfirm([
      buildCreateRegistrarIx({
        registrar,
        maxVoterWeightRecord,
        realm,
        governingTokenMint:  mint,
        realmAuthority:      authority,
        payer:               authority,
        governanceProgramId: REALMS_PROGRAM_ID,
        competition:         competitionAddress,
        tycheEscrowProgram:  TYCHE_ESCROW_PROGRAM_ID,
      }),
    ], authority);

    blank();
    console.log(`  TX: ${explorerUrl(sig)}\n`);
    section('Accounts created on-chain');
    field('Registrar PDA',           `${registrar}  ✓`);
    field('MaxVoterWeightRecord PDA', `${maxVoterWeightRecord}  ✓`);
    field('MaxVoterWeight (initial)', 'u64::MAX  (absolute threshold — DAO sets quorum explicitly)', true);
  });

  it('[Phase 1.2] Governance — CreateVoterWeightRecord × 2 (Alice and Bob)', async () => {
    blank();
    console.log('  Each bidder registers their VoterWeightRecord before their first bid.');
    console.log('  Initial voter_weight = 0. The plugin updates it when');
    console.log('  UpdateVoterWeightRecord is called after SOL is deposited.\n');

    // ── Alice ──────────────────────────────────────────────────────────────
    section('CreateVoterWeightRecord — Alice');
    field('Voter address', alice.address);
    field('VWR PDA',       aliceVwr, true);

    const sigA = await sendAndConfirm([
      buildCreateVoterWeightRecordIx({
        voterWeightRecord:  aliceVwr,
        registrar,
        voter:              alice,
        payer:              alice,
        realm,
        governingTokenMint: mint,
      }),
    ], alice);

    console.log(`\n  TX: ${explorerUrl(sigA)}`);
    console.log('  └─ Alice voter_weight: 0 lamports  (no deposit yet)\n');

    // ── Bob ────────────────────────────────────────────────────────────────
    section('CreateVoterWeightRecord — Bob');
    field('Voter address', bob.address);
    field('VWR PDA',       bobVwr, true);

    const sigB = await sendAndConfirm([
      buildCreateVoterWeightRecordIx({
        voterWeightRecord:  bobVwr,
        registrar,
        voter:              bob,
        payer:              bob,
        realm,
        governingTokenMint: mint,
      }),
    ], bob);

    console.log(`\n  TX: ${explorerUrl(sigB)}`);
    console.log('  └─ Bob voter_weight:  0 lamports  (no deposit yet)\n');

    console.log('  Both VoterWeightRecords created. Voting power will be stamped');
    console.log('  once each bidder\'s EscrowVault is funded via PlaceBid.');
  });

  // ══════════════════════════════════════════════════════════════════════════
  // PHASE 2 — Create auction
  // ══════════════════════════════════════════════════════════════════════════

  it('[Phase 2] Create competition + auction on devnet', async () => {
    banner('PHASE 2 — CREATE AUCTION');

    console.log('  buildCreateAuctionTransaction bundles two instructions:');
    console.log('    [0]  tyche-core    CreateCompetition  — state machine account');
    console.log('    [1]  tyche-auction CreateAuction       — sealed-bid configuration\n');

    // +3 s: small buffer so the create tx lands before start_time.
    // Phase 3 (Activate) will wait until this timestamp passes — the on-chain
    // check requires clock.unix_timestamp >= start_time before delegation.
    startTime = BigInt(Math.floor(Date.now() / 1000)) + 3n;

    section('Auction parameters');
    field('Competition ID',   competitionId.toString());
    field('Start time',       `${new Date(Number(startTime) * 1000).toISOString()}  (+3 s buffer — Phase 3 waits for this)`);
    field('Duration',         `${DURATION_SECS} s  (1 hour)`);
    field('Reserve price',    toSol(RESERVE_PRICE));
    field('Min bid increment', toSol(MIN_INCREMENT));
    field('Asset mint',       `${DUMMY_MINT}  (dummy — no NFT transfer in this demo)`, true);

    const { instructions } = await buildCreateAuctionTransaction({
      authority,
      payer:           authority,
      competitionId,
      startTime,
      durationSecs:    DURATION_SECS,
      reservePrice:    RESERVE_PRICE,
      assetMint:       DUMMY_MINT,
      minBidIncrement: MIN_INCREMENT,
    });

    console.log(`\n  Sending ${instructions.length} instructions in one transaction...`);
    const sig = await sendAndConfirm(instructions, authority);

    console.log(`\n  TX: ${explorerUrl(sig)}\n`);

    // ── Read back state ────────────────────────────────────────────────────
    const [comp, auction] = await Promise.all([
      fetchDecodedCompetition(rpc, competitionAddress),
      fetchDecodedAuction(rpc, auctionAddress),
    ]);

    section('CompetitionState  (tyche-core)');
    field('Address',           competitionAddress);
    field('Authority',         comp.authority);
    field('Phase',             `"${comp.phase}"  ← scheduled means created, not yet active`);
    field('Start time',        new Date(Number(comp.startTime) * 1000).toISOString());
    field('End time',          new Date(Number(comp.endTime)   * 1000).toISOString());
    field('Reserve price',     toSol(comp.reservePrice));
    field('Participant count', `${comp.participantCount}  (0 — no bids placed yet)`, true);

    blank();
    section('AuctionState  (tyche-auction)');
    field('Address',           auctionAddress);
    field('Competition',       auction.competition);
    field('Asset mint',        auction.assetMint);
    field('Min bid increment', toSol(auction.minBidIncrement));
    field('Bid count',         `${auction.bidCount}  (0 — no bids placed yet)`, true);

    blank();
    if (!hasMagicBlock) {
      console.log('  ℹ  Set MAGICBLOCK_VALIDATOR=1 to continue through phases 3–5.');
      console.log('     The competition and governance accounts above are live on devnet.');
    } else {
      console.log('  ✓ Competition and AuctionState created. Proceeding to activation...');
    }
  });

  // ══════════════════════════════════════════════════════════════════════════
  // PHASE 3 — Activate (delegate to MagicBlock PER)
  // ══════════════════════════════════════════════════════════════════════════

  it.skipIf(!hasMagicBlock)('[Phase 3] Activate auction — delegate to MagicBlock PER', async () => {
    banner('PHASE 3 — ACTIVATE AUCTION  (MagicBlock PER Delegation)');

    console.log('  Activation delegates CompetitionState + AuctionState to the');
    console.log('  MagicBlock Ephemeral Rollup (PER). After this point:');
    console.log('    • PlaceBid transactions route through the MagicBlock Router');
    console.log('    • State changes happen on PER with sub-second finality');
    console.log('    • PER commits state back to mainnet every 1 s (commitFrequencyMs)\n');

    console.log('  buildActivateAuctionTransaction produces:');
    console.log('    [0]  tyche-core    ActivateCompetition  (phase: scheduled → active)');
    console.log('    [1]  tyche-auction ActivateAuction       (auction ready for bids)');
    console.log('    [2]  MagicBlock    DelegateAccount(CompetitionState)');
    console.log('    [3]  MagicBlock    DelegateAccount(AuctionState)\n');

    section('Delegation PDAs');
    field('Comp buffer',           compBuffer);
    field('Comp delegation record', compRecord);
    field('Comp metadata',         compMetadata);
    field('Auction buffer',           aucBuffer);
    field('Auction delegation record', aucRecord);
    field('Auction metadata',         aucMetadata);
    field('Permission PDA',           permission, true);

    const competitionDelegation: MagicBlockActivateCompetitionAccounts = {
      buffer:             compBuffer,
      delegationRecord:   compRecord,
      delegationMetadata: compMetadata,
      delegationProgram:  MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS,
      validator:          DEVNET_VALIDATOR,
      permission,
      permissionProgram:  MAGICBLOCK_PERMISSION_PROGRAM_ADDRESS,
    };

    const auctionDelegation: MagicBlockDelegationAccounts = {
      buffer:             aucBuffer,
      delegationRecord:   aucRecord,
      delegationMetadata: aucMetadata,
      delegationProgram:  MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS,
      validator:          DEVNET_VALIDATOR,
    };

    const { instructions } = buildActivateAuctionTransaction({
      authority,
      payer:             authority,
      competitionAddress,
      auctionStateAddress: auctionAddress,
      competitionDelegation,
      auctionDelegation,
      commitFrequencyMs: COMMIT_MS,
    });

    // ── Wait for start_time to pass ────────────────────────────────────────
    // activate.rs step 4: `if clock.unix_timestamp < state.start_time → AuctionNotStarted`
    // Create used start_time = now + 3 s. Poll until wall clock >= start_time.
    {
      const waitUntilMs = Number(startTime + 1n) * 1000; // +1 s past start_time for safety
      const waitMs      = waitUntilMs - Date.now();
      if (waitMs > 0) {
        console.log(`\n  ⏳ Waiting ${(waitMs / 1000).toFixed(1)} s for start_time to pass before activating...`);
        await new Promise(r => setTimeout(r, waitMs));
        console.log('  ✓ Start time reached — sending Activate.\n');
      }
    }

    console.log(`\n  Sending ${instructions.length} instructions in one transaction...`);
    const sig = await sendAndConfirm(instructions, authority);

    console.log(`\n  TX: ${explorerUrl(sig)}\n`);

    const comp = await fetchDecodedCompetition(rpc, competitionAddress);
    section('CompetitionState after activation');
    field('Phase', `"${comp.phase}"  ← competition is live on PER`, true);

    blank();
    console.log('  PlaceBid now routes via the MagicBlock Router:');
    field('Router endpoint', 'https://devnet-router.magicblock.app', true);
    console.log('\n  The Router calls getBlockhashForAccounts([competition, auctionState, ...])');
    console.log('  and returns a blockhash from the PER node since those accounts are delegated.');
  });

  // ══════════════════════════════════════════════════════════════════════════
  // PHASE 4 — Place bids (via MagicBlock Router)
  // ══════════════════════════════════════════════════════════════════════════

  it.skipIf(!hasMagicBlock)('[Phase 4.1] Alice places bid — 0.350 SOL', async () => {
    banner('PHASE 4 — BID PLACEMENT  (MagicBlock Router)');

    console.log('  Bids flow through the MagicBlock Router, not standard Solana RPC.');
    console.log('  The Router inspects the PlaceBid accounts to determine routing:');
    console.log('    delegated accounts  →  route to PER node (ephemeral rollup)');
    console.log('    undelegated accounts →  route to mainnet\n');
    console.log('  PlaceBid atomically CPIs into tyche-escrow::Deposit, creating an');
    console.log('  EscrowVault that holds the bidder\'s SOL. This vault is the source');
    console.log('  of truth for governance voting power.\n');

    // ── Alice bids ─────────────────────────────────────────────────────────
    section('Alice bids 0.350 SOL');
    field('Bidder',           `${alice.address}  (Alice)`);
    field('Amount',           toSol(ALICE_BID));
    field('Competition',      competitionAddress);
    field('AuctionState',     auctionAddress);
    field('EscrowVault',      `${aliceVault}  (created by tyche-escrow::Deposit CPI)`, true);

    const { instruction: bidIx, accounts: bidAccounts } = await buildPlaceBidTransaction({
      bidder:             alice,
      payer:              alice,
      competitionAddress,
      auctionStateAddress: auctionAddress,
      amount:             ALICE_BID,
    });

    blank();
    console.log('  Calling MagicBlock Router: getBlockhashForAccounts...');
    console.log(`  Accounts: [${bidAccounts.map(a => short(a)).join(', ')}]`);

    const blockhash = await getBlockhashForAccounts(bidAccounts);
    console.log(`  Router blockhash: ${blockhash.blockhash.slice(0, 16)}…  (from PER node)`);

    const sigA = await sendAndConfirmWithBlockhash([bidIx], alice, blockhash);
    console.log(`\n  TX (on PER): ${explorerUrl(sigA)}\n`);

    // ── Poll for EscrowVault and BidRecord on mainnet ──────────────────────
    console.log('  PER commits state to mainnet every ~1 s. Polling...\n');
    await pollForAccount(aliceVault, `Alice EscrowVault (${short(aliceVault)})`);

    const lockedAmount = await readVaultAmount(aliceVault);
    blank();
    section('Alice EscrowVault  (committed to mainnet)');
    field('Vault address',  aliceVault);
    field('Competition',    short(competitionAddress));
    field('Depositor',      `${alice.address}  (Alice)`);
    field('Amount locked',  toSol(lockedAmount), true);

    console.log('\n  This vault balance is the ONLY source of Alice\'s governance voting power.');
    console.log('  voter_weight will be set to this amount by UpdateVoterWeightRecord.\n');

    expect(lockedAmount).toBe(ALICE_BID);
  });

  it.skipIf(!hasMagicBlock)('[Phase 4.2] Bob places bid — 0.150 SOL', async () => {
    section('Bob bids 0.150 SOL');
    field('Bidder',       `${bob.address}  (Bob)`);
    field('Amount',       toSol(BOB_BID));
    field('EscrowVault',  `${bobVault}  (will be created by tyche-escrow::Deposit CPI)`, true);

    const { instruction: bidIx, accounts: bidAccounts } = await buildPlaceBidTransaction({
      bidder:             bob,
      payer:              bob,
      competitionAddress,
      auctionStateAddress: auctionAddress,
      amount:             BOB_BID,
    });

    blank();
    console.log('  Routing via MagicBlock Router...');
    const blockhash = await getBlockhashForAccounts(bidAccounts);
    console.log(`  Router blockhash: ${blockhash.blockhash.slice(0, 16)}…  (from PER node)`);

    const sigB = await sendAndConfirmWithBlockhash([bidIx], bob, blockhash);
    console.log(`\n  TX (on PER): ${explorerUrl(sigB)}\n`);

    await pollForAccount(bobVault, `Bob EscrowVault (${short(bobVault)})`);

    const lockedAmount = await readVaultAmount(bobVault);
    blank();
    section('Bob EscrowVault  (committed to mainnet)');
    field('Vault address',  bobVault);
    field('Depositor',      `${bob.address}  (Bob)`);
    field('Amount locked',  toSol(lockedAmount), true);

    // ── Show auction state (bids sealed) ──────────────────────────────────
    blank();
    const auction = await fetchDecodedAuction(rpc, auctionAddress);
    section('AuctionState snapshot  (bids are sealed in PER TEE)');
    field('Bid count',        `${auction.bidCount}`);
    field('Current high bid', '[SEALED — encrypted in PER TEE, not readable from mainnet]');
    field('Current winner',   '[SEALED — encrypted in PER TEE, not readable from mainnet]', true);

    console.log('\n  Bid amounts remain private until the competition ends and settlement');
    console.log('  begins. The winning bid is revealed only during the CLEAR phase.');
    console.log('  This is the core privacy guarantee of the sealed-bid design.\n');

    expect(lockedAmount).toBe(BOB_BID);
  });

  // ══════════════════════════════════════════════════════════════════════════
  // PHASE 5 — Governance voting power
  // ══════════════════════════════════════════════════════════════════════════

  it.skipIf(!hasMagicBlock)('[Phase 5] Governance — UpdateVoterWeightRecord + leaderboard', async () => {
    banner('PHASE 5 — GOVERNANCE VOTING POWER');

    console.log('  UpdateVoterWeightRecord reads each bidder\'s EscrowVault on-chain and');
    console.log('  writes the deposited amount directly into their VoterWeightRecord.');
    blank();
    console.log('  Key properties:');
    console.log('    voter_weight         = EscrowVault.amount  (exact lamports)');
    console.log('    voter_weight_expiry  = Some(current_slot)  ← expires after THIS slot');
    console.log('    weight_action        = Some(CastVote)');
    console.log('    weight_action_target = Some(proposal pubkey)\n');
    console.log('  The just-in-time expiry forces UpdateVoterWeightRecord to be bundled');
    console.log('  atomically with CastVote in the same transaction — preventing stale');
    console.log('  weight from being reused across slots.\n');

    // ── UpdateMaxVoterWeightRecord ─────────────────────────────────────────
    section('UpdateMaxVoterWeightRecord  (refresh slot expiry)');
    field('MaxVWR',    maxVoterWeightRecord, true);

    const sigMax = await sendAndConfirm([
      buildUpdateMaxVoterWeightRecordIx({ maxVoterWeightRecord, registrar }),
    ], authority);
    console.log(`\n  TX: ${explorerUrl(sigMax)}`);
    console.log('  └─ MaxVoterWeightRecord stamped: max_voter_weight = u64::MAX, expiry = current_slot\n');

    // ── Alice UpdateVoterWeightRecord ──────────────────────────────────────
    blank();
    section('UpdateVoterWeightRecord — Alice');
    info('Plugin reads Alice\'s EscrowVault and writes voter_weight to her VWR.');
    info('');
    field('VoterWeightRecord', aliceVwr);
    field('Registrar',         registrar);
    field('EscrowVault',       aliceVault);
    field('Voter (signer)',    alice.address);
    field('Proposal (target)', `${proposal}  ← dummy placeholder for governance vote`, true);

    const sigA = await sendAndConfirm([
      buildUpdateVoterWeightRecordIx({
        voterWeightRecord: aliceVwr,
        registrar,
        escrowVault:       aliceVault,
        voter:             alice,
        proposal,
        action:            0, // CastVote
      }),
    ], alice);

    const aliceWeight = await readVoterWeight(aliceVwr);
    const aliceMeta   = await readVwrMeta(aliceVwr);

    console.log(`\n  TX: ${explorerUrl(sigA)}\n`);
    section('Alice VoterWeightRecord  (on-chain)');
    field('voter_weight',        toSol(aliceWeight));
    field('voter_weight_expiry', `Some(slot ${aliceMeta.slot})  ← must vote THIS slot`);
    field('weight_action',       `Some(${aliceMeta.action === 0 ? 'CastVote' : aliceMeta.action})`, true);
    console.log('\n  Alice\'s voting power = her locked deposit. Exactly 1 lamport per vote.\n');

    // ── Bob UpdateVoterWeightRecord ────────────────────────────────────────
    blank();
    section('UpdateVoterWeightRecord — Bob');
    field('VoterWeightRecord', bobVwr);
    field('EscrowVault',       bobVault);
    field('Voter (signer)',    bob.address, true);

    const sigB = await sendAndConfirm([
      buildUpdateVoterWeightRecordIx({
        voterWeightRecord: bobVwr,
        registrar,
        escrowVault:       bobVault,
        voter:             bob,
        proposal,
        action:            0,
      }),
    ], bob);

    const bobWeight = await readVoterWeight(bobVwr);
    const bobMeta   = await readVwrMeta(bobVwr);

    console.log(`\n  TX: ${explorerUrl(sigB)}\n`);
    section('Bob VoterWeightRecord  (on-chain)');
    field('voter_weight',        toSol(bobWeight));
    field('voter_weight_expiry', `Some(slot ${bobMeta.slot})  ← must vote THIS slot`);
    field('weight_action',       `Some(${bobMeta.action === 0 ? 'CastVote' : bobMeta.action})`, true);

    // ── Governance leaderboard ─────────────────────────────────────────────
    const totalLocked = aliceWeight + bobWeight;

    const participants = [
      { name: 'Alice', address: alice.address, weight: aliceWeight, bid: ALICE_BID },
      { name: 'Bob  ', address: bob.address,   weight: bobWeight,   bid: BOB_BID   },
    ].sort((a, b) => (a.weight > b.weight ? -1 : 1));

    const LW    = 70;
    const hbar  = '═'.repeat(LW);
    const tbar  = '─'.repeat(LW);

    console.log(`\n\n  ╔${hbar}╗`);
    console.log(`  ║  ${'GOVERNANCE LEADERBOARD — Voting Power by Escrow Deposit'.padEnd(LW - 2)}║`);
    console.log(`  ║  ${'Competition: ' + short(competitionAddress) + '    Plugin: ' + short(PLUGIN_PROGRAM_ID as Address)}${' '.repeat(LW - 2 - 13 - 13 - 4 - 11)}║`);
    console.log(`  ╠${hbar}╣`);
    console.log(`  ║  ${'Rank  Voter        Address                                    Voting Power'.padEnd(LW - 2)}║`);
    console.log(`  ║  ${'────  ───────────  ─────────────────────────────────────────  ────────────'.padEnd(LW - 2)}║`);

    participants.forEach(({ name, address: a, weight }, i) => {
      const rank     = `#${i + 1}`;
      const voteStr  = `${toSol(weight)}`;
      const line     = `${rank.padEnd(6)}${name.padEnd(13)}${a}  ${voteStr}`;
      console.log(`  ║  ${line.padEnd(LW - 2)}║`);
    });

    console.log(`  ╠${hbar}╣`);
    console.log(`  ║  ${'Total SOL locked:   ' + toSol(totalLocked)}${' '.repeat(Math.max(0, LW - 2 - 20 - toSol(totalLocked).length))}║`);
    console.log(`  ║  ${'Max voter weight:   u64::MAX  (absolute threshold — DAO sets quorum)'.padEnd(LW - 2)}║`);
    console.log(`  ║  ${'Model:              1 lamport deposited = 1 vote'.padEnd(LW - 2)}║`);
    console.log(`  ║  ${'Expiry:             current_slot  (must vote in same transaction as UpdateVWR)'.padEnd(LW - 2)}║`);
    console.log(`  ╚${hbar}╝\n`);

    console.log('  ┌─ What happens next');
    console.log('  │  The auction is still live on PER. When the competition window closes,');
    console.log('  │  the CLEAR phase begins: the highest sealed bid is revealed and the');
    console.log('  │  winner\'s SOL is released to the authority. Losing bidders can reclaim');
    console.log('  │  their EscrowVault deposits via tyche-escrow::Refund.');
    console.log('  │');
    console.log('  │  Governance votes cast while the auction is active use the live');
    console.log('  │  EscrowVault balance as weight — aligning bidding incentives with');
    console.log('  └─ protocol governance participation.\n');

    // ── Assertions ─────────────────────────────────────────────────────────
    expect(aliceWeight).toBe(ALICE_BID);
    expect(bobWeight).toBe(BOB_BID);
    expect(aliceWeight).toBeGreaterThan(bobWeight);
    expect(totalLocked).toBe(ALICE_BID + BOB_BID);
  });

});
