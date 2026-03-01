/**
 * tyche-core program tests — raw instruction builders.
 *
 * Instruction data is constructed manually from the binary layout defined in
 * each program's `instruction_args/` module.  This approach bypasses the
 * Codama-generated builders and avoids the discriminator mismatch that arises
 * when Codama emits 1-byte ordinal discriminators instead of the 8-byte
 * SHA256-derived values the on-chain programs require.
 *
 * Layout reference (tyche-core/src/instruction_args/create_competition.rs):
 *
 *   CreateCompetitionArgs (#[repr(C)], bytemuck::Pod)
 *   off  8..40  id                   [u8; 32]
 *   off 40      asset_type           u8
 *   off 41..48  _pad                 [u8; 7]
 *   off 48..56  start_time           i64 LE
 *   off 56..64  duration_secs        i64 LE
 *   off 64..72  soft_close_window    i64 LE
 *   off 72..80  soft_close_extension i64 LE
 *   off 80      max_soft_closes      u8
 *   off 81..88  _pad2                [u8; 7]
 *   off 88..96  reserve_price        u64 LE
 *   Total: 8 (disc) + 88 (args) = 96 bytes
 *
 * Devnet only.  Requires:
 *   AUTHORITY_KEYPAIR  — funded authority keypair file path
 *   TREASURY_ADDRESS   — treasury base58 address
 */

import { describe, it, expect, beforeAll } from 'vitest';
import {
  getAddressEncoder,
  AccountRole,
  type Address,
  type Instruction,
  type TransactionSigner,
  address,
} from '@solana/kit';
import { SYSTEM_PROGRAM_ADDRESS } from '@solana-program/system';
import {
  fetchCompetitionState,
  fetchMaybeCompetitionState,
  fetchProtocolConfig,
} from 'tyche-generated-core';
import { rpc, authority as authorityPromise, newCompetitionId } from '../setup/env.js';
import { requireFunds } from '../setup/airdrop.js';
import { sendAndConfirm, accountExists } from '../setup/helpers.js';
import {
  getCompetitionStatePda,
  getProtocolConfigPda,
  getPermissionPda,
  TYCHE_CORE_PROGRAM_ADDRESS,
  PHASE_SCHEDULED,
  PHASE_CANCELLED,
} from 'tyche-sdk';

// ── Discriminators (SHA256("global:<name>")[0..8]) ─────────────────────────────

const MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS =
  address('DELeGGvXp4MwQwxs5RqAxbARrYxwHXXNEH6xRkMxs2X2');


  
const CREATE_COMPETITION_DISC = new Uint8Array([110, 212, 234, 212, 118, 128, 158, 244]);
const CANCEL_COMPETITION_DISC = new Uint8Array([ 62,   4, 198,  98, 200,  41, 255,  72]);

// ── Asset type constants ───────────────────────────────────────────────────────

const ASSET_TYPE_NFT = 0;

// ── Raw instruction builders ───────────────────────────────────────────────────

const enc = getAddressEncoder();

/**
 * Build a `CreateCompetition` instruction from raw bytes.
 *
 * Accounts (must match CreateCompetitionAccounts::try_from order):
 *   [0] competition    writable
 *   [1] authority      readonly signer
 *   [2] payer          writable signer
 *   [3] system_program readonly
 *   [4] protocol_config readonly
 */
function buildCreateCompetitionIx(args: {
  competition:         Address;
  authority:           TransactionSigner;
  payer:               TransactionSigner;
  protocolConfig:      Address;
  id:                  Uint8Array;  // 32 bytes
  assetType:           number;
  startTime:           bigint;
  durationSecs:        bigint;
  softCloseWindow:     bigint;
  softCloseExtension:  bigint;
  maxSoftCloses:       number;
  reservePrice:        bigint;
}): Instruction {
  const data = new Uint8Array(96);
  const view = new DataView(data.buffer);
  let off = 0;

  data.set(CREATE_COMPETITION_DISC, off);          off += 8;  // discriminator
  data.set(args.id.slice(0, 32), off);             off += 32; // id
  view.setUint8(off, args.assetType);              off += 1;  // asset_type
  off += 7;                                                    // _pad
  view.setBigInt64(off, args.startTime, true);     off += 8;  // start_time
  view.setBigInt64(off, args.durationSecs, true);  off += 8;  // duration_secs
  view.setBigInt64(off, args.softCloseWindow, true);  off += 8; // soft_close_window
  view.setBigInt64(off, args.softCloseExtension, true); off += 8; // soft_close_extension
  view.setUint8(off, args.maxSoftCloses);          off += 1;  // max_soft_closes
  off += 7;                                                    // _pad2
  view.setBigUint64(off, args.reservePrice, true);             // reserve_price

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
 * Build a `CancelCompetition` instruction from raw bytes.
 *
 * No args — discriminator only (8 bytes).
 *
 * On the Scheduled path, permission / magicContext / magicProgram are passed
 * but never touched by the processor. Any valid addresses work.
 *
 * Accounts (must match CancelCompetitionAccounts::try_from order):
 *   [0] competition   writable
 *   [1] authority     readonly signer
 *   [2] permission    writable
 *   [3] magic_context writable
 *   [4] magic_program readonly
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

// ── Shared state ───────────────────────────────────────────────────────────────

let authority: TransactionSigner;

beforeAll(async () => {
  authority = await authorityPromise;
  await requireFunds(authority.address, 100_000_000n);
});

// ── ProtocolConfig (read-only — must already be initialised on devnet) ─────────

describe('ProtocolConfig', () => {
  it('exists on devnet', async () => {
    const [configAddress] = await getProtocolConfigPda();
    const exists = await accountExists(configAddress);
    expect(exists, `ProtocolConfig PDA ${configAddress} not found — run InitialiseProtocolConfig first`).toBe(true);
  });

  it('is fetchable and has expected structure', async () => {
    const [configAddress] = await getProtocolConfigPda();
    const config = await fetchProtocolConfig(rpc, configAddress);
    expect(config.data).toBeDefined();
    // fee_basis_points must be ≤ MAX_FEE_BASIS_POINTS (1000 = 10%)
    expect(Number(config.data.feeBasisPoints)).toBeLessThanOrEqual(1_000);
  });
});

// ── CreateCompetition ──────────────────────────────────────────────────────────

describe('CreateCompetition', () => {
  it('creates a CompetitionState PDA in Scheduled phase', async () => {
    const competitionId = newCompetitionId();
    const [competitionAddress] = await getCompetitionStatePda(authority.address, competitionId);
    const [protocolConfigAddress] = await getProtocolConfigPda();

    const idBytes = new Uint8Array(32);
    new DataView(idBytes.buffer).setBigUint64(0, competitionId, true);

    const now = BigInt(Math.floor(Date.now() / 1000)) + 60n; // +60 s buffer: avoid AuctionNotStarted on slow slots

    const ix = buildCreateCompetitionIx({
      competition:        competitionAddress,
      authority,
      payer:              authority,
      protocolConfig:     protocolConfigAddress,
      id:                 idBytes,
      assetType:          ASSET_TYPE_NFT,
      startTime:          now,
      durationSecs:       3_600n,
      softCloseWindow:    300n,
      softCloseExtension: 300n,
      maxSoftCloses:      5,
      reservePrice:       500_000_000n,
    });

    await sendAndConfirm([ix], authority);

    const exists = await accountExists(competitionAddress);
    expect(exists).toBe(true);

    const competition = await fetchCompetitionState(rpc, competitionAddress);
    expect(competition.data.phase).toBe(PHASE_SCHEDULED);
    expect(competition.data.reservePrice).toBe(500_000_000n);
    console.log('\\n[Demo] CompetitionState Account Created:', competitionAddress);
  });

  it('rejects a duplicate competition ID from the same authority', async () => {
    const competitionId = newCompetitionId();
    const [competitionAddress] = await getCompetitionStatePda(authority.address, competitionId);
    const [protocolConfigAddress] = await getProtocolConfigPda();

    const idBytes = new Uint8Array(32);
    new DataView(idBytes.buffer).setBigUint64(0, competitionId, true);

    const ix = buildCreateCompetitionIx({
      competition:        competitionAddress,
      authority,
      payer:              authority,
      protocolConfig:     protocolConfigAddress,
      id:                 idBytes,
      assetType:          ASSET_TYPE_NFT,
      startTime:          BigInt(Math.floor(Date.now() / 1000)) + 60n,
      durationSecs:       3_600n,
      softCloseWindow:    300n,
      softCloseExtension: 300n,
      maxSoftCloses:      5,
      reservePrice:       500_000_000n,
    });

    // First creation succeeds.
    await sendAndConfirm([ix], authority);

    // Second with the same PDA address fails — account already initialised.
    await expect(sendAndConfirm([ix], authority)).rejects.toThrow();
  });
});

// ── CancelCompetition ─────────────────────────────────────────────────────────

describe('CancelCompetition', () => {
  it('cancels a Scheduled competition', async () => {
    const competitionId = newCompetitionId();
    const [competitionAddress] = await getCompetitionStatePda(authority.address, competitionId);
    const [protocolConfigAddress] = await getProtocolConfigPda();

    const idBytes = new Uint8Array(32);
    new DataView(idBytes.buffer).setBigUint64(0, competitionId, true);

    const createIx = buildCreateCompetitionIx({
      competition:        competitionAddress,
      authority,
      payer:              authority,
      protocolConfig:     protocolConfigAddress,
      id:                 idBytes,
      assetType:          ASSET_TYPE_NFT,
      startTime:          BigInt(Math.floor(Date.now() / 1000)) + 60n,
      durationSecs:       3_600n,
      softCloseWindow:    300n,
      softCloseExtension: 300n,
      maxSoftCloses:      5,
      reservePrice:       500_000_000n,
    });
    await sendAndConfirm([createIx], authority);

    // permission is ignored on the Scheduled → Cancelled path.
    const [permission] = await getPermissionPda(authority.address);
    const cancelIx = buildCancelCompetitionIx({
      competition:  competitionAddress,
      authority,
      permission,
      magicContext: SYSTEM_PROGRAM_ADDRESS,
      magicProgram: address('DELeGGvXp4MwQwxs5RqAxbARrYxwHXXNEH6xRkMxs2X2'),
    });

    await sendAndConfirm([cancelIx], authority);

    const competition = await fetchMaybeCompetitionState(rpc, competitionAddress);
    expect(competition.exists).toBe(true);
    if (competition.exists) {
      expect(competition.data.phase).toBe(PHASE_CANCELLED);
      console.log('\\n[Demo] CompetitionState Canceled:', competitionAddress);
    }
  });
});
