/**
 * Full SDK end-to-end flow test.
 *
 * Exercises the complete Tyche sealed-bid auction lifecycle using only the
 * public SDK surface (`tyche-sdk`):
 *
 *   1. buildCreateAuctionTransaction  → create competition + auction
 *   2. buildActivateAuctionTransaction → delegate to MagicBlock PER
 *   3. buildPlaceBidTransaction        → place a bid (via PER Router)
 *   4. Settlement / cancel             → verify final state
 *
 * Phase 1 (create) runs without MagicBlock.
 * Phases 2–4 require MAGICBLOCK_VALIDATOR=1 in .env.test.
 *
 * Devnet required.  Requires:
 *   AUTHORITY_KEYPAIR  — funded authority keypair file path
 *   BIDDER1_KEYPAIR    — funded bidder keypair file path
 */

import { describe, it, expect, beforeAll } from 'vitest';
import { type TransactionSigner, type Address } from '@solana/kit';

import { rpc, authority as authorityPromise, bidder1 as bidder1Promise, newCompetitionId } from '../setup/env.js';
import { requireFunds } from '../setup/airdrop.js';
import { sendAndConfirm, sendAndConfirmWithBlockhash, getBlockhashForAccounts, accountExists } from '../setup/helpers.js';
import {
  buildCreateAuctionTransaction,
  buildActivateAuctionTransaction,
  buildPlaceBidTransaction,
  buildCancelAuctionTransaction,
  fetchDecodedCompetition,
  fetchDecodedAuction,
  getDelegationBufferPda,
  getDelegationRecordPda,
  getDelegationMetadataPda,
  getPermissionPda,
  MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS,
  MAGICBLOCK_PERMISSION_PROGRAM_ADDRESS,
  type MagicBlockActivateCompetitionAccounts,
  type MagicBlockDelegationAccounts,
} from 'tyche-sdk';

// ── Shared state ──────────────────────────────────────────────────────────────

let authority: TransactionSigner;
let bidder1:   TransactionSigner;

const DUMMY_MINT = '11111111111111111111111111111111' as Address;

// Devnet MagicBlock validator address (from MagicBlock documentation).
// Replace with the actual validator address for your devnet setup.
const DEVNET_VALIDATOR = (
  process.env['MAGICBLOCK_VALIDATOR_ADDRESS'] ?? 'LuzXEV3trGF4jQzpRzZaaTB9TqSwLkB7bpKQCQC7BAg'
) as Address;

beforeAll(async () => {
  [authority, bidder1] = await Promise.all([authorityPromise, bidder1Promise]);
  await Promise.all([
    requireFunds(authority.address, 100_000_000n),
    requireFunds(bidder1.address,   100_000_000n),
  ]);
});

// ── Phase 1: Create (no MagicBlock required) ──────────────────────────────────

describe('Phase 1 — create competition + auction', () => {
  it('creates both accounts and returns Scheduled phase', async () => {
    const id = newCompetitionId();
    const { competitionAddress, auctionStateAddress, instructions } =
      await buildCreateAuctionTransaction({
        authority,
        payer:           authority,
        competitionId:   id,
        startTime:       BigInt(Math.floor(Date.now() / 1000)) + 60n,
        durationSecs:    3_600n,
        reservePrice:    500_000_000n,
        assetMint:       DUMMY_MINT,
        minBidIncrement: 1_000_000n,
      });

    await sendAndConfirm(instructions, authority);

    const [comp, auction] = await Promise.all([
      fetchDecodedCompetition(rpc, competitionAddress),
      fetchDecodedAuction(rpc, auctionStateAddress),
    ]);

    expect(comp.phase).toBe('scheduled');
    expect(auction.competition).toBe(competitionAddress);
  });
});

// ── Cancel path (no MagicBlock required) ─────────────────────────────────────

describe('Cancel path — Scheduled → Cancelled', () => {
  it('cancels both competition and auction from Scheduled phase', async () => {
    const id = newCompetitionId();
    const { competitionAddress, auctionStateAddress, instructions: createIxs } =
      await buildCreateAuctionTransaction({
        authority,
        payer:         authority,
        competitionId: id,
        startTime:     BigInt(Math.floor(Date.now() / 1000)) + 60n,
        durationSecs:  3_600n,
        reservePrice:  500_000_000n,
        assetMint:     DUMMY_MINT,
      });

    await sendAndConfirm(createIxs, authority);

    // Cancel
    const { instructions: cancelIxs } = await buildCancelAuctionTransaction({
      authority,
      competitionAddress,
      auctionStateAddress,
      rentRecipient: authority.address,
    });

    await sendAndConfirm(cancelIxs, authority);

    const comp = await fetchDecodedCompetition(rpc, competitionAddress);
    expect(comp.phase).toBe('cancelled');

    // AuctionState should be closed after cancel
    const auctionExists = await accountExists(auctionStateAddress);
    expect(auctionExists).toBe(false);
  });
});

// ── Full lifecycle (requires MagicBlock) ─────────────────────────────────────

const hasMagicBlock = Boolean(process.env['MAGICBLOCK_VALIDATOR']);

describe.skipIf(!hasMagicBlock)('Full lifecycle — create → activate → bid → cancel', () => {
  it('activates a competition and places a bid via the MagicBlock Router', async () => {
    // ── Step 1: Create ────────────────────────────────────────────────────────
    const id = newCompetitionId();
    const { competitionAddress, auctionStateAddress, instructions: createIxs } =
      await buildCreateAuctionTransaction({
        authority,
        payer:           authority,
        competitionId:   id,
        startTime:       BigInt(Math.floor(Date.now() / 1000)) + 60n,
        durationSecs:    3_600n,
        reservePrice:    500_000_000n,
        assetMint:       DUMMY_MINT,
        minBidIncrement: 1_000_000n,
      });

    await sendAndConfirm(createIxs, authority);

    // ── Step 2: Derive delegation PDAs ───────────────────────────────────────
    const [
      [compBuffer],
      [compRecord],
      [compMetadata],
      [aucBuffer],
      [aucRecord],
      [aucMetadata],
      [permission],
    ] = await Promise.all([
      getDelegationBufferPda(competitionAddress),
      getDelegationRecordPda(competitionAddress),
      getDelegationMetadataPda(competitionAddress),
      getDelegationBufferPda(auctionStateAddress),
      getDelegationRecordPda(auctionStateAddress),
      getDelegationMetadataPda(auctionStateAddress),
      getPermissionPda(competitionAddress),
    ]);

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

    // ── Step 3: Activate (delegate both PDAs to PER) ─────────────────────────
    const { instructions: activateIxs } = buildActivateAuctionTransaction({
      authority,
      payer: authority,
      competitionAddress,
      auctionStateAddress,
      competitionDelegation,
      auctionDelegation,
      commitFrequencyMs: 1_000,
    });

    await sendAndConfirm(activateIxs, authority);

    // ── Step 4: Place bid via MagicBlock Router ───────────────────────────────
    const { instruction: bidIx, accounts: bidAccounts } = await buildPlaceBidTransaction({
      bidder:             bidder1,
      payer:              bidder1,
      competitionAddress,
      auctionStateAddress,
      amount:             600_000_000n, // 0.6 SOL > reservePrice (0.5 SOL)
    });

    const blockhash = await getBlockhashForAccounts(bidAccounts);
    await sendAndConfirmWithBlockhash([bidIx], bidder1, blockhash);

    // ── Step 5: Verify PER committed bid record ───────────────────────────────
    // After bidding on PER, the bid record is committed back to mainnet
    // periodically (commitFrequencyMs = 1 s).  Poll briefly.
    let bidRecordExists = false;
    const { getBidRecordPda } = await import('tyche-sdk');
    const [bidRecord] = await getBidRecordPda(auctionStateAddress, bidder1.address);

    for (let i = 0; i < 15; i++) {
      bidRecordExists = await accountExists(bidRecord);
      if (bidRecordExists) break;
      await new Promise(r => setTimeout(r, 1_000));
    }

    expect(bidRecordExists, `BidRecord ${bidRecord} not committed to mainnet within 15 s`).toBe(true);
  });
});
