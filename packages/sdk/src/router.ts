import type { Address } from '@solana/kit';

/**
 * Return type for `getBlockhashForAccounts`
 */
export type BlockhashLifetime = {
  blockhash: string; // e.g. Base58 encoded blockhash
  lastValidBlockHeight: bigint;
};

/**
 * Calls the MagicBlock Router's `getBlockhashForAccounts` JSON-RPC method.
 *
 * @param rpcUrl The MagicBlock Router URL (e.g. MAGICBLOCK_ROUTER_TEE_URL).
 * @param accounts The addresses of the accounts involved in the transaction.
 * @returns A promise that resolves to the blockhash and its last valid block height.
 */
export async function getBlockhashForAccounts(
  rpcUrl: string,
  accounts: Address[],
): Promise<BlockhashLifetime> {
  const response = await fetch(rpcUrl, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      jsonrpc: '2.0',
      id: 1,
      method: 'getBlockhashForAccounts',
      params: [accounts, { commitment: 'confirmed' }],
    }),
  });

  const json = (await response.json()) as {
    result?: { value: { blockhash: string; lastValidBlockHeight: number } };
    error?: { message: string };
  };

  if (json.error || !json.result) {
    throw new Error(`getBlockhashForAccounts failed: ${json.error?.message ?? 'no result returned by router'}`);
  }

  if (!json.result.value) {
    // Router returns result.value = null when none of the provided accounts are
    // delegated to PER.  This usually means Activate has not yet been called.
    throw new Error(
      'getBlockhashForAccounts returned null value — accounts may not be delegated to PER. ' +
      'Ensure ActivateAuction has been called and confirmed before placing bids.',
    );
  }

  const { blockhash, lastValidBlockHeight } = json.result.value;
  return {
    blockhash,
    lastValidBlockHeight: BigInt(lastValidBlockHeight),
  };
}

/**
 * Calls the MagicBlock Router's `getDelegationStatus` JSON-RPC method.
 *
 * @param rpcUrl The MagicBlock Router URL.
 * @param account The address of the account to check.
 * @returns A promise resolving to the delegation status response.
 */
export async function getDelegationStatus(
  rpcUrl: string,
  account: Address,
): Promise<any> {
    const response = await fetch(rpcUrl, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: 1,
          method: 'getDelegationStatus',
          params: [account],
        }),
      });
    
      const json = (await response.json()) as {
        result?: any;
        error?: { message: string };
      };
    
      if (json.error) {
        throw new Error(`getDelegationStatus failed: ${json.error.message}`);
      }
    
      return json.result;
}

/**
 * Calls the MagicBlock Router's `getRoutes` JSON-RPC method.
 *
 * @param rpcUrl The MagicBlock Router URL.
 * @returns A promise resolving to the routes response.
 */
export async function getRoutes(rpcUrl: string): Promise<any> {
    const response = await fetch(rpcUrl, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: 1,
          method: 'getRoutes',
          params: [],
        }),
      });
    
      const json = (await response.json()) as {
        result?: any;
        error?: { message: string };
      };
    
      if (json.error) {
        throw new Error(`getRoutes failed: ${json.error.message}`);
      }
    
      return json.result;
}
