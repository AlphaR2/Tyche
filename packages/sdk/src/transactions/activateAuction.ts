/**
 * buildActivateAuctionTransaction
 *
 * Produces the two instructions that delegate both Tyche PDAs to the
 * MagicBlock Private Ephemeral Rollup (PER), opening the sealed-bid window:
 *   1. `ActivateCompetition` (tyche-core)  — delegates CompetitionState to PER
 *   2. `ActivateAuction`    (tyche-auction) — delegates AuctionState to PER
 *
 * After these instructions are confirmed on mainnet, all `PlaceBid` calls must
 * be routed through the MagicBlock Router (PER node), not a standard Solana RPC.
 * Use `buildPlaceBidTransaction` which handles routing automatically via
 * `getBlockhashForAccounts`.
 *
 * Prerequisites:
 *   - `CreateCompetition` + `CreateAuction` must have been confirmed.
 *   - `CompetitionState.phase` must be `Scheduled`.
 *
 * MagicBlock accounts:
 *   Derive the delegation PDAs with the helpers in `pdas.ts`:
 *   ```ts
 *   const [compBuffer]   = await getDelegationBufferPda(competitionAddress);
 *   const [compRecord]   = await getDelegationRecordPda(competitionAddress);
 *   const [compMetadata] = await getDelegationMetadataPda(competitionAddress);
 *   const [aucBuffer]    = await getDelegationBufferPda(auctionStateAddress);
 *   const [aucRecord]    = await getDelegationRecordPda(auctionStateAddress);
 *   const [aucMetadata]  = await getDelegationMetadataPda(auctionStateAddress);
 *   ```
 *   The `permission` and `permissionProgram` accounts are from the MagicBlock ACL
 *   system — consult MagicBlock's documentation for the correct addresses.
 */

import type { Address, Instruction, TransactionSigner } from '@solana/kit';
import { buildActivateCompetitionIx, buildActivateAuctionIx } from '../rawInstructions.js';
import type { MagicBlockActivateCompetitionAccounts, MagicBlockDelegationAccounts } from '../types.js';
import { MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS } from '../constants.js';

export type ActivateAuctionParams = {
  /** Competition authority (seller). Must sign. */
  authority: TransactionSigner;
  /** Fee payer. May equal `authority`. */
  payer: TransactionSigner;

  /** Address of the CompetitionState PDA. */
  competitionAddress: Address;
  /** Address of the AuctionState PDA. */
  auctionStateAddress: Address;

  /**
   * MagicBlock delegation accounts for the CompetitionState.
   * Includes `permission` and `permissionProgram` (ACL).
   */
  competitionDelegation: MagicBlockActivateCompetitionAccounts;

  /**
   * MagicBlock delegation accounts for the AuctionState.
   * Does not need `permission` / `permissionProgram`.
   */
  auctionDelegation: MagicBlockDelegationAccounts;

  /**
   * Frequency (ms) at which the PER node commits state changes to mainnet.
   * Lower values increase settlement latency costs. Recommended: `1000`.
   */
  commitFrequencyMs?: number;
};

export type ActivateAuctionResult = {
  /** Ordered instructions — include in your transaction in this order. */
  instructions: Instruction[];
};

/**
 * Builds the `ActivateCompetition` + `ActivateAuction` instructions that
 * delegate both PDAs to the MagicBlock PER.
 *
 * @example
 * ```ts
 * import {
 *   buildActivateAuctionTransaction,
 *   getDelegationBufferPda,
 *   getDelegationRecordPda,
 *   getDelegationMetadataPda,
 *   MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS,
 * } from 'tyche-sdk';
 *
 * // Derive MagicBlock delegation PDAs
 * const [[compBuffer], [compRecord], [compMeta]] = await Promise.all([
 *   getDelegationBufferPda(competitionAddress),
 *   getDelegationRecordPda(competitionAddress),
 *   getDelegationMetadataPda(competitionAddress),
 * ]);
 *
 * const { instructions } = buildActivateAuctionTransaction({
 *   authority,
 *   payer,
 *   competitionAddress,
 *   auctionStateAddress,
 *   competitionDelegation: {
 *     buffer: compBuffer,
 *     delegationRecord: compRecord,
 *     delegationMetadata: compMeta,
 *     delegationProgram: MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS,
 *     validator: VALIDATOR_ADDRESS,
 *     permission: MY_PERMISSION_PDA,
 *     permissionProgram: MAGICBLOCK_ACL_PROGRAM_ADDRESS,
 *   },
 *   auctionDelegation: { ... },
 * });
 * ```
 */
export function buildActivateAuctionTransaction(
  params: ActivateAuctionParams,
): ActivateAuctionResult {
  const {
    authority,
    payer,
    competitionAddress,
    auctionStateAddress,
    competitionDelegation: cd,
    auctionDelegation: ad,
    commitFrequencyMs = 1000,
  } = params;

  const activateCompetitionIx = buildActivateCompetitionIx({
    competition:        competitionAddress,
    authority,
    payer,
    permission:         cd.permission,
    delegationBuffer:   cd.buffer,
    delegationRecord:   cd.delegationRecord,
    delegationMetadata: cd.delegationMetadata,
    delegationProgram:  MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS,
    permissionProgram:  cd.permissionProgram,
    validator:          cd.validator,
    commitFrequencyMs,
  });

  const activateAuctionIx = buildActivateAuctionIx({
    auctionState:       auctionStateAddress,
    competition:        competitionAddress,
    authority,
    buffer:             ad.buffer,
    delegationRecord:   ad.delegationRecord,
    delegationMetadata: ad.delegationMetadata,
    delegationProgram:  MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS,
    validator:          ad.validator,
  });

  return {
    instructions: [activateCompetitionIx, activateAuctionIx],
  };
}
