import {
  getAddressDecoder,
  type Address,
  type Rpc,
  type GetAccountInfoApi,
} from '@solana/kit';
import { fetchCompetitionState } from 'tyche-generated-core';
import { fetchAuctionState } from 'tyche-generated-auction';
import type { DecodedCompetition, DecodedAuction, CompetitionPhase } from './types';
import {
  PHASE_SCHEDULED,
  PHASE_ACTIVE,
  PHASE_SETTLING,
  PHASE_SETTLED,
} from './constants';

// Re-export the generated fetchers as the primary account API.
export {
  fetchCompetitionState,
  fetchMaybeCompetitionState,
  fetchAllCompetitionState,
  fetchParticipantRecord,
  fetchMaybeParticipantRecord,
  fetchProtocolConfig,
  fetchMaybeProtocolConfig,
} from 'tyche-generated-core';

export {
  fetchAuctionState,
  fetchMaybeAuctionState,
  fetchBidRecord,
  fetchMaybeBidRecord,
} from 'tyche-generated-auction';

export {
  fetchEscrowVault,
  fetchMaybeEscrowVault,
} from 'tyche-generated-escrow';

// ── Rich decoded helpers ─────────────────────────────────────────────────────

type MinRpc = Rpc<GetAccountInfoApi>;

const addrDec = getAddressDecoder();

/** Decodes a 32-byte Uint8Array to a base58 Address. */
function bytesToAddress(bytes: Uint8Array | readonly number[]): Address {
  const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  return addrDec.decode(u8) as Address;
}

/** Reads 8 little-endian bytes as a bigint (u64). */
function leU64(bytes: Uint8Array | readonly number[]): bigint {
  const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  return new DataView(u8.buffer, u8.byteOffset, 8).getBigUint64(0, true);
}

function toPhase(raw: number): CompetitionPhase {
  switch (raw) {
    case PHASE_SCHEDULED: return 'scheduled';
    case PHASE_ACTIVE:    return 'active';
    case PHASE_SETTLING:  return 'settling';
    case PHASE_SETTLED:   return 'settled';
    default:              return 'cancelled';
  }
}

/**
 * Fetches a `CompetitionState` and returns a `DecodedCompetition` with
 * human-readable fields (string addresses, bigint IDs, phase enum).
 */
export async function fetchDecodedCompetition(
  rpc: MinRpc,
  address: Address,
): Promise<DecodedCompetition> {
  const acct = await fetchCompetitionState(rpc, address);
  const d = acct.data;

  const settlementBytes = d.settlementRef as unknown as Uint8Array;
  const isZero = settlementBytes.every((b) => b === 0);

  return {
    address,
    authority: bytesToAddress(d.authority as unknown as Uint8Array),
    id: leU64(d.id as unknown as Uint8Array),
    phase: toPhase(d.phase),
    startTime: d.startTime,
    endTime: d.endTime,
    softCloseWindow: d.softCloseWindow,
    softCloseExtension: d.softCloseExtension,
    softCloseCount: d.softCloseCount,
    maxSoftCloses: d.maxSoftCloses,
    reservePrice: d.reservePrice,
    participantCount: d.participantCount,
    settlementRef: isZero ? null : bytesToAddress(settlementBytes),
    raw: d,
  };
}

/**
 * Fetches an `AuctionState` and returns a `DecodedAuction` with human-readable
 * fields.
 *
 * **Note:** `currentHighBid` is always `0n` and `currentWinner` is always `null`
 * during the active phase — these fields are sealed inside the PER TEE.
 */
export async function fetchDecodedAuction(
  rpc: MinRpc,
  address: Address,
): Promise<DecodedAuction> {
  const acct = await fetchAuctionState(rpc, address);
  const d = acct.data;

  const winnerBytes = d.currentWinner as unknown as Uint8Array;
  const winnerIsZero = winnerBytes.every((b) => b === 0);

  return {
    address,
    competition: bytesToAddress(d.competition as unknown as Uint8Array),
    authority: bytesToAddress(d.authority as unknown as Uint8Array),
    assetMint: bytesToAddress(d.assetMint as unknown as Uint8Array),
    minBidIncrement: d.minBidIncrement,
    currentHighBid: d.currentHighBid,
    currentWinner: winnerIsZero ? null : bytesToAddress(winnerBytes),
    bidCount: d.bidCount,
    raw: d,
  };
}
