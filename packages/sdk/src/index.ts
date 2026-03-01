/**
 * @tyche-protocol/sdk (tyche-sdk)
 *
 * TypeScript SDK for the Tyche sealed-bid auction protocol on Solana.
 *
 * Quick start:
 * ```ts
 * import {
 *   buildCreateAuctionTransaction,
 *   buildActivateAuctionTransaction,
 *   buildPlaceBidTransaction,
 *   buildCancelAuctionTransaction,
 *   fetchDecodedAuction,
 *   fetchDecodedCompetition,
 *   getCompetitionStatePda,
 *   getAuctionStatePda,
 *   MAGICBLOCK_ROUTER_MAINNET_URL,
 *   TYCHE_CORE_PROGRAM_ADDRESS,
 * } from 'tyche-sdk';
 * import { createSolanaRpc } from '@solana/kit';
 *
 * const rpc = createSolanaRpc(MAGICBLOCK_ROUTER_MAINNET_URL);
 * ```
 */

// ── Constants ────────────────────────────────────────────────────────────────

export {
  // Program addresses
  TYCHE_CORE_PROGRAM_ADDRESS,
  TYCHE_ESCROW_PROGRAM_ADDRESS,
  TYCHE_AUCTION_PROGRAM_ADDRESS,

  // MagicBlock
  MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS,
  MAGICBLOCK_PERMISSION_PROGRAM_ADDRESS,
  MAGICBLOCK_ROUTER_MAINNET_URL,
  MAGICBLOCK_ROUTER_DEVNET_URL,

  // PDA seeds
  COMPETITION_SEED,
  PARTICIPANT_SEED,
  VAULT_SEED,
  AUCTION_SEED,
  BID_SEED,
  PROTOCOL_CONFIG_SEED,
  DELEGATION_BUFFER_SEED,
  DELEGATION_RECORD_SEED,
  DELEGATION_METADATA_SEED,
  PERMISSION_SEED,

  // Protocol limits
  SOFT_CLOSE_WINDOW_SECS,
  SOFT_CLOSE_EXTENSION_SECS,
  MAX_SOFT_CLOSES,
  MAX_PARTICIPANTS,
  MAX_FEE_BASIS_POINTS,
  COMPETITION_MIN_LAMPORTS,

  // Asset types
  ASSET_TYPE_NFT,
  ASSET_TYPE_IN_GAME_ITEM,

  // Phase constants
  PHASE_SCHEDULED,
  PHASE_ACTIVE,
  PHASE_SETTLING,
  PHASE_SETTLED,
  PHASE_CANCELLED,
} from './constants';

export type { AssetType, CompetitionPhase as CompetitionPhaseConst } from './constants';
export * from './router';

// ── PDA Derivations ──────────────────────────────────────────────────────────

export {
  getCompetitionStatePda,
  getParticipantRecordPda,
  getEscrowVaultPda,
  getAuctionStatePda,
  getBidRecordPda,
  getProtocolConfigPda,
  getDelegationBufferPda,
  getDelegationRecordPda,
  getDelegationMetadataPda,
  getPermissionPda,
} from './pdas';

// ── Account Fetchers ─────────────────────────────────────────────────────────

export {
  // Generated fetchers (low-level, return raw Codama types)
  fetchCompetitionState,
  fetchMaybeCompetitionState,
  fetchAllCompetitionState,
  fetchParticipantRecord,
  fetchMaybeParticipantRecord,
  fetchProtocolConfig,
  fetchMaybeProtocolConfig,
  fetchAuctionState,
  fetchMaybeAuctionState,
  fetchBidRecord,
  fetchMaybeBidRecord,
  fetchEscrowVault,
  fetchMaybeEscrowVault,

  // Rich decoded fetchers (return SDK types with human-readable fields)
  fetchDecodedCompetition,
  fetchDecodedAuction,
} from './accounts';

// ── Transaction Builders ─────────────────────────────────────────────────────

export { buildCreateAuctionTransaction } from './transactions/createAuction';
export type {
  CreateAuctionParams,
  CreateAuctionResult,
} from './transactions/createAuction';

export { buildActivateAuctionTransaction } from './transactions/activateAuction';
export type {
  ActivateAuctionParams,
  ActivateAuctionResult,
} from './transactions/activateAuction';

export { buildPlaceBidTransaction } from './transactions/placeBid';
export type {
  PlaceBidParams,
  PlaceBidResult,
} from './transactions/placeBid';

export { buildCancelAuctionTransaction } from './transactions/cancelAuction';
export type {
  CancelAuctionParams,
  CancelAuctionResult,
} from './transactions/cancelAuction';

// ── Error Handling ───────────────────────────────────────────────────────────

export {
  TycheError,
  TycheCoreErrorCode,
  TycheEscrowErrorCode,
  TycheAuctionErrorCode,
  parseTycheError,
  isTycheError,
} from './errors';

// ── Types ────────────────────────────────────────────────────────────────────

export type {
  // Raw account types (from Codama)
  CompetitionState,
  ParticipantRecord,
  ProtocolConfig,
  AuctionState,
  BidRecord,
  EscrowVault,

  // Rich SDK types
  DecodedCompetition,
  DecodedAuction,
  CompetitionPhase,

  // MagicBlock helper types
  MagicBlockDelegationAccounts,
  MagicBlockActivateCompetitionAccounts,
} from './types';
