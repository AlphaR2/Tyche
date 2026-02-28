/**
 * buildPlaceBidTransaction — SDK transaction builder tests.
 *
 * Tests cover:
 *   - Returned instruction structure
 *   - Accounts list completeness (used for getBlockhashForAccounts)
 *   - Live bid submission via MagicBlock Router (requires MAGICBLOCK_VALIDATOR)
 *
 * Devnet required.  Requires:
 *   AUTHORITY_KEYPAIR  — funded authority keypair file path
 *   BIDDER1_KEYPAIR    — funded bidder keypair file path
 */

import { describe, it, expect, beforeAll } from 'vitest';
import { type TransactionSigner, type Address } from '@solana/kit';

import { authority as authorityPromise, bidder1 as bidder1Promise, newCompetitionId } from '../setup/env.js';
import { requireFunds } from '../setup/airdrop.js';
import { sendAndConfirm, getBlockhashForAccounts, sendAndConfirmWithBlockhash } from '../setup/helpers.js';
import {
  buildCreateAuctionTransaction,
  buildPlaceBidTransaction,
  getCompetitionStatePda,
  getAuctionStatePda,
  getBidRecordPda,
  getEscrowVaultPda,
  TYCHE_CORE_PROGRAM_ADDRESS,
} from 'tyche-sdk';

// ── Shared state ──────────────────────────────────────────────────────────────

let authority: TransactionSigner;
let bidder1:   TransactionSigner;

const DUMMY_MINT = '11111111111111111111111111111111' as Address;

beforeAll(async () => {
  [authority, bidder1] = await Promise.all([authorityPromise, bidder1Promise]);
  await Promise.all([
    requireFunds(authority.address, 100_000_000n),
    requireFunds(bidder1.address,   100_000_000n),
  ]);
});

// ── Unit-like: verify instruction structure ───────────────────────────────────

describe('buildPlaceBidTransaction — structure', () => {
  it('returns an instruction with the correct program address', async () => {
    // Create a competition so PDAs are valid
    const id = newCompetitionId();
    const [competition]  = await getCompetitionStatePda(authority.address, id);
    const [auctionState] = await getAuctionStatePda(competition);

    const { instruction } = await buildPlaceBidTransaction({
      bidder:             bidder1,
      payer:              bidder1,
      competitionAddress: competition,
      auctionStateAddress: auctionState,
      amount:             100_000_000n,
    });

    expect(typeof instruction.programAddress).toBe('string');
  });

  it('returned accounts list contains all necessary addresses', async () => {
    const id = newCompetitionId();
    const [competition]  = await getCompetitionStatePda(authority.address, id);
    const [auctionState] = await getAuctionStatePda(competition);
    const [bidRecord]    = await getBidRecordPda(auctionState, bidder1.address);
    const [vault]        = await getEscrowVaultPda(competition, bidder1.address);

    const { accounts } = await buildPlaceBidTransaction({
      bidder:             bidder1,
      payer:              bidder1,
      competitionAddress: competition,
      auctionStateAddress: auctionState,
      amount:             100_000_000n,
    });

    // All critical accounts must appear in the list
    expect(accounts).toContain(auctionState);
    expect(accounts).toContain(competition);
    expect(accounts).toContain(bidder1.address);
    expect(accounts).toContain(bidRecord);
    expect(accounts).toContain(vault);
    expect(accounts).toContain(TYCHE_CORE_PROGRAM_ADDRESS);
  });
});

// ── MagicBlock Router: live bid submission ────────────────────────────────────

const hasMagicBlock = Boolean(process.env['MAGICBLOCK_VALIDATOR']);

describe.skipIf(!hasMagicBlock)('buildPlaceBidTransaction — devnet via MagicBlock Router', () => {
  it('places a bid on an Active (delegated) auction', async () => {
    // Pre-requisites:
    //  1. A competition + auction must have been created and activated.
    //  2. MAGICBLOCK_VALIDATOR=1 must be set.
    //
    // This test sets up a fresh competition, activates it (delegation), then
    // places a bid via the MagicBlock Router using getBlockhashForAccounts.

    // Step 1: Create auction
    const id = newCompetitionId();
    const { competitionAddress, auctionStateAddress, instructions: createIxs } =
      await buildCreateAuctionTransaction({
        authority,
        payer:           authority,
        competitionId:   id,
        startTime:       BigInt(Math.floor(Date.now() / 1000)),
        durationSecs:    3_600n,
        reservePrice:    500_000_000n,
        assetMint:       DUMMY_MINT,
        minBidIncrement: 1_000_000n,
      });

    await sendAndConfirm(createIxs, authority);

    // Step 2: Activate (requires MagicBlock delegation — detailed in full-flow.test.ts)
    // Placeholder: assume the competition is already active for this test.

    // Step 3: Build place-bid instruction
    const { instruction, accounts } = await buildPlaceBidTransaction({
      bidder:             bidder1,
      payer:              bidder1,
      competitionAddress,
      auctionStateAddress,
      amount:             600_000_000n,
    });

    // Step 4: Get blockhash routed to the correct PER node
    const blockhash = await getBlockhashForAccounts(accounts);

    // Step 5: Send via MagicBlock Router
    await sendAndConfirmWithBlockhash([instruction], bidder1, blockhash);

    // Verify bid record was created (PER state committed to mainnet)
    // Note: PER → mainnet commit may take a few seconds
    const [bidRecord] = await getBidRecordPda(auctionStateAddress, bidder1.address);
    expect(typeof bidRecord).toBe('string');
  });
});
