import { getAddressEncoder, getProgramDerivedAddress, type Address } from '@solana/kit';
import {
  TYCHE_CORE_PROGRAM_ADDRESS,
  TYCHE_ESCROW_PROGRAM_ADDRESS,
  TYCHE_AUCTION_PROGRAM_ADDRESS,
  MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS,
  MAGICBLOCK_PERMISSION_PROGRAM_ADDRESS,
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
} from './constants';

// Encode a base58 Address to its raw 32-byte representation.
const addrEnc = getAddressEncoder();
function ab(address: Address): Uint8Array {
  return addrEnc.encode(address);
}

// Encode a u64 (bigint) to an 8-byte little-endian Uint8Array.
function u64Le(value: bigint): Uint8Array {
  const buf = new Uint8Array(8);
  new DataView(buf.buffer).setBigUint64(0, value, true);
  return buf;
}

// ── Tyche PDAs ───────────────────────────────────────────────────────────────

/**
 * Derives the `CompetitionState` PDA.
 *
 * Seeds: `[b"competition", authority, id_le_bytes]`
 *
 * @param authority - The competition creator's address.
 * @param id        - Numeric competition ID (u64). Must match the `id` passed to
 *                    `CreateCompetition`.
 */
export function getCompetitionStatePda(
  authority: Address,
  id: bigint,
): Promise<readonly [Address, number]> {
  return getProgramDerivedAddress({
    programAddress: TYCHE_CORE_PROGRAM_ADDRESS,
    seeds: [COMPETITION_SEED, ab(authority), u64Le(id)],
  });
}

/**
 * Derives the `ParticipantRecord` PDA.
 *
 * Seeds: `[b"participant", competition, participant]`
 */
export function getParticipantRecordPda(
  competition: Address,
  participant: Address,
): Promise<readonly [Address, number]> {
  return getProgramDerivedAddress({
    programAddress: TYCHE_CORE_PROGRAM_ADDRESS,
    seeds: [PARTICIPANT_SEED, ab(competition), ab(participant)],
  });
}

/**
 * Derives the `EscrowVault` PDA.
 *
 * Seeds: `[b"vault", competition, depositor]`
 */
export function getEscrowVaultPda(
  competition: Address,
  depositor: Address,
): Promise<readonly [Address, number]> {
  return getProgramDerivedAddress({
    programAddress: TYCHE_ESCROW_PROGRAM_ADDRESS,
    seeds: [VAULT_SEED, ab(competition), ab(depositor)],
  });
}

/**
 * Derives the `AuctionState` PDA.
 *
 * Seeds: `[b"auction", competition]`
 */
export function getAuctionStatePda(
  competition: Address,
): Promise<readonly [Address, number]> {
  return getProgramDerivedAddress({
    programAddress: TYCHE_AUCTION_PROGRAM_ADDRESS,
    seeds: [AUCTION_SEED, ab(competition)],
  });
}

/**
 * Derives the `BidRecord` PDA.
 *
 * Seeds: `[b"bid", auction_state, bidder]`
 */
export function getBidRecordPda(
  auctionState: Address,
  bidder: Address,
): Promise<readonly [Address, number]> {
  return getProgramDerivedAddress({
    programAddress: TYCHE_AUCTION_PROGRAM_ADDRESS,
    seeds: [BID_SEED, ab(auctionState), ab(bidder)],
  });
}

/**
 * Derives the singleton `ProtocolConfig` PDA.
 *
 * Seeds: `[b"protocol_config"]`
 */
export function getProtocolConfigPda(): Promise<readonly [Address, number]> {
  return getProgramDerivedAddress({
    programAddress: TYCHE_CORE_PROGRAM_ADDRESS,
    seeds: [PROTOCOL_CONFIG_SEED],
  });
}

// ── MagicBlock Delegation PDAs ───────────────────────────────────────────────
// Required when activating (delegating) a competition or auction to the PER.
// Seeds are from the ephemeral-rollups-pinocchio crate.

/**
 * Derives the MagicBlock delegation buffer PDA for a delegated account.
 *
 * Seeds: `[b"delegation-buffer", account]`
 */
export function getDelegationBufferPda(
  account: Address,
): Promise<readonly [Address, number]> {
  return getProgramDerivedAddress({
    programAddress: MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS,
    seeds: [DELEGATION_BUFFER_SEED, ab(account)],
  });
}

/**
 * Derives the MagicBlock delegation record PDA for a delegated account.
 *
 * Seeds: `[b"delegation-record", account]`
 */
export function getDelegationRecordPda(
  account: Address,
): Promise<readonly [Address, number]> {
  return getProgramDerivedAddress({
    programAddress: MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS,
    seeds: [DELEGATION_RECORD_SEED, ab(account)],
  });
}

/**
 * Derives the MagicBlock delegation metadata PDA for a delegated account.
 *
 * Seeds: `[b"delegation-metadata", account]`
 */
export function getDelegationMetadataPda(
  account: Address,
): Promise<readonly [Address, number]> {
  return getProgramDerivedAddress({
    programAddress: MAGICBLOCK_DELEGATION_PROGRAM_ADDRESS,
    seeds: [DELEGATION_METADATA_SEED, ab(account)],
  });
}

// ── MagicBlock ACL Permission PDA ─────────────────────────────────────────────

/**
 * Derives the MagicBlock ACL permission PDA for an authority address.
 *
 * Seeds: `[b"permission:", authority_bytes]`
 *
 * The permission PDA is created once per authority by the MagicBlock ACL
 * program (`ACLseoPoyC3cBqoUtkbjZ4aDrkurZW86v19pXz2XQnp1`).  Pass the
 * returned address as the `permission` account and
 * `MAGICBLOCK_PERMISSION_PROGRAM_ADDRESS` as `permissionProgram` when calling
 * `buildActivateAuctionTransaction`.
 *
 * @param authority - The address that will sign `ActivateCompetition` /
 *                    `ActivateAuction` (i.e. the competition creator).
 */
export function getPermissionPda(authority: Address): Promise<readonly [Address, number]> {
  return getProgramDerivedAddress({
    programAddress: MAGICBLOCK_PERMISSION_PROGRAM_ADDRESS,
    seeds: [PERMISSION_SEED, ab(authority)],
  });
}
