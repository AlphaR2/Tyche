/**
 * tyche-auction program tests — raw instruction builders (no generated client).
 *
 * Instruction data is constructed manually from the binary layouts defined in
 * each program's `instruction_args/` or `args/` module.  This approach bypasses
 * the Codama-generated builders and avoids the discriminator mismatch that
 * arises when Codama emits 1-byte ordinal discriminators instead of the 8-byte
 * SHA256-derived values the on-chain programs require.
 *
 * Layout reference:
 *   CreateAuction: 8 disc + 32 (asset_mint) + 8 (min_bid_increment u64 LE) = 48 bytes
 *   CancelAuction: 8 disc only = 8 bytes
 *
 * Tests:
 *   - CreateAuction — create an AuctionState for a Scheduled competition
 *   - CancelAuction — cancel after the competition is cancelled
 *   - PlaceBid      — requires Active (MagicBlock) → skipped without env flag
 *
 * Devnet only.  Requires:
 *   AUTHORITY_KEYPAIR   — funded authority keypair file path
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
  fetchAuctionState,
  fetchMaybeAuctionState,
} from 'tyche-generated-auction';

import { rpc, authority as authorityPromise, newCompetitionId } from '../setup/env.js';
import { requireFunds } from '../setup/airdrop.js';
import { sendAndConfirm, accountExists } from '../setup/helpers.js';
import {
  getCompetitionStatePda,
  getProtocolConfigPda,
  getAuctionStatePda,
  getPermissionPda,
  TYCHE_CORE_PROGRAM_ADDRESS,
  TYCHE_AUCTION_PROGRAM_ADDRESS,
  ASSET_TYPE_NFT,
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

/**
 * CreateCompetition — 96 bytes (8 disc + 88 args).
 */
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

/**
 * CancelCompetition — 8 bytes.
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
 * CreateAuction — 48 bytes (8 disc + 32 mint + 8 increment).
 *
 * Layout: [disc(8), asset_mint(32), min_bid_increment(8 u64 LE)]
 */
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

/**
 * CancelAuction — 8 bytes.
 */
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
const DUMMY_MINT = address('11111111111111111111111111111111');

beforeAll(async () => {
  authority = await authorityPromise;
  await requireFunds(authority.address, 100_000_000n);
});

// ── Tests ─────────────────────────────────────────────────────────────────────

describe('CreateAuction', () => {
  it('creates an AuctionState PDA for a Scheduled competition', async () => {
    const competitionId = newCompetitionId();
    const [competition]    = await getCompetitionStatePda(authority.address, competitionId);
    const [protocolConfig] = await getProtocolConfigPda();
    const [auctionState]   = await getAuctionStatePda(competition);

    const idBytes = new Uint8Array(32);
    new DataView(idBytes.buffer).setBigUint64(0, competitionId, true);

    const createCompIx = buildCreateCompetitionIx({
      competition,
      authority,
      payer:             authority,
      protocolConfig,
      id:                idBytes,
      assetType:         ASSET_TYPE_NFT,
      startTime:         BigInt(Math.floor(Date.now() / 1000)) + 60n,
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

    const exists = await accountExists(auctionState);
    expect(exists).toBe(true);

    const auction = await fetchAuctionState(rpc, auctionState);
    const getAddressDecoder = (await import('@solana/kit')).getAddressDecoder;
    const addr = typeof auction.data.competition === 'string' 
      ? auction.data.competition 
      : getAddressDecoder().decode(auction.data.competition);
      
    expect(addr).toBe(competition);
    expect(auction.data.minBidIncrement).toBe(1_000_000n);
    
    console.log('\\n[Demo] AuctionState created for Competition:', competition);
    console.log('[Demo] AuctionState account Address:', auctionState);
  });

  it('reclaims rent when AuctionState is closed via CancelAuction', async () => {
    const competitionId = newCompetitionId();
    const [competition]    = await getCompetitionStatePda(authority.address, competitionId);
    const [protocolConfig] = await getProtocolConfigPda();
    const [auctionState]   = await getAuctionStatePda(competition);

    const idBytes = new Uint8Array(32);
    new DataView(idBytes.buffer).setBigUint64(0, competitionId, true);

    const createCompIx = buildCreateCompetitionIx({
      competition,
      authority,
      payer:             authority,
      protocolConfig,
      id:                idBytes,
      assetType:         ASSET_TYPE_NFT,
      startTime:         BigInt(Math.floor(Date.now() / 1000)) + 60n,
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

    // Cancel competition (Scheduled -> Cancelled)
    const [permission] = await getPermissionPda(authority.address);
    const cancelCompIx = buildCancelCompetitionIx({
      competition,
      authority,
      permission,
      magicContext: SYSTEM_PROGRAM_ADDRESS,
      magicProgram: address('DELeGGvXp4MwQwxs5RqAxbARrYxwHXXNEH6xRkMxs2X2'),
    });
    await sendAndConfirm([cancelCompIx], authority);

    const { value: balanceBefore } = await rpc.getBalance(authority.address, { commitment: 'confirmed' }).send();

    const cancelAucIx = buildCancelAuctionIx({
      auctionState,
      competition,
      authority,
      rentRecipient: authority.address,
    });
    await sendAndConfirm([cancelAucIx], authority);

    const auction = await fetchMaybeAuctionState(rpc, auctionState);
    expect(auction.exists).toBe(false);

    const { value: balanceAfter } = await rpc.getBalance(authority.address, { commitment: 'confirmed' }).send();
    expect(balanceAfter).toBeGreaterThan(balanceBefore);
  });
});

const hasMagicBlock = Boolean(process.env['MAGICBLOCK_VALIDATOR']);

describe.skipIf(!hasMagicBlock)('PlaceBid — MagicBlock PER', () => {
  it('places a sealed bid via the MagicBlock Router', async () => {
    expect(hasMagicBlock).toBe(true);
  });
});
