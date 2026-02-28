/**
 * init-protocol-config.test.ts
 *
 * One-time initializer for the tyche-core ProtocolConfig singleton.
 * Safe to run multiple times — if already initialized it just prints the
 * current config and exits without sending a transaction.
 *
 * Run with:
 *   cd tests/ts && npx vitest run init-protocol-config --reporter=verbose
 */

import { describe, it, beforeAll } from 'vitest';
import {
  getAddressEncoder,
  AccountRole,
  type Address,
  type Instruction,
  type TransactionSigner,
} from '@solana/kit';
import { SYSTEM_PROGRAM_ADDRESS } from '@solana-program/system';
import { fetchProtocolConfig } from 'tyche-generated-core';
import { getProtocolConfigPda } from 'tyche-sdk';
import { rpc, authority as authorityPromise, crank as crankPromise, TREASURY_ADDRESS } from './setup/env.js';
import { requireFunds } from './setup/airdrop.js';
import { sendAndConfirm, accountExists } from './setup/helpers.js';

// ── Config values — edit before first run ────────────────────────────────────

const FEE_BASIS_POINTS   = 250;   // 2.5%  (max 1000 = 10%)
const MAX_SOFT_CLOSES    = 5;     // competition extension limit
const MIN_RESERVE_PRICE  = 10_000_000n;  // 0.01 SOL in lamports
const MIN_DURATION_SECS  = 60n;          // 60-second minimum competition

// ─────────────────────────────────────────────────────────────────────────────

const TYCHE_CORE_PROGRAM_ID =
  'TYCANGQk6tumtij3tHwsRPSNkSHU3KGSNxNG59qJrHx' as Address;

// SHA256("global:initialize_protocol_config")[0..8]
const DISC = new Uint8Array([28, 50, 43, 233, 244, 98, 123, 118]);

const enc = getAddressEncoder();

// ── Raw instruction builder ───────────────────────────────────────────────────

/**
 * Builds `InitializeProtocolConfig` from raw bytes.
 *
 * Data layout: 8 (disc) + 152 (args) = 160 bytes
 *   off  0..8   : discriminator
 *   off  8..40  : authority           (32 bytes)
 *   off 40..72  : emergency_authority (32 bytes)
 *   off 72..104 : treasury            (32 bytes)
 *   off 104..136: crank_authority     (32 bytes)
 *   off 136..138: fee_basis_points    (u16 LE)
 *   off 138..140: _pad
 *   off 140     : max_soft_closes_cap (u8)
 *   off 141..144: _pad2
 *   off 144..152: min_reserve_price   (u64 LE)
 *   off 152..160: min_duration_secs   (i64 LE)
 */
function buildInitializeProtocolConfigIx(args: {
  protocolConfig:     Address;
  authority:          TransactionSigner;
  payer:              TransactionSigner;
  emergencyAuthority: Address;
  treasury:           Address;
  crankAuthority:     Address;
  feeBasisPoints:     number;
  maxSoftClosesCap:   number;
  minReservePrice:    bigint;
  minDurationSecs:    bigint;
}): Instruction {
  const data = new Uint8Array(160);
  const view = new DataView(data.buffer);
  let off = 0;

  data.set(DISC, off);                               off += 8;
  data.set(enc.encode(args.authority.address), off); off += 32; // authority
  data.set(enc.encode(args.emergencyAuthority), off); off += 32; // emergency_authority
  data.set(enc.encode(args.treasury), off);           off += 32; // treasury
  data.set(enc.encode(args.crankAuthority), off);     off += 32; // crank_authority
  view.setUint16(off, args.feeBasisPoints, true);     off += 2;  // fee_basis_points LE
  off += 2;                                                       // _pad
  view.setUint8(off, args.maxSoftClosesCap);          off += 1;  // max_soft_closes_cap
  off += 3;                                                       // _pad2
  view.setBigUint64(off, args.minReservePrice, true); off += 8;  // min_reserve_price LE
  view.setBigInt64(off, args.minDurationSecs, true);              // min_duration_secs LE

  return {
    programAddress: TYCHE_CORE_PROGRAM_ID,
    accounts: [
      { address: args.protocolConfig,    role: AccountRole.WRITABLE },
      { address: args.authority.address, role: AccountRole.READONLY_SIGNER, signer: args.authority },
      { address: args.payer.address,     role: AccountRole.WRITABLE_SIGNER, signer: args.payer },
      { address: SYSTEM_PROGRAM_ADDRESS, role: AccountRole.READONLY },
    ],
    data,
  } as Instruction;
}

// ─────────────────────────────────────────────────────────────────────────────

let authority: TransactionSigner;
let crankAddress: Address;
let protocolConfigPda: Address;

beforeAll(async () => {
  authority = await authorityPromise;
  crankAddress = crankPromise
    ? (await crankPromise).address
    : authority.address;

  await requireFunds(authority.address, 100_000_000n); // 0.1 SOL min

  [protocolConfigPda] = await getProtocolConfigPda();
});

describe('InitializeProtocolConfig', () => {
  it('initializes the protocol config (skips if already exists)', async () => {
    // ── Check if already initialized ────────────────────────────────────────
    if (await accountExists(protocolConfigPda)) {
      const cfg = await fetchProtocolConfig(rpc, protocolConfigPda);
      console.log('\n  ProtocolConfig already initialized — current state:');
      console.log('  address           :', protocolConfigPda);
      console.log('  authority         :', cfg.data.authority);
      console.log('  emergency_auth    :', cfg.data.emergencyAuthority);
      console.log('  treasury          :', cfg.data.treasury);
      console.log('  crank_authority   :', cfg.data.crankAuthority);
      console.log('  fee_basis_points  :', cfg.data.feeBasisPoints, `(${cfg.data.feeBasisPoints / 100}%)`);
      console.log('  max_soft_closes   :', cfg.data.maxSoftClosesCap);
      console.log('  min_reserve_price :', cfg.data.minReservePrice.toString(), 'lamports');
      console.log('  min_duration_secs :', cfg.data.minDurationSecs.toString(), 's');
      return;
    }

    const treasury = (TREASURY_ADDRESS || authority.address) as Address;

    // ── Build and send ────────────────────────────────────────────────────────
    const ix = buildInitializeProtocolConfigIx({
      protocolConfig:     protocolConfigPda,
      authority:          authority,
      payer:              authority,
      emergencyAuthority: authority.address,
      treasury:           treasury,
      crankAuthority:     crankAddress,
      feeBasisPoints:     FEE_BASIS_POINTS,
      maxSoftClosesCap:   MAX_SOFT_CLOSES,
      minReservePrice:    MIN_RESERVE_PRICE,
      minDurationSecs:    MIN_DURATION_SECS,
    });

    const sig = await sendAndConfirm([ix], authority);

    // ── Print result ─────────────────────────────────────────────────────────
    const cfg = await fetchProtocolConfig(rpc, protocolConfigPda);
    console.log('\n  ProtocolConfig initialized:');
    console.log('  address           :', protocolConfigPda);
    console.log('  authority         :', cfg.data.authority);
    console.log('  treasury          :', cfg.data.treasury);
    console.log('  crank_authority   :', cfg.data.crankAuthority);
    console.log('  fee_basis_points  :', cfg.data.feeBasisPoints, `(${cfg.data.feeBasisPoints / 100}%)`);
    console.log('  max_soft_closes   :', cfg.data.maxSoftClosesCap);
    console.log('  min_reserve_price :', cfg.data.minReservePrice.toString(), 'lamports');
    console.log('  min_duration_secs :', cfg.data.minDurationSecs.toString(), 's');
    console.log('  tx                :', sig);
  });
});
