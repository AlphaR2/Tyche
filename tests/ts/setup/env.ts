/**
 * Shared test environment — RPC client, WebSocket subscriptions, signers.
 *
 * Follows the @solana/kit "Client" pattern:
 *   const { rpc, rpcSubscriptions } = createClient(rpcUrl, wsUrl);
 *   const { value: balance }        = await rpc.getBalance(wallet).send();
 *   const { value: accountInfo }    = await rpc.getAccountInfo(account).send();
 *   const { value: latestBlockhash } = await rpc.getLatestBlockhash().send();
 *
 * Config is read from .env.test (loaded by setup/global-setup.ts via setupFiles).
 *
 * Required .env.test variables:
 *   RPC_URL            Solana HTTP RPC endpoint (e.g. your Helius devnet URL)
 *   AUTHORITY_KEYPAIR  Path to a funded Solana CLI keypair JSON file
 *   BIDDER1_KEYPAIR    Path to a funded Solana CLI keypair JSON file
 *   BIDDER2_KEYPAIR    Path to a funded Solana CLI keypair JSON file
 *   TREASURY_ADDRESS   Base58 address that receives protocol fees
 *
 * Optional:
 *   RPC_WS_URL    WebSocket endpoint (auto-derived from RPC_URL if absent)
 *   CRANK_KEYPAIR Path to crank keypair (needed for settlement tests)
 */

import {
  createKeyPairSignerFromBytes,
  type Address,
  type Rpc,
  type RpcSubscriptions,
  type SolanaRpcApi,
  type SolanaRpcSubscriptionsApi,
} from '@solana/kit';
import { readFileSync } from 'fs';
import { createClient, wsUrlFromRpcUrl, type Client } from './client.js';

// ── Endpoints ─────────────────────────────────────────────────────────────────

const RPC_URL    = process.env['RPC_URL']    ?? 'https://api.devnet.solana.com';
const RPC_WS_URL = process.env['RPC_WS_URL'] ?? wsUrlFromRpcUrl(RPC_URL);

// Provide a router switch based on the environment configuration
export const MAGICBLOCK_REGION = process.env['MAGICBLOCK_REGION'] ?? 'Devnet';
export const MAGICBLOCK_ROUTER_URL = MAGICBLOCK_REGION === 'TEE' 
  ? 'https://tee.magicblock.app' 
  : 'https://devnet-router.magicblock.app';

// ── Client (rpc + rpcSubscriptions) ──────────────────────────────────────────

export const client: Client = createClient(RPC_URL, RPC_WS_URL);

/** HTTP RPC — use for account fetches, blockhash, balance checks, etc. */
export const rpc: Rpc<SolanaRpcApi> = client.rpc;

/** WebSocket subscriptions — used internally by sendAndConfirmTransactionFactory. */
export const rpcSubscriptions: RpcSubscriptions<SolanaRpcSubscriptionsApi> =
  client.rpcSubscriptions;

/** Raw HTTP URL (needed for manual JSON-RPC calls like getBlockhashForAccounts). */
export const rpcUrl = RPC_URL;

// ── Keypair helpers ───────────────────────────────────────────────────────────

/** Load a Solana CLI keypair JSON file and return a TransactionSigner. */
export async function loadKeypair(envVar: string) {
  const path = process.env[envVar];
  if (!path) throw new Error(`${envVar} is not set in .env.test`);
  const bytes = new Uint8Array(JSON.parse(readFileSync(path, 'utf-8')) as number[]);
  return createKeyPairSignerFromBytes(bytes);
}

// ── Lazily-initialised signers ────────────────────────────────────────────────
// Each is a Promise — await them inside beforeAll() in your test files.

export const authority = loadKeypair('AUTHORITY_KEYPAIR');
export const bidder1   = loadKeypair('BIDDER1_KEYPAIR');
export const bidder2   = loadKeypair('BIDDER2_KEYPAIR');
export const crank     = process.env['CRANK_KEYPAIR'] ? loadKeypair('CRANK_KEYPAIR') : null;

// ── Static config ─────────────────────────────────────────────────────────────

export const TREASURY_ADDRESS = (process.env['TREASURY_ADDRESS'] ?? '') as Address;

// ── Competition ID factory ────────────────────────────────────────────────────

/**
 * Returns a fresh competition ID very unlikely to collide with existing ones.
 *
 * ```ts
 * const id  = newCompetitionId();                                   // single
 * const ids = Array.from({ length: 3 }, (_, i) => newCompetitionId() + BigInt(i)); // batch
 * ```
 */
export function newCompetitionId(): bigint {
  return BigInt(Date.now());
}
