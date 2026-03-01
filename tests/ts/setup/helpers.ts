/**
 * Low-level test helpers — transaction building, sending, confirmation.
 *
 * Uses `sendAndConfirmTransactionFactory` from @solana/kit (web3.js v2 pattern),
 * which internally uses WebSocket subscriptions for fast, reliable confirmation —
 * no manual polling loops needed.
 *
 * Pattern:
 *   const sendAndConfirmTx = sendAndConfirmTransactionFactory({ rpc, rpcSubscriptions });
 *   await sendAndConfirmTx(signedTx, { commitment: 'confirmed' });
 */

import {
  pipe,
  createTransactionMessage,
  setTransactionMessageFeePayerSigner,
  appendTransactionMessageInstructions,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners,
  sendAndConfirmTransactionFactory,
  getSignatureFromTransaction,
  type Instruction,
  type TransactionSigner,
  type Blockhash,
  type Address,
} from '@solana/kit';
import { rpc, rpcSubscriptions, rpcUrl, MAGICBLOCK_ROUTER_URL } from './env.js';
import { getBlockhashForAccounts as SDKGetBlockhashForAccounts } from '../../../packages/sdk/src/router';

// ── Shared send-and-confirm ───────────────────────────────────────────────────

/**
 * Initialised once from the shared client — uses WebSocket subscriptions
 * for instant transaction confirmation (no polling loop required).
 */
const sendAndConfirmTx = sendAndConfirmTransactionFactory({ rpc, rpcSubscriptions });

// ── Types ─────────────────────────────────────────────────────────────────────

type BlockhashLifetime = {
  blockhash: Blockhash;
  lastValidBlockHeight: bigint;
};

// ── Internal builder ──────────────────────────────────────────────────────────

async function buildSignedTx(
  instructions: Instruction[],
  payer:        TransactionSigner,
  blockhash:    BlockhashLifetime,
) {
  return signTransactionMessageWithSigners(
    pipe(
      createTransactionMessage({ version: 0 }),
      tx => setTransactionMessageFeePayerSigner(payer, tx),
      tx => appendTransactionMessageInstructions(instructions, tx),
      tx => setTransactionMessageLifetimeUsingBlockhash(blockhash, tx),
    ),
  );
}

// ── Public: send + confirm ────────────────────────────────────────────────────

/**
 * Build, sign, send, and confirm a transaction.
 *
 * Fetches a fresh blockhash from the shared RPC, builds a v0 transaction,
 * signs it (all instruction signers picked up automatically), then waits
 * for `confirmed` commitment via WebSocket subscription.
 *
 * ```ts
 * await sendAndConfirm([createCompetitionIx], authority);
 * ```
 *
 * @returns The base58 transaction signature.
 */
export async function sendAndConfirm(
  instructions: Instruction[],
  payer:        TransactionSigner,
): Promise<string> {
  const { value: latestBlockhash } = await rpc
    .getLatestBlockhash({ commitment: 'confirmed' })
    .send();

  const signed = await buildSignedTx(instructions, payer, latestBlockhash);
  await sendAndConfirmTx(signed as Parameters<typeof sendAndConfirmTx>[0], { commitment: 'confirmed' });
  const sig = getSignatureFromTransaction(signed);
  console.log(`[Demo] Transaction Confirmed: https://explorer.solana.com/tx/${sig}?cluster=devnet`);
  return sig;
}

/**
 * Same as {@link sendAndConfirm} but with a pre-fetched blockhash.
 *
 * Use for MagicBlock PER transactions where you must call
 * `getBlockhashForAccounts` first to get the PER-routed blockhash.
 *
 * ```ts
 * const blockhash = await getBlockhashForAccounts(accounts);
 * await sendAndConfirmWithBlockhash([placeBidIx], bidder, blockhash);
 * ```
 */
export async function sendAndConfirmWithBlockhash(
  instructions: Instruction[],
  payer:        TransactionSigner,
  blockhash:    BlockhashLifetime,
): Promise<string> {
  const signed = await buildSignedTx(instructions, payer, blockhash);
  await sendAndConfirmTx(signed as Parameters<typeof sendAndConfirmTx>[0], { commitment: 'confirmed' });
  const sig = getSignatureFromTransaction(signed);
  console.log(`[Demo] Transaction Confirmed: https://explorer.solana.com/tx/${sig}?cluster=devnet`);
  return sig;
}

// ── MagicBlock Router: getBlockhashForAccounts ────────────────────────────────

/**
 * Calls the MagicBlock Router's `getBlockhashForAccounts` JSON-RPC method.
 *
 * The Router inspects the provided accounts and returns a blockhash from
 * the correct node — mainnet if none are delegated, the PER node otherwise.
 * Pass the result directly to {@link sendAndConfirmWithBlockhash}.
 *
 * @see https://docs.magicblock.gg/router
 */
export async function getBlockhashForAccounts(
  accounts: Address[],
): Promise<BlockhashLifetime> {
  const result = await SDKGetBlockhashForAccounts(MAGICBLOCK_ROUTER_URL, accounts);
  return {
    blockhash: result.blockhash,
    lastValidBlockHeight: result.lastValidBlockHeight,
  } as BlockhashLifetime;
}

// ── Account helpers ───────────────────────────────────────────────────────────

/**
 * Returns `true` if an account exists on-chain (non-null `getAccountInfo` response).
 */
export async function accountExists(address: Address): Promise<boolean> {
  const { value } = await rpc
    .getAccountInfo(address, { encoding: 'base64', commitment: 'confirmed' })
    .send();
  return value !== null;
}

/**
 * Poll until a CompetitionState account reaches a specific phase byte.
 *
 * Phase byte is stored at offset 8 (after the 8-byte Pinocchio discriminator).
 * Polls every 1 s for up to `timeoutMs` (default 30 s).
 */
export async function waitForPhase(
  competitionAddress: Address,
  expectedPhase:      number,
  timeoutMs = 30_000,
): Promise<void> {
  const deadline = Date.now() + timeoutMs;

  while (Date.now() < deadline) {
    const { value: account } = await rpc
      .getAccountInfo(competitionAddress, { encoding: 'base64', commitment: 'confirmed' })
      .send();

    if (account) {
      const data  = Buffer.from(account.data[0], 'base64');
      const phase = data[8];
      if (phase === expectedPhase) return;
    }

    await new Promise(r => setTimeout(r, 1_000));
  }

  throw new Error(
    `Competition ${competitionAddress} did not reach phase ${expectedPhase} within ${timeoutMs} ms`,
  );
}
