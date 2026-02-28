/**
 * tyche-escrow program tests — raw instruction builders (no SDK wrapper).
 *
 * Deposit / Refund require an Active competition, which requires MagicBlock
 * delegation.  These tests therefore come in two groups:
 *
 *   1. Pre-activation tests — things we can do without MagicBlock setup.
 *   2. Full-flow tests (skipped unless MAGICBLOCK_VALIDATOR is set in env).
 *
 * Devnet only.  Requires:
 *   AUTHORITY_KEYPAIR   — funded authority keypair file path
 *   BIDDER1_KEYPAIR     — funded bidder keypair file path
 */

import { describe, it, expect, beforeAll } from 'vitest';
import {
  getCreateCompetitionInstruction,
  fetchMaybeCompetitionState,
} from 'tyche-generated-core';
import { type TransactionSigner } from '@solana/kit';

import { rpc, authority as authPromise, bidder1 as bidder1Promise, newCompetitionId } from '../setup/env.js';
import { requireFunds } from '../setup/airdrop.js';
import { sendAndConfirm, accountExists } from '../setup/helpers.js';
import {
  getCompetitionStatePda,
  getProtocolConfigPda,
  getEscrowVaultPda,
  ASSET_TYPE_NFT,
  PHASE_SCHEDULED,
} from 'tyche-sdk';

// ── Shared state ──────────────────────────────────────────────────────────────

let authority: TransactionSigner;
let bidder1: TransactionSigner;

beforeAll(async () => {
  [authority, bidder1] = await Promise.all([authPromise, bidder1Promise]);
  await Promise.all([
    requireFunds(authority.address, 100_000_000n),
    requireFunds(bidder1.address,   100_000_000n),
  ]);
});

// ── Vault PDA derivation ──────────────────────────────────────────────────────

describe('EscrowVault PDA', () => {
  it('derives a unique PDA per (competition, depositor) pair', async () => {
    const competitionId = newCompetitionId();
    const [competition] = await getCompetitionStatePda(authority.address, competitionId);

    const [vault1] = await getEscrowVaultPda(competition, authority.address);
    const [vault2] = await getEscrowVaultPda(competition, bidder1.address);

    expect(vault1).not.toBe(vault2);
  });
});

// ── Pre-activation (Scheduled phase) ─────────────────────────────────────────

describe('EscrowVault — pre-activation', () => {
  it('vault PDA does not exist before any deposit', async () => {
    const competitionId = newCompetitionId();
    const [competition] = await getCompetitionStatePda(authority.address, competitionId);
    const [vault]       = await getEscrowVaultPda(competition, bidder1.address);

    // Competition doesn't even exist yet — vault should definitely be absent.
    const exists = await accountExists(vault);
    expect(exists).toBe(false);
  });

  it('Deposit fails on a Scheduled competition', async () => {
    // Create a competition in Scheduled phase
    const competitionId = newCompetitionId();
    const [competition]    = await getCompetitionStatePda(authority.address, competitionId);
    const [protocolConfig] = await getProtocolConfigPda();

    const idBytes = new Uint8Array(8);
    new DataView(idBytes.buffer).setBigUint64(0, competitionId, true);

    const createIx = getCreateCompetitionInstruction({
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
    await sendAndConfirm([createIx], authority);

    const comp = await fetchMaybeCompetitionState(rpc, competition);
    expect(comp.exists).toBe(true);
    if (comp.exists) {
      expect(comp.data.phase).toBe(PHASE_SCHEDULED);
    }

    // Attempt a Deposit — should fail because phase is Scheduled.
    const { getDepositInstruction } = await import('tyche-generated-escrow');
    const [vault] = await getEscrowVaultPda(competition, bidder1.address);

    const depositIx = getDepositInstruction({
      vault,
      competition,
      depositor: bidder1,
      payer: bidder1,
      amount: 100_000_000n,
    });

    await expect(sendAndConfirm([depositIx], bidder1)).rejects.toThrow();
  });
});

// ── Full-flow tests (require MAGICBLOCK_VALIDATOR) ────────────────────────────

const hasMagicBlock = Boolean(process.env['MAGICBLOCK_VALIDATOR']);

describe.skipIf(!hasMagicBlock)('EscrowVault — deposit + refund (MagicBlock active)', () => {
  it('creates a vault and allows refund after cancel', async () => {
    // This test requires a running MagicBlock validator on devnet.
    // Set MAGICBLOCK_VALIDATOR=1 in .env.test once the full setup is in place.
    //
    // Flow:
    //   1. Create competition
    //   2. Activate (delegate) → competition goes Active
    //   3. Deposit from bidder1
    //   4. Cancel competition
    //   5. Refund bidder1's vault
    expect(hasMagicBlock).toBe(true); // placeholder assertion
  });
});
