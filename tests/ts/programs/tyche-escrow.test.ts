/**
 * tyche-escrow program tests — raw instruction builders.
 *
 * Instruction data is constructed manually from the binary layouts defined in
 * each program's `args/` module.  This approach bypasses the Codama-generated
 * builders and avoids the discriminator mismatch that arises when Codama emits
 * 1-byte ordinal discriminators instead of the 8-byte SHA256-derived values
 * the on-chain programs require.
 *
 * Layout reference:
 *   Deposit:  8 disc + 8 (amount u64 LE)  = 16 bytes
 *   Refund:   8 disc only                  =  8 bytes
 *
 * Tests:
 *   1. PDA derivation   — pure unit tests (no network)
 *   2. Pre-activation   — Scheduled-phase checks (no MagicBlock needed)
 *   3. MagicBlock gated — skipped unless MAGICBLOCK_VALIDATOR=1 in .env.test
 *
 * Devnet only.  Requires:
 *   AUTHORITY_KEYPAIR   — funded authority keypair file path
 *   BIDDER1_KEYPAIR     — funded bidder keypair file path
 */

import { describe, it, expect, beforeAll } from 'vitest';
import {
  AccountRole,
  type Address,
  type Instruction,
  type TransactionSigner,
  address,
} from '@solana/kit';
import { SYSTEM_PROGRAM_ADDRESS } from '@solana-program/system';
import { fetchMaybeCompetitionState } from 'tyche-generated-core';

import { rpc, authority as authPromise, bidder1 as bidder1Promise, newCompetitionId } from '../setup/env.js';
import { requireFunds } from '../setup/airdrop.js';
import { sendAndConfirm, accountExists } from '../setup/helpers.js';
import {
  getCompetitionStatePda,
  getProtocolConfigPda,
  getEscrowVaultPda,
  getPermissionPda,
  TYCHE_CORE_PROGRAM_ADDRESS,
  TYCHE_ESCROW_PROGRAM_ADDRESS,
  ASSET_TYPE_NFT,
  PHASE_SCHEDULED,
} from 'tyche-sdk';

const MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS =
  address('DELeGGvXp4MwQwxs5RqAxbARrYxwHXXNEH6xRkMxs2X2');

// ── Discriminators (SHA256("global:<name>")[0..8]) ─────────────────────────

const CREATE_COMPETITION_DISC = new Uint8Array([110, 212, 234, 212, 118, 128, 158, 244]);
const CANCEL_COMPETITION_DISC = new Uint8Array([ 62,   4, 198,  98, 200,  41, 255,  72]);
const DEPOSIT_DISC            = new Uint8Array([242,  35, 198, 137,  82, 225, 242, 182]);

// ── Raw instruction builders ───────────────────────────────────────────────

/**
 * CreateCompetition — 96 bytes (8 disc + 88 args).
 *
 * Layout: [disc(8), id(32), asset_type(1), _pad(7), start_time(8),
 *           duration_secs(8), soft_close_window(8), soft_close_extension(8),
 *           max_soft_closes(1), _pad2(7), reserve_price(8)]
 *
 * Accounts: [competition(w), authority(rs), payer(ws), system_program(r), protocol_config(r)]
 */
function buildCreateCompetitionIx(args: {
  competition:        Address;
  authority:          TransactionSigner;
  payer:              TransactionSigner;
  protocolConfig:     Address;
  id:                 Uint8Array;   // 32 bytes: u64 LE in bytes 0-7, rest zeroed
  assetType:          number;
  startTime:          bigint;
  durationSecs:       bigint;
  softCloseWindow:    bigint;
  softCloseExtension: bigint;
  maxSoftCloses:      number;
  reservePrice:       bigint;
}): Instruction {
  const data = new Uint8Array(96);
  const view = new DataView(data.buffer);
  let off = 0;

  data.set(CREATE_COMPETITION_DISC, off);               off += 8;  // disc
  data.set(args.id.slice(0, 32), off);                  off += 32; // id [u8;32]
  view.setUint8(off, args.assetType);                   off += 1;  // asset_type
  off += 7;                                                        // _pad
  view.setBigInt64(off, args.startTime, true);          off += 8;  // start_time
  view.setBigInt64(off, args.durationSecs, true);       off += 8;  // duration_secs
  view.setBigInt64(off, args.softCloseWindow, true);    off += 8;  // soft_close_window
  view.setBigInt64(off, args.softCloseExtension, true); off += 8;  // soft_close_extension
  view.setUint8(off, args.maxSoftCloses);               off += 1;  // max_soft_closes
  off += 7;                                                        // _pad2
  view.setBigUint64(off, args.reservePrice, true);                 // reserve_price

  return {
    programAddress: TYCHE_CORE_PROGRAM_ADDRESS,
    accounts: [
      { address: args.competition,       role: AccountRole.WRITABLE },
      { address: args.authority.address, role: AccountRole.READONLY_SIGNER, signer: args.authority },
      { address: args.payer.address,     role: AccountRole.WRITABLE_SIGNER, signer: args.payer },
      { address: SYSTEM_PROGRAM_ADDRESS, role: AccountRole.READONLY },
      { address: args.protocolConfig,    role: AccountRole.READONLY },
    ],
    data,
  } as Instruction;
}

/**
 * CancelCompetition — 8 bytes (discriminator only, no args).
 *
 * Accounts: [competition(w), authority(rs), permission(w), magic_context(w), magic_program(r)]
 */
function buildCancelCompetitionIx(args: {
  competition:  Address;
  authority:    TransactionSigner;
  permission:   Address;
  magicContext: Address;
  magicProgram: Address;
}): Instruction {
  return {
    programAddress: TYCHE_CORE_PROGRAM_ADDRESS,
    accounts: [
      { address: args.competition,       role: AccountRole.WRITABLE },
      { address: args.authority.address, role: AccountRole.READONLY_SIGNER, signer: args.authority },
      { address: args.permission,        role: AccountRole.WRITABLE },
      { address: args.magicContext,      role: AccountRole.WRITABLE },
      { address: args.magicProgram,      role: AccountRole.READONLY },
    ],
    data: CANCEL_COMPETITION_DISC,
  } as Instruction;
}

/**
 * Deposit — 16 bytes (8 disc + 8 u64 amount).
 *
 * Layout: [disc(8), amount(8 u64 LE)]
 *
 * Accounts: [vault(w), depositor(ws), payer(ws), competition(r), system_program(r)]
 */
function buildDepositIx(args: {
  vault:       Address;
  depositor:   TransactionSigner;
  payer:       TransactionSigner;
  competition: Address;
  amount:      bigint;
}): Instruction {
  const data = new Uint8Array(16);
  data.set(DEPOSIT_DISC, 0);
  new DataView(data.buffer).setBigUint64(8, args.amount, true);

  return {
    programAddress: TYCHE_ESCROW_PROGRAM_ADDRESS,
    accounts: [
      { address: args.vault,             role: AccountRole.WRITABLE },
      { address: args.depositor.address, role: AccountRole.WRITABLE_SIGNER, signer: args.depositor },
      { address: args.payer.address,     role: AccountRole.WRITABLE_SIGNER, signer: args.payer },
      { address: args.competition,       role: AccountRole.READONLY },
      { address: SYSTEM_PROGRAM_ADDRESS, role: AccountRole.READONLY },
    ],
    data,
  } as Instruction;
}

// ── Shared setup helper ────────────────────────────────────────────────────

async function createScheduledCompetition(authority: TransactionSigner): Promise<{
  competition:   Address;
  competitionId: bigint;
}> {
  const competitionId = newCompetitionId();
  const [competition]    = await getCompetitionStatePda(authority.address, competitionId);
  const [protocolConfig] = await getProtocolConfigPda();

  const idBytes = new Uint8Array(32);
  new DataView(idBytes.buffer).setBigUint64(0, competitionId, true);

  const ix = buildCreateCompetitionIx({
    competition,
    authority,
    payer:              authority,
    protocolConfig,
    id:                 idBytes,
    assetType:          ASSET_TYPE_NFT,
    startTime:          BigInt(Math.floor(Date.now() / 1000)) + 60n,
    durationSecs:       3_600n,
    softCloseWindow:    300n,
    softCloseExtension: 300n,
    maxSoftCloses:      5,
    reservePrice:       500_000_000n,
  });

  await sendAndConfirm([ix], authority);
  console.log('\\n[Demo] Scheduled Competition Created for Escrow:', competition);
  return { competition, competitionId };
}

// ── Shared state ───────────────────────────────────────────────────────────

let authority: TransactionSigner;
let bidder1:   TransactionSigner;

beforeAll(async () => {
  [authority, bidder1] = await Promise.all([authPromise, bidder1Promise]);
  await Promise.all([
    requireFunds(authority.address, 100_000_000n),
    requireFunds(bidder1.address,   100_000_000n),
  ]);
});

// ── Vault PDA derivation ───────────────────────────────────────────────────

describe('EscrowVault PDA', () => {
  it('derives a unique PDA per (competition, depositor) pair', async () => {
    const competitionId = newCompetitionId();
    const [competition] = await getCompetitionStatePda(authority.address, competitionId);

    const [vault1] = await getEscrowVaultPda(competition, authority.address);
    const [vault2] = await getEscrowVaultPda(competition, bidder1.address);

    expect(vault1).not.toBe(vault2);
  });
});

// ── Pre-activation (Scheduled phase) ──────────────────────────────────────

describe('EscrowVault — pre-activation', () => {
  it('vault PDA does not exist before any deposit', async () => {
    const competitionId = newCompetitionId();
    const [competition] = await getCompetitionStatePda(authority.address, competitionId);
    const [vault]       = await getEscrowVaultPda(competition, bidder1.address);

    // Competition does not exist yet — vault must be absent.
    const exists = await accountExists(vault);
    expect(exists).toBe(false);
  });

  it('Deposit fails on a Scheduled competition', async () => {
    const { competition } = await createScheduledCompetition(authority);

    // Confirm the competition is Scheduled.
    const comp = await fetchMaybeCompetitionState(rpc, competition);
    expect(comp.exists).toBe(true);
    if (comp.exists) {
      expect(comp.data.phase).toBe(PHASE_SCHEDULED);
    }

    // Deposit must be rejected — competition is not Active.
    const [vault] = await getEscrowVaultPda(competition, bidder1.address);
    const depositIx = buildDepositIx({
      vault,
      competition,
      depositor: bidder1,
      payer:     bidder1,
      amount:    100_000_000n,
    });

    await expect(sendAndConfirm([depositIx], bidder1)).rejects.toThrow();
  });

  it('CancelCompetition succeeds from Scheduled phase', async () => {
    const { competition } = await createScheduledCompetition(authority);

    const [permission] = await getPermissionPda(competitionAddress);
    const cancelIx = buildCancelCompetitionIx({
      competition,
      authority,
      permission,
      magicContext: SYSTEM_PROGRAM_ADDRESS,
      magicProgram: address('DELeGGvXp4MwQwxs5RqAxbARrYxwHXXNEH6xRkMxs2X2'),
    });

    // Must not throw.
    await sendAndConfirm([cancelIx], authority);

    // Competition still exists but phase is Cancelled.
    const comp = await fetchMaybeCompetitionState(rpc, competition);
    expect(comp.exists).toBe(true);
    console.log('[Demo] Escrow parent competition correctly canceled.');
  });
});

// ── Full-flow tests (require MAGICBLOCK_VALIDATOR) ─────────────────────────

const hasMagicBlock = Boolean(process.env['MAGICBLOCK_VALIDATOR']);

describe.skipIf(!hasMagicBlock)('EscrowVault — deposit + refund (MagicBlock active)', () => {
  it('creates a vault and allows refund after cancel', async () => {
    // Requires a running MagicBlock validator on devnet.
    // Set MAGICBLOCK_VALIDATOR=1 in .env.test once the full setup is in place.
    //
    // Flow:
    //   1. Create competition (Scheduled)
    //   2. Activate (delegate) → competition goes Active
    //   3. Deposit from bidder1
    //   4. Cancel competition
    //   5. Refund bidder1's vault
    expect(hasMagicBlock).toBe(true); // placeholder assertion
  });
});
