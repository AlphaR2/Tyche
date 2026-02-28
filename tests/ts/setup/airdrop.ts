/**
 * Airdrop and balance utilities for devnet tests.
 *
 * Devnet rate-limits airdrop requests (HTTP 403). We now check balance
 * and require manual funding if an account has 0 SOL.
 */

import { type Address } from '@solana/kit';
import { rpc } from './env.js';

/**
 * Ensure an address has at least `minLamports`. 
 * If balance is 0, throws an error with funding instructions.
 */
export async function requireFunds(address: Address, minLamports: bigint): Promise<void> {
  const balance = await getBalance(address);
  
  if (balance === 0n) {
    console.error(`\n[ERROR] Account ${address} has 0 SOL.`);
    console.error(`Please fund it manually: solana airdrop 2 ${address} --url devnet\n`);
    throw new Error(`Account ${address} is unfunded (0 SOL).`);
  }

  if (balance < minLamports) {
    console.warn(`[WARNING] Account ${address} balance (${balance}) is below recommended ${minLamports}.`);
  }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

async function getBalance(address: Address): Promise<bigint> {
  const { value } = await rpc.getBalance(address, { commitment: 'confirmed' }).send();
  return value;
}
