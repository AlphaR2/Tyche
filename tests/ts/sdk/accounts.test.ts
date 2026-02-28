/**
 * SDK account fetcher tests.
 *
 * Verifies that `fetchDecodedCompetition` and `fetchDecodedAuction` correctly
 * decode on-chain accounts created via the raw instruction builders.
 *
 * Devnet required.  Requires:
 *   AUTHORITY_KEYPAIR  — funded authority keypair file path
 */

import { describe, it, expect, beforeAll } from 'vitest';
import {
  getCreateCompetitionInstruction,
} from 'tyche-generated-core';
import {
  getCreateAuctionInstruction,
} from 'tyche-generated-auction';
import { getAddressEncoder, type TransactionSigner, type Address } from '@solana/kit';

import { rpc, authority as authorityPromise, newCompetitionId } from '../setup/env.js';
import { requireFunds } from '../setup/airdrop.js';
import { sendAndConfirm } from '../setup/helpers.js';
import {
  getCompetitionStatePda,
  getProtocolConfigPda,
  getAuctionStatePda,
  fetchDecodedCompetition,
  fetchDecodedAuction,
  ASSET_TYPE_NFT,
  PHASE_SCHEDULED,
} from 'tyche-sdk';

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

    const idBytes = new Uint8Array(8);
    new DataView(idBytes.buffer).setBigUint64(0, id, true);

    const now = BigInt(Math.floor(Date.now() / 1000));

    const ix = getCreateCompetitionInstruction({
      competition,
      authority,
      payer: authority,
      protocolConfig,
      id:               idBytes,
      assetType:        ASSET_TYPE_NFT,
      pad:              new Uint8Array(6),
      startTime:        now,
      durationSecs:     7_200n,
      softCloseWindow:  300n,
      softCloseExtension: 300n,
      maxSoftCloses:    5,
      pad2:             new Uint8Array(2),
      reservePrice:     1_500_000_000n, // 1.5 SOL
    });

    await sendAndConfirm([ix], authority);

    const decoded = await fetchDecodedCompetition(rpc, competition);

    expect(decoded.phase).toBe(PHASE_SCHEDULED);
    expect(decoded.reservePrice).toBe(1_500_000_000n);
    expect(typeof decoded.authority).toBe('string');
  });

  it('throws if the account does not exist', async () => {
    // Derive a PDA that was never created
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

    const idBytes = new Uint8Array(8);
    new DataView(idBytes.buffer).setBigUint64(0, id, true);

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
      payer:           authority,
      assetMint:       addrEnc.encode(DUMMY_MINT),
      minBidIncrement: 2_000_000n,
    });

    await sendAndConfirm([createCompIx, createAucIx], authority);

    const decoded = await fetchDecodedAuction(rpc, auctionState);

    expect(decoded.competition).toBe(competition);
    expect(decoded.minBidIncrement).toBe(2_000_000n);
    expect(typeof decoded.currentWinner).toBe('string');
  });
});
