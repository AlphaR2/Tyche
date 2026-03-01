/**
 * SDK account fetcher tests.
 *
 * Verifies that `fetchDecodedCompetition` and `fetchDecodedAuction` correctly
 * decode on-chain accounts created via raw instruction builders.
 *
 * Uses raw instruction builders (not the Codama-generated client) to avoid
 * discriminator mismatches (generated client emits 1-byte ordinals; on-chain
 * programs expect 8-byte SHA256-derived discriminators).
 *
 * Devnet required.  Requires:
 *   AUTHORITY_KEYPAIR  — funded authority keypair file path
 */

import { describe, it, expect, beforeAll } from 'vitest';
import {
  getAddressEncoder,
  AccountRole,
  type TransactionSigner,
  type Address,
  type Instruction,
} from '@solana/kit';
import { SYSTEM_PROGRAM_ADDRESS } from '@solana-program/system';

import { rpc, authority as authorityPromise, newCompetitionId } from '../setup/env.js';
import { requireFunds } from '../setup/airdrop.js';
import { sendAndConfirm } from '../setup/helpers.js';
import {
  getCompetitionStatePda,
  getProtocolConfigPda,
  getAuctionStatePda,
  fetchDecodedCompetition,
  fetchDecodedAuction,
  TYCHE_CORE_PROGRAM_ADDRESS,
  TYCHE_AUCTION_PROGRAM_ADDRESS,
  ASSET_TYPE_NFT,
} from 'tyche-sdk';

// ── Discriminators (SHA256("global:<name>")[0..8]) ─────────────────────────────

const CREATE_COMPETITION_DISC = new Uint8Array([110, 212, 234, 212, 118, 128, 158, 244]);
const CREATE_AUCTION_DISC     = new Uint8Array([234,   6, 201, 246,  47, 219, 176, 107]);

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

  data.set(CREATE_COMPETITION_DISC, off);               off += 8;
  data.set(args.id.slice(0, 32), off);                  off += 32;
  view.setUint8(off, args.assetType);                   off += 1;
  off += 7;
  view.setBigInt64(off, args.startTime, true);          off += 8;
  view.setBigInt64(off, args.durationSecs, true);       off += 8;
  view.setBigInt64(off, args.softCloseWindow, true);    off += 8;
  view.setBigInt64(off, args.softCloseExtension, true); off += 8;
  view.setUint8(off, args.maxSoftCloses);               off += 1;
  off += 7;
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

// ── Shared state ──────────────────────────────────────────────────────────────

let authority: TransactionSigner;
const DUMMY_MINT = '11111111111111111111111111111111' as Address;

beforeAll(async () => {
  authority = await authorityPromise;
  await requireFunds(authority.address, 100_000_000n);
});

// ── fetchDecodedCompetition ───────────────────────────────────────────────────

describe('fetchDecodedCompetition', () => {
  it('returns a decoded competition matching on-chain state', async () => {
    const id = newCompetitionId();
    const [competition]    = await getCompetitionStatePda(authority.address, id);
    const [protocolConfig] = await getProtocolConfigPda();

    const idBytes = new Uint8Array(32);
    new DataView(idBytes.buffer).setBigUint64(0, id, true);

    const ix = buildCreateCompetitionIx({
      competition,
      authority,
      payer:              authority,
      protocolConfig,
      id:                 idBytes,
      assetType:          ASSET_TYPE_NFT,
      startTime:          BigInt(Math.floor(Date.now() / 1000)) + 60n,
      durationSecs:       7_200n,
      softCloseWindow:    300n,
      softCloseExtension: 300n,
      maxSoftCloses:      5,
      reservePrice:       1_500_000_000n,
    });

    await sendAndConfirm([ix], authority);

    const decoded = await fetchDecodedCompetition(rpc, competition);

    // fetchDecodedCompetition returns phase as a human-readable string
    expect(decoded.phase).toBe('scheduled');
    expect(decoded.reservePrice).toBe(1_500_000_000n);
    expect(typeof decoded.authority).toBe('string');
  });

  it('throws if the account does not exist', async () => {
    const [fakeAddr] = await getCompetitionStatePda(authority.address, BigInt(Date.now()) + 999_999n);
    await expect(fetchDecodedCompetition(rpc, fakeAddr)).rejects.toThrow();
  });
});

// ── fetchDecodedAuction ───────────────────────────────────────────────────────

describe('fetchDecodedAuction', () => {
  it('returns a decoded auction matching on-chain state', async () => {
    const id = newCompetitionId();
    const [competition]    = await getCompetitionStatePda(authority.address, id);
    const [protocolConfig] = await getProtocolConfigPda();
    const [auctionState]   = await getAuctionStatePda(competition);

    const idBytes = new Uint8Array(32);
    new DataView(idBytes.buffer).setBigUint64(0, id, true);

    const createCompIx = buildCreateCompetitionIx({
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

    const createAucIx = buildCreateAuctionIx({
      auctionState,
      competition,
      authority,
      payer:           authority,
      assetMint:       DUMMY_MINT,
      minBidIncrement: 2_000_000n,
    });

    await sendAndConfirm([createCompIx, createAucIx], authority);

    const decoded = await fetchDecodedAuction(rpc, auctionState);

    expect(decoded.competition).toBe(competition);
    expect(decoded.minBidIncrement).toBe(2_000_000n);
    // currentWinner is null (zero address) until settled; null means no winner yet
    expect(decoded.currentWinner).toBeNull();
  });
});
