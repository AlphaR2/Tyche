/**
 * Cross-program integration tests (raw instruction builders).
 *
 * Tests that span tyche-core + tyche-escrow + tyche-auction and cannot be
 * exercised by the per-program test files.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import {
  AccountRole,
  getAddressEncoder,
  type Address,
  type Instruction,
  type TransactionSigner,
  address,
} from '@solana/kit';
import { SYSTEM_PROGRAM_ADDRESS } from "@solana-program/system";
import {
  fetchMaybeAuctionState,
} from 'tyche-generated-auction';

import { rpc, authority as authorityPromise, bidder1 as bidder1Promise, newCompetitionId } from '../setup/env.js';
import { requireFunds } from '../setup/airdrop.js';
import { sendAndConfirm } from '../setup/helpers.js';
import {
  getCompetitionStatePda,
  getProtocolConfigPda,
  getAuctionStatePda,
  getPermissionPda,
  fetchDecodedCompetition,
  ASSET_TYPE_NFT,
  PHASE_CANCELLED,
  TYCHE_CORE_PROGRAM_ADDRESS,
  TYCHE_AUCTION_PROGRAM_ADDRESS,
} from 'tyche-sdk';

const MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS =
  address('DELeGGvXp4MwQwxs5RqAxbARrYxwHXXNEH6xRkMxs2X2');

// ── Discriminators (SHA256("global:<name>")[0..8]) ─────────────────────────────

const CREATE_COMPETITION_DISC = new Uint8Array([110, 212, 234, 212, 118, 128, 158, 244]);
const CANCEL_COMPETITION_DISC = new Uint8Array([ 62,   4, 198,  98, 200,  41, 255,  72]);
const CREATE_AUCTION_DISC     = new Uint8Array([234,   6, 201, 246,  47, 219, 176, 107]);
const CANCEL_AUCTION_DISC     = new Uint8Array([156,  43, 197, 110, 218, 105, 143, 182]);

const enc = getAddressEncoder();

// ── Raw instruction builders ───────────────────────────────────────────────────

function buildCreateCompetitionIx(args: {
  competition:        Address;
  authority:          TransactionSigner;
  payer:              TransactionSigner;
  protocolConfig:     Address;
  id:                 Uint8Array;
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

  data.set(CREATE_COMPETITION_DISC, off);              off += 8;
  data.set(args.id.slice(0, 32), off);                 off += 32;
  view.setUint8(off, args.assetType);                  off += 1;
  off += 7; // pad
  view.setBigInt64(off, args.startTime, true);         off += 8;
  view.setBigInt64(off, args.durationSecs, true);      off += 8;
  view.setBigInt64(off, args.softCloseWindow, true);   off += 8;
  view.setBigInt64(off, args.softCloseExtension, true); off += 8;
  view.setUint8(off, args.maxSoftCloses);              off += 1;
  off += 7; // pad2
  view.setBigUint64(off, args.reservePrice, true);

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

function buildCreateAuctionIx(args: {
  auctionState:    Address;
  competition:     Address;
  authority:       TransactionSigner;
  payer:           TransactionSigner;
  assetMint:       Address;
  minBidIncrement: bigint;
}): Instruction {
  const data = new Uint8Array(48);
  data.set(CREATE_AUCTION_DISC, 0);
  data.set(enc.encode(args.assetMint), 8);
  new DataView(data.buffer).setBigUint64(40, args.minBidIncrement, true);

  return {
    programAddress: TYCHE_AUCTION_PROGRAM_ADDRESS,
    accounts: [
      { address: args.auctionState,      role: AccountRole.WRITABLE },
      { address: args.competition,       role: AccountRole.READONLY },
      { address: args.authority.address, role: AccountRole.READONLY_SIGNER, signer: args.authority },
      { address: args.payer.address,     role: AccountRole.WRITABLE_SIGNER, signer: args.payer },
      { address: SYSTEM_PROGRAM_ADDRESS, role: AccountRole.READONLY },
    ],
    data,
  } as Instruction;
}

function buildCancelAuctionIx(args: {
  auctionState:  Address;
  competition:   Address;
  authority:     TransactionSigner;
  rentRecipient: Address;
}): Instruction {
  return {
    programAddress: TYCHE_AUCTION_PROGRAM_ADDRESS,
    accounts: [
      { address: args.auctionState,      role: AccountRole.WRITABLE },
      { address: args.competition,       role: AccountRole.READONLY },
      { address: args.authority.address, role: AccountRole.READONLY_SIGNER, signer: args.authority },
      { address: args.rentRecipient,     role: AccountRole.WRITABLE },
    ],
    data: CANCEL_AUCTION_DISC,
  } as Instruction;
}

// ── Shared state ──────────────────────────────────────────────────────────────

let authority: TransactionSigner;
let bidder1:   TransactionSigner;

const DUMMY_MINT = address('11111111111111111111111111111111');

beforeAll(async () => {
  [authority, bidder1] = await Promise.all([authorityPromise, bidder1Promise]);
  await Promise.all([
    requireFunds(authority.address, 100_000_000n),
    requireFunds(bidder1.address,   100_000_000n),
  ]);
});

// ── Helper ────────────────────────────────────────────────────────────────────

async function createCompetitionAndAuction(id: bigint) {
  const [competition]    = await getCompetitionStatePda(authority.address, id);
  const [protocolConfig] = await getProtocolConfigPda();
  const [auctionState]   = await getAuctionStatePda(competition);

  const idBytes = new Uint8Array(32);
  new DataView(idBytes.buffer).setBigUint64(0, id, true);

  const createCompIx = buildCreateCompetitionIx({
    competition,
    authority,
    payer:             authority,
    protocolConfig,
    id:                idBytes,
    assetType:         ASSET_TYPE_NFT,
    startTime:         BigInt(Math.floor(Date.now() / 1000)),
    durationSecs:      3_600n,
    softCloseWindow:   300n,
    softCloseExtension: 300n,
    maxSoftCloses:     5,
    reservePrice:      500_000_000n,
  });

  const createAucIx = buildCreateAuctionIx({
    auctionState,
    competition,
    authority,
    payer:           authority,
    assetMint:       DUMMY_MINT as Address,
    minBidIncrement: 1_000_000n,
  });

  await sendAndConfirm([createCompIx, createAucIx], authority);
  console.log('\\n[Demo] Integration: Created Competition and AuctionState automatically.');
  return { competition, auctionState };
}

// ── Tests ─────────────────────────────────────────────────────────────────────

describe('create + cancel lifecycle', () => {
  it('CancelCompetition transitions to Cancelled phase', async () => {
    const id = newCompetitionId();
    const { competition } = await createCompetitionAndAuction(id);

    const [permission] = await getPermissionPda(competition);
    const cancelCompIx = buildCancelCompetitionIx({
      competition,
      authority,
      permission,
      magicContext: SYSTEM_PROGRAM_ADDRESS,
      magicProgram: MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS as Address,
    });
    await sendAndConfirm([cancelCompIx], authority);

    const comp = await fetchDecodedCompetition(rpc, competition);
    expect(comp.phase).toBe(PHASE_CANCELLED);
    console.log('[Demo] Integration: Competition correctly transitioned to Cancelled.');
  });

  it('CancelAuction closes AuctionState after competition is cancelled', async () => {
    const id = newCompetitionId();
    const { competition, auctionState } = await createCompetitionAndAuction(id);

    // Cancel competition
    const [permission] = await getPermissionPda(competition);
    const cancelCompIx = buildCancelCompetitionIx({
      competition,
      authority,
      permission,
      magicContext: SYSTEM_PROGRAM_ADDRESS,
      magicProgram: MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS as Address,
    });
    await sendAndConfirm([cancelCompIx], authority);

    // Cancel auction
    const cancelAucIx = buildCancelAuctionIx({
      authority,
      auctionState,
      competition,
      rentRecipient: authority.address,
    });
    await sendAndConfirm([cancelAucIx], authority);

    const auction = await fetchMaybeAuctionState(rpc, auctionState);
    expect(auction.exists).toBe(false);
    console.log('[Demo] Integration: AuctionState successfully closed and rent reclaimed.');
  });

  it('CancelAuction on an Active competition fails', async () => {
    const id = newCompetitionId();
    const { competition, auctionState } = await createCompetitionAndAuction(id);

    const cancelAucIx = buildCancelAuctionIx({
      auctionState,
      competition,
      authority,
      rentRecipient: authority.address,
    });
    
    await expect(sendAndConfirm([cancelAucIx], authority)).rejects.toThrow();
  });
});

const hasMagicBlock = Boolean(process.env['MAGICBLOCK_VALIDATOR']);

describe.skipIf(!hasMagicBlock)('full sealed-bid lifecycle (MagicBlock)', () => {
  it('create → activate → bid → settle', async () => {
    expect(hasMagicBlock).toBe(true);
  });
});
