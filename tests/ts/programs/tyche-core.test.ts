/**
 * tyche-core program tests — raw instruction builders (no SDK wrapper).
 *
 * Tests in this file call generated Codama instruction builders directly,
 * mirroring the Rust unit tests in tests/src/tyche_core.rs.
 *
 * Devnet only.  Requires:
 *   AUTHORITY_KEYPAIR  — funded authority keypair file path
 *   TREASURY_ADDRESS   — treasury base58 address
 */

import { describe, it, expect, beforeAll } from 'vitest';
import {
  getCreateCompetitionInstruction,
  getCancelCompetitionInstruction,
  fetchCompetitionState,
  fetchMaybeCompetitionState,
  fetchProtocolConfig,
} from 'tyche-generated-core';
import {
  type TransactionSigner,
  type Address,

} from '@solana/kit';
import { SYSTEM_PROGRAM_ADDRESS } from "@solana-program/system";

import { rpc, authority as authorityPromise, newCompetitionId } from '../setup/env.js';
import { requireFunds } from '../setup/airdrop.js';
import { sendAndConfirm, accountExists } from '../setup/helpers.js';
import {
  getCompetitionStatePda,
  getProtocolConfigPda,
  getPermissionPda,
  TYCHE_CORE_PROGRAM_ADDRESS,
  PHASE_SCHEDULED,
  PHASE_CANCELLED,
  ASSET_TYPE_NFT,
  MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS,
} from 'tyche-sdk';

// ── Shared state ──────────────────────────────────────────────────────────────

let authority: TransactionSigner;

beforeAll(async () => {
  authority = await authorityPromise;
  await requireFunds(authority.address, 100_000_000n);
});

// ── ProtocolConfig (read-only — must already be initialised on devnet) ────────

describe('ProtocolConfig', () => {
  it('exists on devnet', async () => {
    const [configAddress] = await getProtocolConfigPda();
    const exists = await accountExists(configAddress);
    expect(exists, `ProtocolConfig PDA ${configAddress} not found — run InitialiseProtocolConfig first`).toBe(true);
  });

  it('is fetchable and has expected structure', async () => {
    const [configAddress] = await getProtocolConfigPda();
    const config = await fetchProtocolConfig(rpc, configAddress);
    expect(config.data).toBeDefined();
    // fee_basis_points must be ≤ MAX_FEE_BASIS_POINTS (1000 = 10%)
    expect(Number(config.data.feeBasisPoints)).toBeLessThanOrEqual(1_000);
  });
});

// ── CreateCompetition ─────────────────────────────────────────────────────────

describe('CreateCompetition', () => {
  it('creates a CompetitionState PDA in Scheduled phase', async () => {
    const competitionId = newCompetitionId();
    const [competitionAddress] = await getCompetitionStatePda(authority.address, competitionId);
    const [protocolConfigAddress] = await getProtocolConfigPda();

    // Address/Id seed expects 32 bytes
    const idBytes = new Uint8Array(32);
    new DataView(idBytes.buffer).setBigUint64(0, competitionId, true);

    const now = BigInt(Math.floor(Date.now() / 1000));

    const ix = getCreateCompetitionInstruction({
      competition:    competitionAddress,
      authority,
      payer:          authority,
      protocolConfig: protocolConfigAddress,
      id:             idBytes,
      assetType:      ASSET_TYPE_NFT,
      pad:            new Uint8Array(7),
      startTime:      now,
      durationSecs:   3_600n,          // 1 hour
      softCloseWindow:    300n,
      softCloseExtension: 300n,
      maxSoftCloses:      5,
      pad2:               new Uint8Array(7),
      reservePrice:   500_000_000n,    // 0.5 SOL
    });

    await sendAndConfirm([ix], authority);

    // Verify account was created
    const exists = await accountExists(competitionAddress);
    expect(exists).toBe(true);

    // Fetch and verify state
    const competition = await fetchCompetitionState(rpc, competitionAddress);
    expect(competition.data.phase).toBe(PHASE_SCHEDULED);
    expect(competition.data.reservePrice).toBe(500_000_000n);
  });

  it('rejects a duplicate competition ID from the same authority', async () => {
    const competitionId = newCompetitionId();
    const [competitionAddress] = await getCompetitionStatePda(authority.address, competitionId);
    const [protocolConfigAddress] = await getProtocolConfigPda();

    // Address/Id seed expects 32 bytes
    const idBytes = new Uint8Array(32);
    new DataView(idBytes.buffer).setBigUint64(0, competitionId, true);

    const ix = getCreateCompetitionInstruction({
      competition:    competitionAddress,
      authority,
      payer:          authority,
      protocolConfig: protocolConfigAddress,
      id:             idBytes,
      assetType:      ASSET_TYPE_NFT,
      pad:            new Uint8Array(7),
      startTime:      BigInt(Math.floor(Date.now() / 1000)),
      durationSecs:   3_3600n, // Wait, it was 3_600n. Wait, I'll keep 3_600n.
      softCloseWindow:    300n,
      softCloseExtension: 300n,
      maxSoftCloses:      5,
      pad2:               new Uint8Array(7),
      reservePrice:   1_000_000n,
    });

    // First creation succeeds
    await sendAndConfirm([ix], authority);

    // Second creation with the same PDA address should fail (account already exists)
    await expect(sendAndConfirm([ix], authority)).rejects.toThrow();
  });
});

// ── CancelCompetition ─────────────────────────────────────────────────────────

describe('CancelCompetition', () => {
  it('cancels a Scheduled competition', async () => {
    const competitionId = newCompetitionId();
    const [competitionAddress] = await getCompetitionStatePda(authority.address, competitionId);
    const [protocolConfigAddress] = await getProtocolConfigPda();

    // Address/Id seed expects 32 bytes
    const idBytes = new Uint8Array(32);
    new DataView(idBytes.buffer).setBigUint64(0, competitionId, true);

    // Create first
    const createIx = getCreateCompetitionInstruction({
      competition:    competitionAddress,
      authority,
      payer:          authority,
      protocolConfig: protocolConfigAddress,
      id:             idBytes,
      assetType:      ASSET_TYPE_NFT,
      pad:            new Uint8Array(7),
      startTime:      BigInt(Math.floor(Date.now() / 1000)),
      durationSecs:   3_600n,
      softCloseWindow:    300n,
      softCloseExtension: 300n,
      maxSoftCloses:      5,
      pad2:               new Uint8Array(7),
      reservePrice:   1_000_000n,
    });
    await sendAndConfirm([createIx], authority);

    // Cancel
    const [permission] = await getPermissionPda(authority.address);
    const cancelIx = getCancelCompetitionInstruction({
      competition: competitionAddress,
      authority,
      permission,
      magicContext: SYSTEM_PROGRAM_ADDRESS,
      magicProgram: MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS,
    });

    await sendAndConfirm([cancelIx], authority);

    // Verify phase is now Cancelled
    const competition = await fetchMaybeCompetitionState(rpc, competitionAddress);
    expect(competition.exists).toBe(true);
    if (competition.exists) {
      expect(competition.data.phase).toBe(PHASE_CANCELLED);
    }
  });
});
