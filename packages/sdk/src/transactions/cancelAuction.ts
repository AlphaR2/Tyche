/**
 * buildCancelAuctionTransaction
 *
 * Produces the two instructions to cancel an auction:
 *   1. `CancelCompetition` (tyche-core)  — transitions competition to Cancelled
 *   2. `CancelAuction`    (tyche-auction) — closes the AuctionState account
 *
 * Cancellation is only possible when there are **zero participants** (no bids
 * have been placed). Once any bidder has called `PlaceBid`, the competition
 * cannot be cancelled — it must proceed to settlement.
 *
 * If the competition was already activated (delegated to PER), the MagicBlock
 * accounts (`permission`, `magicContext`, `magicProgram`) are required to
 * undelegate the CompetitionState. For pre-activation cancellations, pass the
 * zero address (`'11111111111111111111111111111111'`) for these accounts.
 */

import type { Address, Instruction, TransactionSigner } from '@solana/kit';
import { buildCancelCompetitionIx, buildCancelAuctionIx } from '../rawInstructions.js';

/** The system program address (zero pubkey equivalent for optional MagicBlock accounts). */
const SYSTEM_PROGRAM = '11111111111111111111111111111111' as Address;

export type CancelAuctionParams = {
  /**
   * Competition authority (seller). Must sign.
   */
  authority: TransactionSigner;

  /**
   * Address of the CompetitionState PDA.
   */
  competitionAddress: Address;

  /**
   * Address of the AuctionState PDA.
   */
  auctionStateAddress: Address;

  /**
   * Account that receives the reclaimed rent from the closed AuctionState.
   * Usually the authority or payer.
   */
  rentRecipient: Address;

  /**
   * MagicBlock accounts required if the competition was already activated
   * (delegated to PER). Pass `undefined` for pre-activation cancellations —
   * the system program address will be used as a placeholder.
   */
  magicBlock?: {
    /** MagicBlock ACL permission PDA for the CompetitionState. */
    permission: Address;
    /** MagicBlock context account. */
    magicContext: Address;
    /** MagicBlock program address. */
    magicProgram: Address;
  };
};

export type CancelAuctionResult = {
  /** Ordered instructions — include in your transaction in this order. */
  instructions: Instruction[];
};

/**
 * Builds the `CancelCompetition` + `CancelAuction` instructions.
 *
 * @example
 * ```ts
 * // Pre-activation cancel (no MagicBlock accounts needed)
 * const { instructions } = buildCancelAuctionTransaction({
 *   authority,
 *   competitionAddress,
 *   auctionStateAddress,
 *   rentRecipient: authority.address,
 * });
 * ```
 */
export function buildCancelAuctionTransaction(
  params: CancelAuctionParams,
): CancelAuctionResult {
  const {
    authority,
    competitionAddress,
    auctionStateAddress,
    rentRecipient,
    magicBlock,
  } = params;

  const permission = magicBlock?.permission ?? SYSTEM_PROGRAM;
  const magicContext = magicBlock?.magicContext ?? SYSTEM_PROGRAM;
  const magicProgram = magicBlock?.magicProgram ?? SYSTEM_PROGRAM;

  const cancelCompetitionIx = buildCancelCompetitionIx({
    competition:  competitionAddress,
    authority,
    permission,
    magicContext,
    magicProgram,
  });

  const cancelAuctionIx = buildCancelAuctionIx({
    auctionState:  auctionStateAddress,
    competition:   competitionAddress,
    authority,
    rentRecipient,
  });

  return {
    instructions: [cancelCompetitionIx, cancelAuctionIx],
  };
}
