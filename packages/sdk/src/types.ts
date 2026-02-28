/**
 * Augmented types for the Tyche SDK.
 *
 * Re-exports the raw Codama-generated account types alongside richer,
 * human-readable variants used throughout the SDK.
 */

import type { Address } from '@solana/kit';
import type {
  CompetitionState as RawCompetitionState,
} from 'tyche-generated-core';
import type {
  AuctionState as RawAuctionState,
} from 'tyche-generated-auction';

export type {
  CompetitionState,
  ParticipantRecord,
  ProtocolConfig,
} from 'tyche-generated-core';

export type {
  AuctionState,
  BidRecord,
} from 'tyche-generated-auction';

export type { EscrowVault } from 'tyche-generated-escrow';

// ── Rich decoded types ───────────────────────────────────────────────────────

export type CompetitionPhase =
  | 'scheduled'
  | 'active'
  | 'settling'
  | 'settled'
  | 'cancelled';

/** A `CompetitionState` decoded with human-readable fields. */
export type DecodedCompetition = {
  /** On-chain address of the CompetitionState PDA. */
  address: Address;
  /** Competition authority (creator). */
  authority: Address;
  /** Competition ID as a bigint (u64 little-endian, 8 bytes). */
  id: bigint;
  /** Current phase of the competition. */
  phase: CompetitionPhase;
  /** Unix timestamp (seconds) when the competition becomes active. */
  startTime: bigint;
  /** Unix timestamp (seconds) when the competition closes (absent soft-close). */
  endTime: bigint;
  softCloseWindow: bigint;
  softCloseExtension: bigint;
  softCloseCount: number;
  maxSoftCloses: number;
  reservePrice: bigint;
  participantCount: number;
  /** Non-zero after settlement — the off-chain reference (e.g. AuctionState pubkey). */
  settlementRef: Address | null;
  raw: RawCompetitionState;
};

/** An `AuctionState` decoded with human-readable fields. */
export type DecodedAuction = {
  /** On-chain address of the AuctionState PDA. */
  address: Address;
  competition: Address;
  authority: Address;
  /** Mint of the asset being auctioned. */
  assetMint: Address;
  minBidIncrement: bigint;
  /**
   * Current highest bid in lamports.
   * **Sealed (always 0) during the active phase** — the value is only revealed
   * after the competition is settled and the account is undelegated from PER.
   */
  currentHighBid: bigint;
  /**
   * Current winning bidder.
   * **Sealed (null) during the active phase** — same reason as `currentHighBid`.
   */
  currentWinner: Address | null;
  bidCount: number;
  raw: RawAuctionState;
};

/**
 * MagicBlock delegation accounts required for `ActivateCompetition` and
 * `ActivateAuction`. Derive these with the helpers in `pdas.ts` or fetch them
 * from the MagicBlock SDK.
 */
export type MagicBlockDelegationAccounts = {
  /** `[b"delegation-buffer", account]` under the delegation program. */
  buffer: Address;
  /** `[b"delegation-record", account]` under the delegation program. */
  delegationRecord: Address;
  /** `[b"delegation-metadata", account]` under the delegation program. */
  delegationMetadata: Address;
  /** Address of the MagicBlock delegation program. */
  delegationProgram: Address;
  /**
   * TEE validator node address that will host the PER session.
   * Obtain this from MagicBlock's infrastructure docs or validator registry.
   */
  validator: Address;
};

/** Extended form of `MagicBlockDelegationAccounts` needed for `ActivateCompetition`. */
export type MagicBlockActivateCompetitionAccounts = MagicBlockDelegationAccounts & {
  /** MagicBlock ACL permission PDA for the CompetitionState. */
  permission: Address;
  /** Address of the MagicBlock ACL permission program. */
  permissionProgram: Address;
};
