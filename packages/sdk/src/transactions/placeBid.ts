/**
 * buildPlaceBidTransaction
 *
 * Produces the `PlaceBid` instruction (tyche-auction).
 *
 * PlaceBid runs on the MagicBlock PER (ephemeral rollup), not on mainnet.
 * The MagicBlock Router automatically routes the transaction to the correct
 * PER node when you use `getBlockhashForAccounts` to obtain the blockhash.
 *
 * All sealed-bid invariants are enforced inside the PER TEE:
 *   - Bid amounts are never logged (privacy guarantee)
 *   - Bidder identities are not exposed during the active phase
 *   - `PlaceBid` CPIs into `tyche-escrow::Deposit` to lock funds atomically
 *
 * Usage: point your `@solana/kit` `createSolanaRpc` at the MagicBlock Router
 * URL (`MAGICBLOCK_ROUTER_MAINNET_URL` or `MAGICBLOCK_ROUTER_DEVNET_URL`) and
 * use `getBlockhashForAccounts` instead of `getLatestBlockhash`.
 *
 * @see MAGICBLOCK_ROUTER_MAINNET_URL
 * @see MAGICBLOCK_ROUTER_DEVNET_URL
 */

import type { Address, Instruction, TransactionSigner } from '@solana/kit';
import { getPlaceBidInstruction } from 'tyche-generated-auction';
import {
  getAuctionStatePda,
  getCompetitionStatePda,
  getEscrowVaultPda,
  getBidRecordPda,
  getParticipantRecordPda,
} from '../pdas';
import { TYCHE_CORE_PROGRAM_ADDRESS } from '../constants';

export type PlaceBidParams = {
  /**
   * The bidder — their vault will be created/topped up, sealed bid recorded.
   * Must sign the transaction.
   */
  bidder: TransactionSigner;

  /**
   * The fee payer for rent (vault, bid record, participant record).
   * May equal `bidder`.
   */
  payer: TransactionSigner;

  /**
   * Address of the CompetitionState PDA for this auction.
   */
  competitionAddress: Address;

  /**
   * The competition authority (seller). Required to derive the correct
   * CompetitionState PDA for participant tracking in tyche-core.
   * Alternatively, pass `competitionId` + `competitionAuthority` and let the
   * SDK derive `competitionAddress`.
   */
  auctionStateAddress: Address;

  /**
   * Bid amount in lamports. Must exceed current high bid + min_bid_increment
   * (enforced inside the PER TEE — not validated client-side).
   */
  amount: bigint;
};

export type PlaceBidResult = {
  /** The PlaceBid instruction. Route this via the MagicBlock Router. */
  instruction: Instruction;
  /** All account addresses involved — pass these to `getBlockhashForAccounts`. */
  accounts: Address[];
};

/**
 * Builds the `PlaceBid` instruction and returns the list of accounts for use
 * with the MagicBlock Router's `getBlockhashForAccounts`.
 *
 * @example
 * ```ts
 * import {
 *   buildPlaceBidTransaction,
 *   MAGICBLOCK_ROUTER_MAINNET_URL,
 * } from 'tyche-sdk';
 * import {
 *   createSolanaRpc,
 *   pipe,
 *   createTransactionMessage,
 *   setTransactionMessageFeePayerSigner,
 *   appendTransactionMessageInstruction,
 *   signTransactionMessageWithSigners,
 *   sendTransaction,
 * } from '@solana/kit';
 *
 * // Use the MagicBlock Router as your single RPC endpoint
 * const rpc = createSolanaRpc(MAGICBLOCK_ROUTER_MAINNET_URL);
 *
 * const { instruction, accounts } = await buildPlaceBidTransaction({
 *   bidder,
 *   payer: bidder,
 *   competitionAddress,
 *   auctionStateAddress,
 *   amount: 2_000_000_000n, // 2 SOL
 * });
 *
 * // Ask the router for a blockhash valid for the delegated PER node
 * const { blockhash } = await rpc
 *   .getBlockhashForAccounts(accounts)
 *   .send();
 *
 * const tx = pipe(
 *   createTransactionMessage({ version: 0 }),
 *   tx => setTransactionMessageFeePayerSigner(bidder, tx),
 *   tx => appendTransactionMessageInstruction(instruction, tx),
 *   tx => setTransactionMessageLifetimeUsingBlockhash({ blockhash, lastValidBlockHeight: 0n }, tx),
 * );
 * const signed = await signTransactionMessageWithSigners(tx);
 * await sendTransaction(rpc, signed);
 * ```
 */
export async function buildPlaceBidTransaction(
  params: PlaceBidParams,
): Promise<PlaceBidResult> {
  const { bidder, payer, competitionAddress, auctionStateAddress, amount } = params;

  const [[bidRecord], [vault], [participantRecord]] = await Promise.all([
    getBidRecordPda(auctionStateAddress, bidder.address),
    getEscrowVaultPda(competitionAddress, bidder.address),
    getParticipantRecordPda(competitionAddress, bidder.address),
  ]);

  const instruction = getPlaceBidInstruction({
    auctionState: auctionStateAddress,
    competition: competitionAddress,
    bidRecord,
    vault,
    bidder,
    payer,
    tycheCoreProgram: TYCHE_CORE_PROGRAM_ADDRESS,
    competitionParticipantRecord: participantRecord,
    amount,
  });

  const accounts: Address[] = [
    auctionStateAddress,
    competitionAddress,
    bidRecord,
    vault,
    bidder.address,
    payer.address,
    TYCHE_CORE_PROGRAM_ADDRESS,
    participantRecord,
  ];

  return { instruction, accounts };
}
