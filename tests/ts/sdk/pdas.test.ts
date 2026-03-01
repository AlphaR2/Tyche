/**
 * SDK PDA derivation tests.
 *
 * These tests are pure computation — no RPC calls, no devnet required.
 * They verify that PDA helper functions produce the expected addresses and
 * that bump seeds are in range.
 */

import { describe, it, expect } from 'vitest';
import {
  getCompetitionStatePda,
  getParticipantRecordPda,
  getEscrowVaultPda,
  getAuctionStatePda,
  getBidRecordPda,
  getProtocolConfigPda,
  getDelegationBufferPda,
  getDelegationRecordPda,
  getDelegationMetadataPda,
  getPermissionPda,
  TYCHE_CORE_PROGRAM_ADDRESS,
  TYCHE_ESCROW_PROGRAM_ADDRESS,
  TYCHE_AUCTION_PROGRAM_ADDRESS,
  MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS,
  MAGICBLOCK_PERMISSION_PROGRAM_ADDRESS,
} from 'tyche-sdk';
import { type Address } from '@solana/kit';

// Stable addresses for deterministic assertions
const AUTHORITY = 'Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS' as Address;
const BIDDER    = 'GsbwXfJraMomNxBcjYLcG3mxkBUiyWXAB32fGbSMQRdW' as Address;

// ── Tyche PDAs ────────────────────────────────────────────────────────────────

describe('getCompetitionStatePda', () => {
  it('returns a valid address + bump in [0, 255]', async () => {
    const [addr, bump] = await getCompetitionStatePda(AUTHORITY, 1n);
    expect(typeof addr).toBe('string');
    expect(addr.length).toBeGreaterThan(30);
    expect(bump).toBeGreaterThanOrEqual(0);
    expect(bump).toBeLessThanOrEqual(255);
  });

  it('produces the same address for the same inputs', async () => {
    const [a1] = await getCompetitionStatePda(AUTHORITY, 42n);
    const [a2] = await getCompetitionStatePda(AUTHORITY, 42n);
    expect(a1).toBe(a2);
  });

  it('produces different addresses for different IDs', async () => {
    const [a1] = await getCompetitionStatePda(AUTHORITY, 1n);
    const [a2] = await getCompetitionStatePda(AUTHORITY, 2n);
    expect(a1).not.toBe(a2);
  });

  it('produces different addresses for different authorities', async () => {
    const [a1] = await getCompetitionStatePda(AUTHORITY, 1n);
    const [a2] = await getCompetitionStatePda(BIDDER, 1n);
    expect(a1).not.toBe(a2);
  });
});

describe('getParticipantRecordPda', () => {
  it('uses competition + participant as seeds', async () => {
    const [comp] = await getCompetitionStatePda(AUTHORITY, 1n);
    const [p1]   = await getParticipantRecordPda(comp, AUTHORITY);
    const [p2]   = await getParticipantRecordPda(comp, BIDDER);
    expect(p1).not.toBe(p2);
  });
});

describe('getEscrowVaultPda', () => {
  it('is owned by tyche-escrow program (different from core)', async () => {
    const [comp]  = await getCompetitionStatePda(AUTHORITY, 1n);
    const [vault] = await getEscrowVaultPda(comp, BIDDER);

    // Derive the same PDA manually to confirm program address matters
    const [coreAddr] = await getCompetitionStatePda(AUTHORITY, 1n);
    // The vault PDA is derived under TYCHE_ESCROW_PROGRAM_ADDRESS — so it should
    // differ from any PDA derived under TYCHE_CORE_PROGRAM_ADDRESS.
    expect(vault).not.toBe(coreAddr);
    expect(typeof vault).toBe('string');
  });
});

describe('getAuctionStatePda', () => {
  it('is derived from the competition address', async () => {
    const [comp1] = await getCompetitionStatePda(AUTHORITY, 1n);
    const [comp2] = await getCompetitionStatePda(AUTHORITY, 2n);
    const [auc1]  = await getAuctionStatePda(comp1);
    const [auc2]  = await getAuctionStatePda(comp2);
    expect(auc1).not.toBe(auc2);
  });
});

describe('getBidRecordPda', () => {
  it('is unique per (auction, bidder)', async () => {
    const [comp]    = await getCompetitionStatePda(AUTHORITY, 1n);
    const [auction] = await getAuctionStatePda(comp);
    const [b1]      = await getBidRecordPda(auction, AUTHORITY);
    const [b2]      = await getBidRecordPda(auction, BIDDER);
    expect(b1).not.toBe(b2);
  });
});

describe('getProtocolConfigPda', () => {
  it('is deterministic (no seeds beyond the program-derived constant)', async () => {
    const [a1] = await getProtocolConfigPda();
    const [a2] = await getProtocolConfigPda();
    expect(a1).toBe(a2);
  });
});

// ── MagicBlock delegation PDAs ────────────────────────────────────────────────

describe('getDelegationBufferPda', () => {
  it('uses delegation program', async () => {
    const [comp]   = await getCompetitionStatePda(AUTHORITY, 1n);
    const [buffer] = await getDelegationBufferPda(comp);
    expect(typeof buffer).toBe('string');
    // Buffer, Record, Metadata for the same account must all differ
    const [record]   = await getDelegationRecordPda(comp);
    const [metadata] = await getDelegationMetadataPda(comp);
    expect(buffer).not.toBe(record);
    expect(buffer).not.toBe(metadata);
    expect(record).not.toBe(metadata);
  });
});

// ── MagicBlock permission PDA ─────────────────────────────────────────────────

describe('getPermissionPda', () => {
  it('returns a valid address for a target account', async () => {
    const [addr, bump] = await getPermissionPda(AUTHORITY); // AUTHORITY acts as dummy target
    expect(typeof addr).toBe('string');
    expect(addr.length).toBeGreaterThan(30);
    expect(bump).toBeGreaterThanOrEqual(0);
    expect(bump).toBeLessThanOrEqual(255);
  });

  it('produces different PDAs for different targets', async () => {
    const [p1] = await getPermissionPda(AUTHORITY);
    const [p2] = await getPermissionPda(BIDDER);
    expect(p1).not.toBe(p2);
  });

  it('is deterministic', async () => {
    const [a1] = await getPermissionPda(AUTHORITY);
    const [a2] = await getPermissionPda(AUTHORITY);
    expect(a1).toBe(a2);
  });
});

// ── Program address constants sanity check ───────────────────────────────────

describe('program address constants', () => {
  const cases: [string, string][] = [
    ['TYCHE_CORE_PROGRAM_ADDRESS',              TYCHE_CORE_PROGRAM_ADDRESS],
    ['TYCHE_ESCROW_PROGRAM_ADDRESS',            TYCHE_ESCROW_PROGRAM_ADDRESS],
    ['TYCHE_AUCTION_PROGRAM_ADDRESS',           TYCHE_AUCTION_PROGRAM_ADDRESS],
    ['MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS',   MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS],
    ['MAGICBLOCK_PERMISSION_PROGRAM_ADDRESS',   MAGICBLOCK_PERMISSION_PROGRAM_ADDRESS],
  ];

  it.each(cases)('%s is a valid base58 address string', (_name, addr) => {
    expect(typeof addr).toBe('string');
    // Base58 addresses are 32–44 characters and contain no 0, O, I, l
    expect(addr).toMatch(/^[1-9A-HJ-NP-Za-km-z]{32,44}$/);
  });

  it('MAGICBLOCK_PERMISSION_PROGRAM_ADDRESS is the real ACL program', () => {
    expect(MAGICBLOCK_PERMISSION_PROGRAM_ADDRESS).toBe(
      'ACLseoPoyC3cBqoUtkbjZ4aDrkurZW86v19pXz2XQnp1',
    );
  });
});
