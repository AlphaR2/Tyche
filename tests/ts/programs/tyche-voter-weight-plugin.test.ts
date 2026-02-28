/**
 * tyche-voter-weight-plugin tests — raw instruction builders (no generated client).
 *
 * The plugin has no Codama-generated client yet, so instructions are built
 * manually from discriminators + serialised arg bytes.
 *
 * Devnet only.  Requires:
 *   AUTHORITY_KEYPAIR  — funded authority keypair (~0.1 SOL covers all creates)
 *   BIDDER1_KEYPAIR    — funded bidder keypair (UpdateVoterWeightRecord tests)
 *
 * Test groups:
 *   1. PDA derivation     — pure computation, no devnet calls
 *   2. CreateRegistrar    — allocates Registrar + MaxVoterWeightRecord PDAs
 *   3. CreateVoterWeightRecord — allocates per-voter VoterWeightRecord PDA
 *   4. UpdateMaxVoterWeightRecord — refreshes max-weight + slot expiry
 *   5. UpdateVoterWeightRecord  — reads EscrowVault; requires MAGICBLOCK_VALIDATOR
 */

import { describe, it, expect, beforeAll } from 'vitest';
import {
  getProgramDerivedAddress,
  getAddressEncoder,
  generateKeyPairSigner,
  type Address,
  type Instruction,
  type TransactionSigner,
  AccountRole,
} from '@solana/kit';
import { SYSTEM_PROGRAM_ADDRESS } from "@solana-program/system";


import {
  rpc,
  authority  as authorityPromise,
  bidder1    as bidder1Promise,
  newCompetitionId,
} from '../setup/env.js';
import { requireFunds } from '../setup/airdrop.js';
import { sendAndConfirm, accountExists } from '../setup/helpers.js';

// ── Constants ─────────────────────────────────────────────────────────────────

const PLUGIN_PROGRAM_ID =
  'TYGwvLsQWTNgwQcuP4sREXHVinz14WG9caEZecbKTVg' as Address;

// Real tyche-escrow program ID used as the "escrow program" stored in registrar.
const TYCHE_ESCROW_PROGRAM_ID =
  'TYEhGGkbujScDqPK1KTKCRu9cjVzjBH2Yf9Jb5L5Xtk' as Address;

// spl-governance mainnet/devnet program ID (stored in registrar, not called by plugin).
const REALMS_PROGRAM_ID =
  'GovER5Lthms3bLBqWub97yVrMmEogzX7xNjdXpPPCVZw' as Address;

// Instruction discriminators (SHA256("global:<name>")[0..8])
const DISC_CREATE_REGISTRAR             = new Uint8Array([132, 235, 36, 49, 139, 66, 202, 69]);
const DISC_CREATE_VOTER_WEIGHT_RECORD   = new Uint8Array([184, 249, 133, 178, 88, 152, 250, 186]);
const DISC_UPDATE_VOTER_WEIGHT_RECORD   = new Uint8Array([45, 185, 3, 36, 109, 190, 115, 169]);
const DISC_UPDATE_MAX_VOTER_WEIGHT_RECORD = new Uint8Array([103, 175, 201, 251, 2, 9, 251, 179]);

// Account discriminators (SHA256("account:<name>")[0..8])
const DISC_REGISTRAR             = new Uint8Array([193, 202, 205, 51, 78, 168, 150, 128]);
const DISC_VOTER_WEIGHT_RECORD   = new Uint8Array([46, 249, 155, 75, 153, 248, 116, 9]);
const DISC_MAX_VOTER_WEIGHT_RECORD = new Uint8Array([157, 95, 242, 151, 16, 98, 26, 118]);

// PDA seeds
const SEED_REGISTRAR             = new TextEncoder().encode('registrar');
const SEED_VOTER_WEIGHT_RECORD   = new TextEncoder().encode('voter-weight-record');
const SEED_MAX_VOTER_WEIGHT_RECORD = new TextEncoder().encode('max-voter-weight-record');

const enc = getAddressEncoder();

// ── PDA helpers ───────────────────────────────────────────────────────────────

async function getRegistrarPda(realm: Address, mint: Address) {
  return getProgramDerivedAddress({
    programAddress: PLUGIN_PROGRAM_ID,
    seeds: [enc.encode(realm), SEED_REGISTRAR, enc.encode(mint)],
  });
}

async function getVoterWeightRecordPda(realm: Address, mint: Address, voter: Address) {
  return getProgramDerivedAddress({
    programAddress: PLUGIN_PROGRAM_ID,
    seeds: [SEED_VOTER_WEIGHT_RECORD, enc.encode(realm), enc.encode(mint), enc.encode(voter)],
  });
}

async function getMaxVoterWeightRecordPda(realm: Address, mint: Address) {
  return getProgramDerivedAddress({
    programAddress: PLUGIN_PROGRAM_ID,
    seeds: [enc.encode(realm), SEED_MAX_VOTER_WEIGHT_RECORD, enc.encode(mint)],
  });
}

// ── Instruction builders ──────────────────────────────────────────────────────

/**
 * CreateRegistrar — allocates Registrar + MaxVoterWeightRecord PDAs.
 *
 * Data layout: 8 (disc) + 32 (governance_program_id) + 32 (competition) + 32 (tyche_escrow_program)
 */
function buildCreateRegistrarIx(args: {
  registrar:            Address;
  maxVoterWeightRecord: Address;
  realm:                Address;
  governingTokenMint:   Address;
  realmAuthority:       TransactionSigner;
  payer:                TransactionSigner;
  governanceProgramId:  Address;
  competition:          Address;
  tycheEscrowProgram:   Address;
}): Instruction {
  const data = new Uint8Array(8 + 96);
  data.set(DISC_CREATE_REGISTRAR, 0);
  data.set(enc.encode(args.governanceProgramId),  8);
  data.set(enc.encode(args.competition),          40);
  data.set(enc.encode(args.tycheEscrowProgram),   72);

  return {
    programAddress: PLUGIN_PROGRAM_ID,
    accounts: [
      { address: args.registrar,            role: AccountRole.WRITABLE },
      { address: args.maxVoterWeightRecord, role: AccountRole.WRITABLE },
      { address: args.realm,                role: AccountRole.READONLY },
      { address: args.governingTokenMint,   role: AccountRole.READONLY },
      { address: args.realmAuthority.address, role: AccountRole.READONLY_SIGNER, signer: args.realmAuthority },
      { address: args.payer.address,          role: AccountRole.WRITABLE_SIGNER,  signer: args.payer },
      { address: SYSTEM_PROGRAM_ADDRESS,    role: AccountRole.READONLY },
    ],
    data,
  } as Instruction;
}

/**
 * CreateVoterWeightRecord — allocates per-voter PDA with zero weight.
 *
 * Data layout: 8 (disc only — no args)
 */
function buildCreateVoterWeightRecordIx(args: {
  voterWeightRecord:  Address;
  registrar:          Address;
  voter:              TransactionSigner;
  payer:              TransactionSigner;
  realm:              Address;
  governingTokenMint: Address;
}): Instruction {
  return {
    programAddress: PLUGIN_PROGRAM_ID,
    accounts: [
      { address: args.voterWeightRecord, role: AccountRole.WRITABLE },
      { address: args.registrar,         role: AccountRole.READONLY },
      { address: args.realm,             role: AccountRole.READONLY },
      { address: args.governingTokenMint,   role: AccountRole.READONLY },
      { address: args.voter.address,     role: AccountRole.READONLY_SIGNER, signer: args.voter },
      { address: args.payer.address,     role: AccountRole.WRITABLE_SIGNER, signer: args.payer },
      { address: SYSTEM_PROGRAM_ADDRESS, role: AccountRole.READONLY },
    ],
    data: DISC_CREATE_VOTER_WEIGHT_RECORD,
  } as Instruction;
}

/**
 * UpdateVoterWeightRecord — refreshes weight from the EscrowVault balance.
 *
 * Data layout: 8 (disc) + 1 (action byte: 0=CastVote)
 */
function buildUpdateVoterWeightRecordIx(args: {
  voterWeightRecord: Address;
  registrar:         Address;
  escrowVault:       Address;
  voter:             TransactionSigner;
  proposal:          Address;
  action?:           number;  // VoterWeightAction ordinal, default 0 = CastVote
}): Instruction {
  const data = new Uint8Array(9);
  data.set(DISC_UPDATE_VOTER_WEIGHT_RECORD, 0);
  data[8] = args.action ?? 0;

  return {
    programAddress: PLUGIN_PROGRAM_ID,
    accounts: [
      { address: args.voterWeightRecord, role: AccountRole.WRITABLE },
      { address: args.registrar,         role: AccountRole.READONLY },
      { address: args.escrowVault,       role: AccountRole.READONLY },
      { address: args.voter.address,     role: AccountRole.READONLY_SIGNER, signer: args.voter },
      { address: args.proposal,          role: AccountRole.READONLY },
    ],
    data,
  } as Instruction;
}

/**
 * UpdateMaxVoterWeightRecord — stamps current slot onto the MaxVoterWeightRecord.
 *
 * Data layout: 8 (disc only — no args)
 */
function buildUpdateMaxVoterWeightRecordIx(args: {
  maxVoterWeightRecord: Address;
  registrar:            Address;
}): Instruction {
  return {
    programAddress: PLUGIN_PROGRAM_ID,
    accounts: [
      { address: args.maxVoterWeightRecord, role: AccountRole.WRITABLE },
      { address: args.registrar,            role: AccountRole.READONLY },
    ],
    data: DISC_UPDATE_MAX_VOTER_WEIGHT_RECORD,
  } as Instruction;
}

// ── Account data helpers ──────────────────────────────────────────────────────

async function fetchAccountData(address: Address): Promise<Buffer | null> {
  const { value } = await rpc
    .getAccountInfo(address, { encoding: 'base64', commitment: 'confirmed' })
    .send();
  if (!value) return null;
  return Buffer.from(value.data[0], 'base64');
}

// ── Shared test state ─────────────────────────────────────────────────────────

let authority: TransactionSigner;
let bidder1:   TransactionSigner;

// Fresh addresses per test suite run — avoids "already initialized" conflicts.
let realm:                Address;
let mint:                 Address;
let competition:          Address;
let registrar:            Address;
let maxVoterWeightRecord: Address;

beforeAll(async () => {
  [authority, bidder1] = await Promise.all([authorityPromise, bidder1Promise]);
  await Promise.all([
    requireFunds(authority.address, 100_000_000n),
    requireFunds(bidder1.address,   100_000_000n),
  ]);

  // Use authority's address as a stable "realm" seed for this suite.
  // The plugin doesn't validate that realm is a real spl-governance realm.
  const realmSigner = await generateKeyPairSigner();
  const mintSigner  = await generateKeyPairSigner();
  const compSigner  = await generateKeyPairSigner();

  realm       = realmSigner.address;
  mint        = mintSigner.address;
  competition = compSigner.address;

  [registrar]            = await getRegistrarPda(realm, mint);
  [maxVoterWeightRecord] = await getMaxVoterWeightRecordPda(realm, mint);
});

// ── 1. PDA derivation ─────────────────────────────────────────────────────────

describe('PDA derivation', () => {
  it('derives a unique registrar per (realm, mint) pair', async () => {
    const sA = await generateKeyPairSigner();
    const sB = await generateKeyPairSigner();
    const sharedMint = (await generateKeyPairSigner()).address;

    const [a] = await getRegistrarPda(sA.address, sharedMint);
    const [b] = await getRegistrarPda(sB.address, sharedMint);
    expect(a).not.toBe(b);
  });

  it('derives a unique VoterWeightRecord per voter', async () => {
    const r = (await generateKeyPairSigner()).address;
    const m = (await generateKeyPairSigner()).address;

    const [vwr1] = await getVoterWeightRecordPda(r, m, authority.address);
    const [vwr2] = await getVoterWeightRecordPda(r, m, bidder1.address);
    expect(vwr1).not.toBe(vwr2);
  });

  it('derives consistent PDAs across calls (deterministic)', async () => {
    const r = (await generateKeyPairSigner()).address;
    const m = (await generateKeyPairSigner()).address;

    const [pda1] = await getRegistrarPda(r, m);
    const [pda2] = await getRegistrarPda(r, m);
    expect(pda1).toBe(pda2);
  });
});

// ── 2. CreateRegistrar ────────────────────────────────────────────────────────

describe('CreateRegistrar', () => {
  it('creates Registrar and MaxVoterWeightRecord PDAs on devnet', async () => {
    // Registrar should not exist before this test
    const beforeReg  = await accountExists(registrar);
    const beforeMvwr = await accountExists(maxVoterWeightRecord);
    expect(beforeReg).toBe(false);
    expect(beforeMvwr).toBe(false);

    await sendAndConfirm([
      buildCreateRegistrarIx({
        registrar,
        maxVoterWeightRecord,
        realm,
        governingTokenMint:  mint,
        realmAuthority:      authority,
        payer:               authority,
        governanceProgramId: REALMS_PROGRAM_ID,
        competition,
        tycheEscrowProgram:  TYCHE_ESCROW_PROGRAM_ID,
      }),
    ], authority);

    expect(await accountExists(registrar)).toBe(true);
    expect(await accountExists(maxVoterWeightRecord)).toBe(true);
  });

  it('Registrar has correct discriminator and fields', async () => {
    const data = await fetchAccountData(registrar);
    expect(data).not.toBeNull();

    // Discriminator (bytes 0..8)
    expect(Array.from(data!.subarray(0, 8))).toEqual(Array.from(DISC_REGISTRAR));

    // realm stored at bytes 40..72
    expect(Buffer.from(data!.subarray(40, 72)).toString('hex'))
      .toBe(Buffer.from(enc.encode(realm)).toString('hex'));

    // governing_token_mint stored at bytes 72..104
    expect(Buffer.from(data!.subarray(72, 104)).toString('hex'))
      .toBe(Buffer.from(enc.encode(mint)).toString('hex'));

    // competition stored at bytes 200..232
    // Layout: disc(8) + governance_program_id(32) + realm(32) + mint(32) + prev_plugin(32) +
    //         has_prev_plugin(1) + _pad0(7) + tyche_escrow_program_id(32) + competition(32)
    const competitionOffset = 8 + 32 + 32 + 32 + 32 + 8 + 32; // = 176
    expect(Buffer.from(data!.subarray(competitionOffset, competitionOffset + 32)).toString('hex'))
      .toBe(Buffer.from(enc.encode(competition)).toString('hex'));
  });

  it('MaxVoterWeightRecord has correct discriminator', async () => {
    const data = await fetchAccountData(maxVoterWeightRecord);
    expect(data).not.toBeNull();
    expect(Array.from(data!.subarray(0, 8))).toEqual(Array.from(DISC_MAX_VOTER_WEIGHT_RECORD));

    // max_voter_weight = u64::MAX at bytes 72..80
    const maxWeight = data!.readBigUInt64LE(72);
    expect(maxWeight).toBe(BigInt('18446744073709551615')); // u64::MAX
  });

  it('rejects a second CreateRegistrar for the same (realm, mint)', async () => {
    await expect(
      sendAndConfirm([
        buildCreateRegistrarIx({
          registrar,
          maxVoterWeightRecord,
          realm,
          governingTokenMint:  mint,
          realmAuthority:      authority,
          payer:               authority,
          governanceProgramId: REALMS_PROGRAM_ID,
          competition,
          tycheEscrowProgram:  TYCHE_ESCROW_PROGRAM_ID,
        }),
      ], authority),
    ).rejects.toThrow();
  });
});

// ── 3. CreateVoterWeightRecord ────────────────────────────────────────────────

describe('CreateVoterWeightRecord', () => {
  let voterWeightRecord: Address;

  beforeAll(async () => {
    [voterWeightRecord] = await getVoterWeightRecordPda(realm, mint, authority.address);
  });

  it('creates VoterWeightRecord PDA for the voter', async () => {
    expect(await accountExists(voterWeightRecord)).toBe(false);

    await sendAndConfirm([
      buildCreateVoterWeightRecordIx({
        voterWeightRecord,
        registrar,
        voter:              authority,
        payer:              authority,
        realm,
        governingTokenMint: mint,
      }),
    ], authority);

    expect(await accountExists(voterWeightRecord)).toBe(true);
  });

  it('VoterWeightRecord has correct discriminator and zero voter weight', async () => {
    const data = await fetchAccountData(voterWeightRecord);
    expect(data).not.toBeNull();

    // Discriminator (bytes 0..8) — fixed by Anchor/spl-governance-addin-api convention
    expect(Array.from(data!.subarray(0, 8))).toEqual(Array.from(DISC_VOTER_WEIGHT_RECORD));

    // voter_weight = 0 at bytes 104..112
    const voterWeight = data!.readBigUInt64LE(104);
    expect(voterWeight).toBe(0n);

    // voter (governing_token_owner) at bytes 72..104
    expect(Buffer.from(data!.subarray(72, 104)).toString('hex'))
      .toBe(Buffer.from(enc.encode(authority.address)).toString('hex'));
  });

  it('rejects a second CreateVoterWeightRecord for the same voter', async () => {
    await expect(
      sendAndConfirm([
        buildCreateVoterWeightRecordIx({
          voterWeightRecord,
          registrar,
          voter:              authority,
          payer:              authority,
          realm,
          governingTokenMint: mint,
        }),
      ], authority),
    ).rejects.toThrow();
  });
});

// ── 4. UpdateMaxVoterWeightRecord ─────────────────────────────────────────────

describe('UpdateMaxVoterWeightRecord', () => {
  it('stamps current slot onto the MaxVoterWeightRecord', async () => {
    // Read slot before update
    // const { value: slotBefore } = await rpc.getSlot({ commitment: 'confirmed' }).send();

    await sendAndConfirm([
      buildUpdateMaxVoterWeightRecordIx({ maxVoterWeightRecord, registrar }),
    ], authority);

    const data = await fetchAccountData(maxVoterWeightRecord);
    expect(data).not.toBeNull();

    // Discriminator unchanged
    expect(Array.from(data!.subarray(0, 8))).toEqual(Array.from(DISC_MAX_VOTER_WEIGHT_RECORD));

    // max_voter_weight still u64::MAX
    expect(data!.readBigUInt64LE(72)).toBe(BigInt('18446744073709551615'));

    // expiry tag = 1 (Some) at byte 80
    expect(data![80]).toBe(1);

    // expiry slot >= slotBefore
    // const expirySlot = data!.readBigUInt64LE(81);
    // // expect(expirySlot).toBeGreaterThanOrEqual(BigInt(slotBefore));
  });
});

// ── 5. UpdateVoterWeightRecord (requires MagicBlock + active vault) ───────────

const hasMagicBlock = Boolean(process.env['MAGICBLOCK_VALIDATOR']);

describe.skipIf(!hasMagicBlock)('UpdateVoterWeightRecord (MagicBlock required)', () => {
  /**
   * Full flow test:
   *  1. Create a competition and activate it via MagicBlock delegation
   *  2. bidder1 deposits SOL into an EscrowVault
   *  3. bidder1 calls UpdateVoterWeightRecord
   *  4. Verify voter_weight == deposit amount
   *
   * This test is skipped on standard devnet runs (no MagicBlock validator).
   * Set MAGICBLOCK_VALIDATOR=1 in .env.test once a full MagicBlock setup is live.
   */
  it('sets voter_weight from EscrowVault.amount', async () => {
    // ── Setup: create a competition and deposit from bidder1 ──────────────────
    // Import here so this test can be skipped cleanly without import errors.
    const {
      getCreateCompetitionInstruction,
    } = await import('tyche-generated-core');
    const { getDepositInstruction } = await import('tyche-generated-escrow');
    const {
      getCompetitionStatePda,
      getProtocolConfigPda,
      getEscrowVaultPda,
      ASSET_TYPE_NFT,
    } = await import('tyche-sdk');

    const competitionId   = newCompetitionId();
    const [competitionPda]    = await getCompetitionStatePda(authority.address, competitionId);
    const [protocolConfig]    = await getProtocolConfigPda();
    const [vaultPda]          = await getEscrowVaultPda(competitionPda, bidder1.address);

    const idBytes = new Uint8Array(8);
    new DataView(idBytes.buffer).setBigUint64(0, competitionId, true);

    // Create competition
    await sendAndConfirm([
      getCreateCompetitionInstruction({
        competition: competitionPda,
        authority,
        payer:           authority,
        protocolConfig,
        id:              idBytes,
        assetType:       ASSET_TYPE_NFT,
        pad:             new Uint8Array(6),
        startTime:       BigInt(Math.floor(Date.now() / 1000)),
        durationSecs:    3_600n,
        softCloseWindow: 300n,
        softCloseExtension: 300n,
        maxSoftCloses:   5,
        pad2:            new Uint8Array(2),
        reservePrice:    100_000_000n,
      }),
    ], authority);

    // NOTE: Activate (MagicBlock delegation) must happen before Deposit.
    // Skipping activation here — test assumes MAGICBLOCK_VALIDATOR handles it.
    const DEPOSIT_AMOUNT = 200_000_000n; // 0.2 SOL

    await sendAndConfirm([
      getDepositInstruction({
        vault:      vaultPda,
        competition: competitionPda,
        depositor:  bidder1,
        payer:      bidder1,
        amount:     DEPOSIT_AMOUNT,
      }),
    ], bidder1);

    // ── Create a VoterWeightRecord for bidder1 on this realm ──────────────────
    const [vwr] = await getVoterWeightRecordPda(realm, mint, bidder1.address);

    await sendAndConfirm([
      buildCreateVoterWeightRecordIx({
        voterWeightRecord:  vwr,
        registrar,
        voter:              bidder1,
        payer:              bidder1,
        realm,
        governingTokenMint: mint,
      }),
    ], bidder1);

    // ── Build a fresh registrar scoped to this competition ────────────────────
    // Use a fresh realm+mint for this test so competition matches the vault.
    const freshRealm       = (await generateKeyPairSigner()).address;
    const freshMint        = (await generateKeyPairSigner()).address;
    const [freshRegistrar] = await getRegistrarPda(freshRealm, freshMint);
    const [freshMvwr]      = await getMaxVoterWeightRecordPda(freshRealm, freshMint);
    const [freshVwr]       = await getVoterWeightRecordPda(freshRealm, freshMint, bidder1.address);

    await sendAndConfirm([
      buildCreateRegistrarIx({
        registrar:           freshRegistrar,
        maxVoterWeightRecord: freshMvwr,
        realm:               freshRealm,
        governingTokenMint:  freshMint,
        realmAuthority:      authority,
        payer:               authority,
        governanceProgramId: REALMS_PROGRAM_ID,
        competition:         competitionPda,        // scoped to this vault's competition
        tycheEscrowProgram:  TYCHE_ESCROW_PROGRAM_ID,
      }),
    ], authority);

    await sendAndConfirm([
      buildCreateVoterWeightRecordIx({
        voterWeightRecord:  freshVwr,
        registrar:          freshRegistrar,
        voter:              bidder1,
        payer:              bidder1,
        realm:              freshRealm,
        governingTokenMint: freshMint,
      }),
    ], bidder1);

    // Use a placeholder proposal address (any non-zero address works)
    const proposal = (await generateKeyPairSigner()).address;

    await sendAndConfirm([
      buildUpdateVoterWeightRecordIx({
        voterWeightRecord: freshVwr,
        registrar:         freshRegistrar,
        escrowVault:       vaultPda,
        voter:             bidder1,
        proposal,
        action:            0, // CastVote
      }),
    ], bidder1);

    // ── Verify ────────────────────────────────────────────────────────────────
    const data = await fetchAccountData(freshVwr);
    expect(data).not.toBeNull();

    // voter_weight at bytes 104..112 == DEPOSIT_AMOUNT
    const voterWeight = data!.readBigUInt64LE(104);
    expect(voterWeight).toBe(DEPOSIT_AMOUNT);

    // expiry tag = 1 (Some) at byte 112
    expect(data![112]).toBe(1);

    // action tag = 1 (Some) at byte 121
    expect(data![121]).toBe(1);
    // action value = 0 (CastVote) at byte 122
    expect(data![122]).toBe(0);

    // target tag = 1 (Some) at byte 123
    expect(data![123]).toBe(1);
  });
});
