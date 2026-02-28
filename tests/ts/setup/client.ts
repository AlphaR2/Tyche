/**
 * Tyche test client — follows the @solana/kit "Client" pattern from the docs.
 *
 * @see https://solana-foundation.github.io/solana-web3.js/
 *
 * Usage:
 * ```ts
 * import { createClient } from './client.js';
 * const { rpc, rpcSubscriptions } = createClient();
 * const { value: balance } = await rpc.getBalance(myWallet).send();
 * ```
 */

import {
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  type Rpc,
  type RpcSubscriptions,
  type SolanaRpcApi,
  type SolanaRpcSubscriptionsApi,
} from '@solana/kit';

// ── Client type ───────────────────────────────────────────────────────────────

export type Client = {
  rpc:              Rpc<SolanaRpcApi>;
  rpcSubscriptions: RpcSubscriptions<SolanaRpcSubscriptionsApi>;
};

// ── Singleton factory ─────────────────────────────────────────────────────────

let _client: Client | undefined;

/**
 * Returns a cached Client for the given RPC endpoints.
 *
 * The first call initialises the singleton; subsequent calls return the same
 * instance regardless of arguments (endpoints are determined once at startup).
 *
 * @param rpcUrl   HTTP(S) endpoint  — e.g. `https://devnet.helius-rpc.com/?api-key=...`
 * @param wsUrl    WebSocket endpoint — e.g. `wss://devnet.helius-rpc.com/?api-key=...`
 */
export function createClient(rpcUrl: string, wsUrl: string): Client {
  if (!_client) {
    _client = {
      rpc:              createSolanaRpc(rpcUrl),
      rpcSubscriptions: createSolanaRpcSubscriptions(wsUrl),
    };
  }
  return _client;
}

/**
 * Derives the WebSocket URL from an HTTP RPC URL.
 *
 * - `https://` → `wss://`
 * - `http://`  → `ws://`
 *
 * Works for standard Solana, Helius, and the MagicBlock Router endpoints.
 */
export function wsUrlFromRpcUrl(rpcUrl: string): string {
  return rpcUrl.replace(/^https:\/\//, 'wss://').replace(/^http:\/\//, 'ws://');
}
