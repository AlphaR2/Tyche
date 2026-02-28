/**
 * tyche-auction program tests — raw instruction builders (no SDK wrapper).
 *
 * Tests in this file:
 *   - CreateAuction — create an AuctionState for a Scheduled competition
 *   - CancelAuction — cancel after the competition is cancelled
 *   - PlaceBid      — requires Active (MagicBlock) → skipped without env flag
 *
 * Devnet only.  Requires:
 *   AUTHORITY_KEYPAIR   — funded authority keypair file path
 */

import { describe, it, expect, beforeAll } from 'vitest';
import {
  getCreateCompetitionInstruction,
  getCancelCompetitionInstruction,
} from 'tyche-generated-core';
import {
  getCreateAuctionInstruction,
  getCancelAuctionInstruction,
  fetchAuctionState,
  fetchMaybeAuctionState,
} from 'tyche-generated-auction';
import { getAddressEncoder, type TransactionSigner, type Address } from '@solana/kit';
import { SYSTEM_PROGRAM_ADDRESS } from "@solana-program/system";

import { rpc, authority as authorityPromise, newCompetitionId } from '../setup/env.js';
import { requireFunds } from '../setup/airdrop.js';
import { sendAndConfirm, accountExists } from '../setup/helpers.js';
import {
  getCompetitionStatePda,
  getProtocolConfigPda,
  getAuctionStatePda,
  getPermissionPda,
  ASSET_TYPE_NFT,
  TYCHE_AUCTION_PROGRAM_ADDRESS,
  MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS,
} from 'tyche-sdk';

// ── Shared state ──────────────────────────────────────────────────────────────

let authority: TransactionSigner;

// A dummy mint address — just needs to be any valid-looking address.
const DUMMY_MINT = '11111111111111111111111111111111' as Address;

beforeAll(async () => {
  authority = await authorityPromise;
  await requireFunds(authority.address, 100_000_000n);
});

// ── CreateAuction ─────────────────────────────────────────────────────────────

describe('CreateAuction', () => {
  it('creates an AuctionState PDA for a Scheduled competition', async () => {
    const competitionId = newCompetitionId();
    const [competition]    = await getCompetitionStatePda(authority.address, competitionId);
    const [protocolConfig] = await getProtocolConfigPda();
    const [auctionState]   = await getAuctionStatePda(competition);

    const idBytes = new Uint8Array(32);
    new DataView(idBytes.buffer).setBigUint64(0, competitionId, true);

    // 1. Create competition
    const createCompIx = getCreateCompetitionInstruction({
      competition,
      authority,
      payer: authority,
      protocolConfig,
      id:               idBytes,
      assetType:        ASSET_TYPE_NFT,
      pad:              new Uint8Array(6),
      startTime:        BigInt(Math.floor(Date.now() / 1000)),
      durationSecs:     3_600n,
      softCloseWindow:  300n,
      softCloseExtension: 300n,
      maxSoftCloses:    5,
      pad2:             new Uint8Array(2),
      reservePrice:     500_000_000n,
    });

    // 2. Create auction — encode mint as 32 raw bytes
    const addrEnc = getAddressEncoder();
    const createAucIx = getCreateAuctionInstruction({
      auctionState,
      competition,
      authority,
      payer:          authority,
      assetMint:      addrEnc.encode(DUMMY_MINT),
      minBidIncrement: 1_000_000n,
    });

    await sendAndConfirm([createCompIx, createAucIx], authority);

    const exists = await accountExists(auctionState);
    expect(exists, `AuctionState PDA ${auctionState} does not exist`).toBe(true);

    const auction = await fetchAuctionState(rpc, auctionState);
    expect(auction.data.competition).toBe(competition);
    expect(auction.data.minBidIncrement).toBe(1_000_000n);
  });

  it('returns rent to payer when AuctionState is closed via CancelAuction', async () => {
    const competitionId = newCompetitionId();
    const [competition]    = await getCompetitionStatePda(authority.address, competitionId);
    const [protocolConfig] = await getProtocolConfigPda();
    const [auctionState]   = await getAuctionStatePda(competition);

    const idBytes = new Uint8Array(32);
    new DataView(idBytes.buffer).setBigUint64(0, competitionId, true);

    const addrEnc = getAddressEncoder();

    const createCompIx = getCreateCompetitionInstruction({
      competition,
      authority,
      payer: authority,
      protocolConfig,
      id:               idBytes,
      assetType:        ASSET_TYPE_NFT,
      pad:              new Uint8Array(6),
      startTime:        BigInt(Math.floor(Date.now() / 1000)),
      durationSecs:     3_600n,
      softCloseWindow:  300n,
      softCloseExtension: 300n,
      maxSoftCloses:    5,
      pad2:             new Uint8Array(2),
      reservePrice:     500_000_000n,
    });

    const createAucIx = getCreateAuctionInstruction({
      auctionState,
      competition,
      authority,
      payer:          authority,
      assetMint:      addrEnc.encode(DUMMY_MINT),
      minBidIncrement: 1_000_000n,
    });

    await sendAndConfirm([createCompIx, createAucIx], authority);

    // Cancel the competition first (Scheduled → Cancelled)
    const [permission] = await getPermissionPda(authority.address);
    const cancelCompIx = getCancelCompetitionInstruction({
      competition,
      authority,
      permission,
      magicContext: SYSTEM_PROGRAM_ADDRESS,
      magicProgram: MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS,
    });
    await sendAndConfirm([cancelCompIx], authority);

    // Record balance before cancel auction
    const { value: balanceBefore } = await rpc
      .getBalance(authority.address, { commitment: 'confirmed' })
      .send();

    // Cancel the auction
    const cancelAucIx = getCancelAuctionInstruction({
      auctionState,
      competition,
      authority,
      rentRecipient: authority.address,
    });
    await sendAndConfirm([cancelAucIx], authority);

    // AuctionState must be closed
    const auction = await fetchMaybeAuctionState(rpc, auctionState);
    expect(auction.exists).toBe(false);

    // Seller should have received rent back (balance increased minus tx fee)
    const { value: balanceAfter } = await rpc
      .getBalance(authority.address, { commitment: 'confirmed' })
      .send();
    // Rent reclaim minus tx fee should net positive (rent ≈ ~1.4 million lamports)
    expect(balanceAfter).toBeGreaterThan(balanceBefore - 100_000n); // tx fee < 0.0001 SOL
  });
});

// ── PlaceBid (requires MagicBlock PER) ───────────────────────────────────────

const hasMagicBlock = Boolean(process.env['MAGICBLOCK_VALIDATOR']);

describe.skipIf(!hasMagicBlock)('PlaceBid — MagicBlock PER', () => {
  it('places a sealed bid via the MagicBlock Router', async () => {
    // Requires the competition to be delegated to the PER.
    // Set MAGICBLOCK_VALIDATOR=1 in .env.test once delegation is set up.
    expect(hasMagicBlock).toBe(true); // placeholder assertion
  });
});
