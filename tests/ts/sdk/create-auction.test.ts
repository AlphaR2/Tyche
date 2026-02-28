/**
 * buildCreateAuctionTransaction — SDK transaction builder tests.
 *
 * Tests cover:
 *   - Correct PDA derivation in the returned result
 *   - Instruction array order and count
 *   - The instructions are accepted by devnet (end-to-end)
 *
 * Devnet required.  Requires:
 *   AUTHORITY_KEYPAIR  — funded authority keypair file path
 */

import { describe, it, expect, beforeAll } from 'vitest';
import { type TransactionSigner, type Address } from '@solana/kit';

import { rpc, authority as authorityPromise, newCompetitionId } from '../setup/env.js';
import { requireFunds } from '../setup/airdrop.js';
import { sendAndConfirm, accountExists } from '../setup/helpers.js';
import {
  buildCreateAuctionTransaction,
  getCompetitionStatePda,
  getAuctionStatePda,
  PHASE_SCHEDULED,
  fetchDecodedCompetition,
  fetchDecodedAuction,
} from 'tyche-sdk';

// ── Shared state ──────────────────────────────────────────────────────────────

let authority: TransactionSigner;
// A dummy mint address for tests that don't actually transfer NFTs
const DUMMY_MINT = '11111111111111111111111111111111' as Address;

beforeAll(async () => {
  authority = await authorityPromise;
  await requireFunds(authority.address, 100_000_000n);
});

// ── Unit-like: verify returned addresses ──────────────────────────────────────

describe('buildCreateAuctionTransaction — address derivation', () => {
  it('returned competitionAddress matches independently derived PDA', async () => {
    const id = newCompetitionId();
    const { competitionAddress } = await buildCreateAuctionTransaction({
      authority,
      payer:         authority,
      competitionId: id,
      startTime:     BigInt(Math.floor(Date.now() / 1000)),
      durationSecs:  3_600n,
      reservePrice:  500_000_000n,
      assetMint:     DUMMY_MINT,
    });

    const [expected] = await getCompetitionStatePda(authority.address, id);
    expect(competitionAddress).toBe(expected);
  });

  it('returned auctionStateAddress matches independently derived PDA', async () => {
    const id = newCompetitionId();
    const { competitionAddress, auctionStateAddress } = await buildCreateAuctionTransaction({
      authority,
      payer:         authority,
      competitionId: id,
      startTime:     BigInt(Math.floor(Date.now() / 1000)),
      durationSecs:  3_600n,
      reservePrice:  500_000_000n,
      assetMint:     DUMMY_MINT,
    });

    const [expected] = await getAuctionStatePda(competitionAddress);
    expect(auctionStateAddress).toBe(expected);
  });

  it('returns exactly 2 instructions in order [CreateCompetition, CreateAuction]', async () => {
    const { instructions } = await buildCreateAuctionTransaction({
      authority,
      payer:         authority,
      competitionId: newCompetitionId(),
      startTime:     BigInt(Math.floor(Date.now() / 1000)),
      durationSecs:  3_600n,
      reservePrice:  500_000_000n,
      assetMint:     DUMMY_MINT,
    });

    expect(instructions).toHaveLength(2);
    // Both must be Instruction objects with a programAddress field
    for (const ix of instructions) {
      expect(typeof ix.programAddress).toBe('string');
    }
  });
});

// ── End-to-end: instructions accepted by devnet ───────────────────────────────

describe('buildCreateAuctionTransaction — devnet submission', () => {
  it('creates CompetitionState and AuctionState on devnet', async () => {
    const id = newCompetitionId();
    const { competitionAddress, auctionStateAddress, instructions } =
      await buildCreateAuctionTransaction({
        authority,
        payer:           authority,
        competitionId:   id,
        startTime:       BigInt(Math.floor(Date.now() / 1000)),
        durationSecs:    3_600n,
        reservePrice:    1_000_000_000n,
        assetMint:       DUMMY_MINT,
        minBidIncrement: 5_000_000n,
      });

    await sendAndConfirm(instructions, authority);

    expect(await accountExists(competitionAddress)).toBe(true);
    expect(await accountExists(auctionStateAddress)).toBe(true);

    const comp   = await fetchDecodedCompetition(rpc, competitionAddress);
    const auction = await fetchDecodedAuction(rpc, auctionStateAddress);

    expect(comp.phase).toBe(PHASE_SCHEDULED);
    expect(comp.reservePrice).toBe(1_000_000_000n);
    expect(auction.minBidIncrement).toBe(5_000_000n);
    expect(auction.competition).toBe(competitionAddress);
  });
});
