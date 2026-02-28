/**
 * This SDK targets @solana/kit (web3.js v2).
 *
 * If your project or any dependency still uses @solana/web3.js v1, use the
 * `@solana/compat` bridge package to convert between v1 and v2 types:
 *
 * ```bash
 * npm i @solana/compat
 * ```
 * ```ts
 * import { fromVersionedTransaction, toPublicKey } from '@solana/compat';
 * ```
 * @see https://github.com/solana-labs/solana-web3.js/tree/master/packages/compat
 */

import type { Address } from '@solana/kit';

// ── Program Addresses ────────────────────────────────────────────────────────

export const TYCHE_CORE_PROGRAM_ADDRESS =
  'TYCANGQk6tumtij3tHwsRPSNkSHU3KGSNxNG59qJrHx' as Address<'TYCANGQk6tumtij3tHwsRPSNkSHU3KGSNxNG59qJrHx'>;

export const TYCHE_ESCROW_PROGRAM_ADDRESS =
  'TYEhGGkbujScDqPK1KTKCRu9cjVzjBH2Yf9Jb5L5Xtk' as Address<'TYEhGGkbujScDqPK1KTKCRu9cjVzjBH2Yf9Jb5L5Xtk'>;

export const TYCHE_AUCTION_PROGRAM_ADDRESS =
  'TYAKZZsLmYU65ScdqSGz6GxXs9KaUKF8sCFU52qmNTG' as Address<'TYAKZZsLmYU65ScdqSGz6GxXs9KaUKF8sCFU52qmNTG'>;

// ── MagicBlock Addresses ─────────────────────────────────────────────────────

/** MagicBlock delegation program — owns all delegation PDAs. */
export const MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS =
  'DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh' as Address<'DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh'>;

/**
 * MagicBlock ACL permission program.
 * Owns the `permission` PDA required by `ActivateCompetition` / `ActivateAuction`.
 * Source: ephemeral-rollups-pinocchio crate, PERMISSION_PROGRAM_ID constant.
 */
export const MAGICBLOCK_PERMISSION_PROGRAM_ADDRESS =
  'ACLseoPoyC3cBqoUtkbjZ4aDrkurZW86v19pXz2XQnp1' as Address<'ACLseoPoyC3cBqoUtkbjZ4aDrkurZW86v19pXz2XQnp1'>;

// ── MagicBlock Router RPC Endpoints ─────────────────────────────────────────

/**
 * MagicBlock Router — mainnet endpoint.
 *
 * Use this as your single RPC URL. The router implements the full Solana RPC
 * API and adds `getBlockhashForAccounts`, which returns a blockhash from the
 * correct node (mainnet or PER) based on whether the involved accounts are
 * currently delegated.
 *
 * @see https://docs.magicblock.gg/router
 */
export const MAGICBLOCK_ROUTER_MAINNET_URL = 'https://router.magicblock.app';

/** MagicBlock Router — devnet endpoint. */
export const MAGICBLOCK_ROUTER_DEVNET_URL = 'https://devnet-router.magicblock.app';

// ── PDA Seeds ────────────────────────────────────────────────────────────────
// These must stay in sync with crates/tyche-common/src/seeds.rs

export const COMPETITION_SEED = new TextEncoder().encode('competition');
export const PARTICIPANT_SEED = new TextEncoder().encode('participant');
export const VAULT_SEED = new TextEncoder().encode('vault');
export const AUCTION_SEED = new TextEncoder().encode('auction');
export const BID_SEED = new TextEncoder().encode('bid');
export const PROTOCOL_CONFIG_SEED = new TextEncoder().encode('protocol_config');

// MagicBlock delegation PDA seeds (from ephemeral-rollups-pinocchio crate)
export const DELEGATION_BUFFER_SEED = new TextEncoder().encode('delegation-buffer');
export const DELEGATION_RECORD_SEED = new TextEncoder().encode('delegation-record');
export const DELEGATION_METADATA_SEED = new TextEncoder().encode('delegation-metadata');

// MagicBlock ACL permission PDA seed (from ephemeral-rollups-pinocchio crate)
// Permission PDA: seeds = [b"permission:", authority_bytes]
export const PERMISSION_SEED = new TextEncoder().encode('permission:');

// ── Protocol Limits ──────────────────────────────────────────────────────────
// These must stay in sync with crates/tyche-common/src/constants.rs

/** Default soft-close window in seconds (5 minutes). */
export const SOFT_CLOSE_WINDOW_SECS = 300n;

/** Default soft-close extension added to end_time in seconds (5 minutes). */
export const SOFT_CLOSE_EXTENSION_SECS = 300n;

/** Default maximum number of soft-close extensions per competition. */
export const MAX_SOFT_CLOSES = 5;

/** Maximum number of bidders per competition (bounds the settlement scan). */
export const MAX_PARTICIPANTS = 1_000;

/** Maximum protocol fee in basis points (10%). Cannot be raised by config. */
export const MAX_FEE_BASIS_POINTS = 1_000;

/** Minimum lamports above rent-exemption a competition must hold at settlement. */
export const COMPETITION_MIN_LAMPORTS = 10_000n;

// ── Asset Types ──────────────────────────────────────────────────────────────

export const ASSET_TYPE_NFT = 0 as const;
export const ASSET_TYPE_IN_GAME_ITEM = 1 as const;
export type AssetType = typeof ASSET_TYPE_NFT | typeof ASSET_TYPE_IN_GAME_ITEM;

// ── Competition Phases ───────────────────────────────────────────────────────

export const PHASE_SCHEDULED = 0 as const;
export const PHASE_ACTIVE = 1 as const;
export const PHASE_SETTLING = 2 as const;
export const PHASE_SETTLED = 3 as const;
export const PHASE_CANCELLED = 4 as const;
export type CompetitionPhase =
  | typeof PHASE_SCHEDULED
  | typeof PHASE_ACTIVE
  | typeof PHASE_SETTLING
  | typeof PHASE_SETTLED
  | typeof PHASE_CANCELLED;
