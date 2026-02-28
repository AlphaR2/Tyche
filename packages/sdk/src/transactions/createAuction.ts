/**
 * buildCreateAuctionTransaction
 *
 * Produces the two instructions required to set up a sealed-bid auction:
 *   1. `CreateCompetition` (tyche-core)  — allocates the CompetitionState PDA
 *   2. `CreateAuction`    (tyche-auction) — allocates the AuctionState PDA
 *
 * Both instructions must be included in the same transaction (or submitted
 * back-to-back — `CreateAuction` validates that the CompetitionState exists).
 *
 * Follow this with `buildActivateAuctionTransaction` to delegate both PDAs to
 * the MagicBlock PER and open the sealed-bid window.
 */

import { getAddressEncoder, type Address, type Instruction, type TransactionSigner } from '@solana/kit';
import { getCreateCompetitionInstruction } from 'tyche-generated-core';
import { getCreateAuctionInstruction } from 'tyche-generated-auction';
import {
  getCompetitionStatePda,
  getAuctionStatePda,
  getProtocolConfigPda,
} from '../pdas';
import {
  SOFT_CLOSE_WINDOW_SECS,
  SOFT_CLOSE_EXTENSION_SECS,
  MAX_SOFT_CLOSES,
  ASSET_TYPE_NFT,
} from '../constants';
import type { AssetType } from '../constants';

const addrEnc = getAddressEncoder();

/** Parameters for creating a new Tyche sealed-bid auction. */
export type CreateAuctionParams = {
  /**
   * The competition authority (seller / auction creator). Must sign the transaction.
   */
  authority: TransactionSigner;

  /**
   * The fee payer for rent. May be the same signer as `authority`.
   */
  payer: TransactionSigner;

  /**
   * Numeric ID for this competition (u64). Must be unique per `authority`.
   * Use a random u64 or an auto-incrementing counter.
   */
  competitionId: bigint;

  /**
   * Unix timestamp (seconds) when the competition opens for bids.
   * Set to current time to open immediately on activation.
   */
  startTime: bigint;

  /**
   * Duration of the bidding window in seconds (u64).
   * e.g. `3600n` for 1 hour.
   */
  durationSecs: bigint;

  /**
   * Minimum reserve price in lamports. A failed auction results if no bid
   * meets or exceeds this threshold.
   */
  reservePrice: bigint;

  /**
   * Mint address of the asset being auctioned (NFT or in-game item).
   * Stored in `AuctionState.asset_mint` for reference.
   */
  assetMint: Address;

  /**
   * Minimum amount (in lamports) a new bid must exceed the current high bid.
   * @default 1_000_000n (0.001 SOL)
   */
  minBidIncrement?: bigint;

  /**
   * Asset type.
   * @default ASSET_TYPE_NFT (0)
   */
  assetType?: AssetType;

  /** Seconds before end_time that arm a soft-close extension (default: 300). */
  softCloseWindow?: bigint;
  /** Seconds added to end_time on each soft-close trigger (default: 300). */
  softCloseExtension?: bigint;
  /** Maximum soft-close extensions per competition (default: 5). */
  maxSoftCloses?: number;
};

export type CreateAuctionResult = {
  /** Address of the CompetitionState PDA. */
  competitionAddress: Address;
  /** Address of the AuctionState PDA. */
  auctionStateAddress: Address;
  /** Ordered instructions — include in your transaction in this order. */
  instructions: Instruction[];
};

/**
 * Builds the `CreateCompetition` + `CreateAuction` instructions.
 *
 * @example
 * ```ts
 * import { buildCreateAuctionTransaction } from 'tyche-sdk';
 * import {
 *   pipe,
 *   createTransactionMessage,
 *   setTransactionMessageFeePayerSigner,
 *   appendTransactionMessageInstructions,
 *   setTransactionMessageLifetimeUsingBlockhash,
 * } from '@solana/kit';
 *
 * const { competitionAddress, auctionStateAddress, instructions } =
 *   await buildCreateAuctionTransaction({
 *     authority,
 *     payer,
 *     competitionId: 1n,
 *     startTime: BigInt(Math.floor(Date.now() / 1000)),
 *     durationSecs: 3600n,
 *     reservePrice: 1_000_000_000n, // 1 SOL
 *     assetMint: myNftMint,
 *   });
 *
 * const tx = pipe(
 *   createTransactionMessage({ version: 0 }),
 *   tx => setTransactionMessageFeePayerSigner(payer, tx),
 *   tx => appendTransactionMessageInstructions(instructions, tx),
 *   tx => setTransactionMessageLifetimeUsingBlockhash(blockhash, tx),
 * );
 * ```
 */
export async function buildCreateAuctionTransaction(
  params: CreateAuctionParams,
): Promise<CreateAuctionResult> {
  const {
    authority,
    payer,
    competitionId,
    startTime,
    durationSecs,
    reservePrice,
    assetMint,
    minBidIncrement = 1_000_000n,
    assetType = ASSET_TYPE_NFT,
    softCloseWindow = SOFT_CLOSE_WINDOW_SECS,
    softCloseExtension = SOFT_CLOSE_EXTENSION_SECS,
    maxSoftCloses = MAX_SOFT_CLOSES,
  } = params;

  // Encode the competition ID as 8 little-endian bytes (CompetitionState.id field)
  const idBytes = new Uint8Array(8);
  new DataView(idBytes.buffer).setBigUint64(0, competitionId, true);

  // Derive PDAs sequentially: AuctionState depends on CompetitionState address
  const [competitionAddress] = await getCompetitionStatePda(authority.address, competitionId);
  const [[auctionStateAddress], [protocolConfigAddress]] = await Promise.all([
    getAuctionStatePda(competitionAddress),
    getProtocolConfigPda(),
  ]);

  // Encode assetMint address to raw 32 bytes (instruction data expects ReadonlyUint8Array)
  const assetMintBytes = addrEnc.encode(assetMint);

  const createCompetitionIx = getCreateCompetitionInstruction({
    competition: competitionAddress,
    authority,
    payer,
    protocolConfig: protocolConfigAddress,
    id: idBytes,
    assetType,
    pad: new Uint8Array(6),
    startTime,
    durationSecs,
    softCloseWindow,
    softCloseExtension,
    maxSoftCloses,
    pad2: new Uint8Array(2),
    reservePrice,
  });

  const createAuctionIx = getCreateAuctionInstruction({
    auctionState: auctionStateAddress,
    competition: competitionAddress,
    authority,
    payer,
    assetMint: assetMintBytes,
    minBidIncrement,
  });

  return {
    competitionAddress,
    auctionStateAddress,
    instructions: [createCompetitionIx, createAuctionIx],
  };
}
