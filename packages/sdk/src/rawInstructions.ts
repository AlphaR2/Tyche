/**
 * Raw instruction builders for all Tyche programs.
 *
 * Constructs instruction data manually from the binary layouts defined in each
 * program's `instruction_args/` or `args/` module.  Bypasses Codama-generated
 * builders to avoid discriminator mismatches (Codama emits 1-byte ordinal
 * discriminators; the on-chain programs require 8-byte SHA256-derived values).
 *
 * These builders are **internal** to tyche-sdk and are not exported from the
 * public index.  External callers should use the higher-level transaction
 * builders (buildCreateAuctionTransaction, etc.) instead.
 *
 * Layout reference per program:
 *   tyche-core/src/instruction_args/
 *   tyche-escrow/src/args/
 *   tyche-auction/src/args/
 */

import {
  AccountRole,
  getAddressEncoder,
  type Address,
  type Instruction,
  type TransactionSigner,
} from '@solana/kit';
import {
  TYCHE_CORE_PROGRAM_ADDRESS,
  TYCHE_ESCROW_PROGRAM_ADDRESS,
  TYCHE_AUCTION_PROGRAM_ADDRESS,
} from './constants.js';

const enc = getAddressEncoder();

/** Solana system program address — used as a placeholder for optional MagicBlock accounts. */
const SYSTEM_PROGRAM = '11111111111111111111111111111111' as Address;

// ── Discriminators (SHA256("global:<name>")[0..8]) ──────────────────────────

// tyche-core
const CREATE_COMPETITION_DISC   = new Uint8Array([110, 212, 234, 212, 118, 128, 158, 244]);
const CANCEL_COMPETITION_DISC   = new Uint8Array([ 62,   4, 198,  98, 200,  41, 255,  72]);
const ACTIVATE_COMPETITION_DISC = new Uint8Array([153, 105, 130,  88, 198, 208,  30, 118]);

// tyche-escrow
const DEPOSIT_DISC = new Uint8Array([242,  35, 198, 137,  82, 225, 242, 182]);
const REFUND_DISC  = new Uint8Array([  2,  96, 183, 251,  63, 208,  46,  46]);
const RELEASE_DISC = new Uint8Array([253, 249,  15, 206,  28, 127, 193, 241]);

// tyche-auction
const CREATE_AUCTION_DISC   = new Uint8Array([234,   6, 201, 246,  47, 219, 176, 107]);
const ACTIVATE_AUCTION_DISC = new Uint8Array([212,  24, 210,   7, 183, 147,  66, 109]);
const PLACE_BID_DISC        = new Uint8Array([238,  77, 148,  91, 200, 151,  92, 146]);
const CANCEL_AUCTION_DISC   = new Uint8Array([156,  43, 197, 110, 218, 105, 143, 182]);

// ── tyche-core builders ─────────────────────────────────────────────────────

/**
 * CreateCompetition — 96 bytes total (8 disc + 88 args).
 *
 * CreateCompetitionArgs (#[repr(C)] bytemuck::Pod, 88 bytes):
 *   off  8..40  id                   [u8; 32]
 *   off 40      asset_type           u8
 *   off 41..48  _pad                 [u8; 7]
 *   off 48..56  start_time           i64 LE
 *   off 56..64  duration_secs        i64 LE
 *   off 64..72  soft_close_window    i64 LE
 *   off 72..80  soft_close_extension i64 LE
 *   off 80      max_soft_closes      u8
 *   off 81..88  _pad2                [u8; 7]
 *   off 88..96  reserve_price        u64 LE
 *
 * Accounts: [competition(w), authority(rs), payer(ws), system_program(r), protocol_config(r)]
 */
export function buildCreateCompetitionIx(args: {
  competition:        Address;
  authority:          TransactionSigner;
  payer:              TransactionSigner;
  protocolConfig:     Address;
  /** 32-byte id array (u64 competitionId stored LE in first 8 bytes, rest zeroed). */
  id:                 Uint8Array;
  assetType:          number;
  startTime:          bigint;
  durationSecs:       bigint;
  softCloseWindow:    bigint;
  softCloseExtension: bigint;
  maxSoftCloses:      number;
  reservePrice:       bigint;
}): Instruction {
  const data = new Uint8Array(96);
  const view = new DataView(data.buffer);
  let off = 0;

  data.set(CREATE_COMPETITION_DISC, off);              off += 8;  // discriminator
  data.set(args.id.slice(0, 32), off);                 off += 32; // id [u8;32]
  view.setUint8(off, args.assetType);                  off += 1;  // asset_type
  off += 7;                                                       // _pad
  view.setBigInt64(off, args.startTime, true);         off += 8;  // start_time
  view.setBigInt64(off, args.durationSecs, true);      off += 8;  // duration_secs
  view.setBigInt64(off, args.softCloseWindow, true);   off += 8;  // soft_close_window
  view.setBigInt64(off, args.softCloseExtension, true); off += 8; // soft_close_extension
  view.setUint8(off, args.maxSoftCloses);              off += 1;  // max_soft_closes
  off += 7;                                                       // _pad2
  view.setBigUint64(off, args.reservePrice, true);                // reserve_price

  return {
    programAddress: TYCHE_CORE_PROGRAM_ADDRESS,
    accounts: [
      { address: args.competition,       role: AccountRole.WRITABLE },
      { address: args.authority.address, role: AccountRole.READONLY_SIGNER, signer: args.authority },
      { address: args.payer.address,     role: AccountRole.WRITABLE_SIGNER, signer: args.payer },
      { address: SYSTEM_PROGRAM,         role: AccountRole.READONLY },
      { address: args.protocolConfig,    role: AccountRole.READONLY },
    ],
    data,
  } as Instruction;
}

/**
 * CancelCompetition — 8 bytes (discriminator only, no args).
 *
 * Accounts: [competition(w), authority(rs), permission(w), magic_context(w), magic_program(r)]
 *
 * On the Scheduled → Cancelled path, permission / magicContext / magicProgram are
 * passed but never touched by the processor.  Any valid addresses work.
 */
export function buildCancelCompetitionIx(args: {
  competition:  Address;
  authority:    TransactionSigner;
  permission:   Address;
  magicContext: Address;
  magicProgram: Address;
}): Instruction {
  return {
    programAddress: TYCHE_CORE_PROGRAM_ADDRESS,
    accounts: [
      { address: args.competition,       role: AccountRole.WRITABLE },
      { address: args.authority.address, role: AccountRole.READONLY_SIGNER, signer: args.authority },
      { address: args.permission,        role: AccountRole.WRITABLE },
      { address: args.magicContext,      role: AccountRole.WRITABLE },
      { address: args.magicProgram,      role: AccountRole.READONLY },
    ],
    data: CANCEL_COMPETITION_DISC,
  } as Instruction;
}

/**
 * ActivateCompetition — 12 bytes (8 disc + 4 u32 commit_frequency_ms).
 *
 * ActivateCompetitionArgs (#[repr(C)] bytemuck::Pod, 4 bytes):
 *   off  8..12  commit_frequency_ms  u32 LE
 *
 * Accounts: [competition(w), authority(rs), payer(ws), permission(w),
 *            delegation_buffer(w), delegation_record(w), delegation_metadata(w),
 *            delegation_program(r), permission_program(r), system_program(r), validator(r)]
 */
export function buildActivateCompetitionIx(args: {
  competition:        Address;
  authority:          TransactionSigner;
  payer:              TransactionSigner;
  permission:         Address;
  delegationBuffer:   Address;
  delegationRecord:   Address;
  delegationMetadata: Address;
  delegationProgram:  Address;
  permissionProgram:  Address;
  validator:          Address;
  commitFrequencyMs:  number;
}): Instruction {
  const data = new Uint8Array(12);
  data.set(ACTIVATE_COMPETITION_DISC, 0);
  new DataView(data.buffer).setUint32(8, args.commitFrequencyMs, true);

  return {
    programAddress: TYCHE_CORE_PROGRAM_ADDRESS,
    accounts: [
      { address: args.competition,        role: AccountRole.WRITABLE },
      { address: args.authority.address,  role: AccountRole.READONLY_SIGNER, signer: args.authority },
      { address: args.payer.address,      role: AccountRole.WRITABLE_SIGNER, signer: args.payer },
      { address: args.permission,         role: AccountRole.WRITABLE },
      { address: args.delegationBuffer,   role: AccountRole.WRITABLE },
      { address: args.delegationRecord,   role: AccountRole.WRITABLE },
      { address: args.delegationMetadata, role: AccountRole.WRITABLE },
      { address: args.delegationProgram,  role: AccountRole.READONLY },
      { address: args.permissionProgram,  role: AccountRole.READONLY },
      { address: SYSTEM_PROGRAM,          role: AccountRole.READONLY },
      { address: args.validator,          role: AccountRole.READONLY },
    ],
    data,
  } as Instruction;
}

// ── tyche-escrow builders ───────────────────────────────────────────────────

/**
 * Deposit — 16 bytes (8 disc + 8 u64 amount).
 *
 * DepositArgs (#[repr(C)] bytemuck::Pod, 8 bytes):
 *   off  8..16  amount  u64 LE
 *
 * Accounts: [vault(w), depositor(ws), payer(ws), competition(r), system_program(r)]
 */
export function buildDepositIx(args: {
  vault:       Address;
  depositor:   TransactionSigner;
  payer:       TransactionSigner;
  competition: Address;
  amount:      bigint;
}): Instruction {
  const data = new Uint8Array(16);
  data.set(DEPOSIT_DISC, 0);
  new DataView(data.buffer).setBigUint64(8, args.amount, true);

  return {
    programAddress: TYCHE_ESCROW_PROGRAM_ADDRESS,
    accounts: [
      { address: args.vault,             role: AccountRole.WRITABLE },
      { address: args.depositor.address, role: AccountRole.WRITABLE_SIGNER, signer: args.depositor },
      { address: args.payer.address,     role: AccountRole.WRITABLE_SIGNER, signer: args.payer },
      { address: args.competition,       role: AccountRole.READONLY },
      { address: SYSTEM_PROGRAM,         role: AccountRole.READONLY },
    ],
    data,
  } as Instruction;
}

/**
 * Refund — 8 bytes (discriminator only, no args).
 *
 * Accounts: [vault(w), depositor(ws), competition(r), participant_record(r)]
 */
export function buildRefundIx(args: {
  vault:             Address;
  depositor:         TransactionSigner;
  competition:       Address;
  participantRecord: Address;
}): Instruction {
  return {
    programAddress: TYCHE_ESCROW_PROGRAM_ADDRESS,
    accounts: [
      { address: args.vault,             role: AccountRole.WRITABLE },
      { address: args.depositor.address, role: AccountRole.WRITABLE_SIGNER, signer: args.depositor },
      { address: args.competition,       role: AccountRole.READONLY },
      { address: args.participantRecord, role: AccountRole.READONLY },
    ],
    data: REFUND_DISC,
  } as Instruction;
}

/**
 * Release — 8 bytes (discriminator only, crank-only, no args).
 *
 * Accounts: [vault(w), authority(w), depositor(w), crank(rs),
 *            competition(r), participant_record(r), protocol_config(r), treasury(w)]
 */
export function buildReleaseIx(args: {
  vault:             Address;
  authority:         Address;
  depositor:         Address;
  crank:             TransactionSigner;
  competition:       Address;
  participantRecord: Address;
  protocolConfig:    Address;
  treasury:          Address;
}): Instruction {
  return {
    programAddress: TYCHE_ESCROW_PROGRAM_ADDRESS,
    accounts: [
      { address: args.vault,             role: AccountRole.WRITABLE },
      { address: args.authority,         role: AccountRole.WRITABLE },
      { address: args.depositor,         role: AccountRole.WRITABLE },
      { address: args.crank.address,     role: AccountRole.READONLY_SIGNER, signer: args.crank },
      { address: args.competition,       role: AccountRole.READONLY },
      { address: args.participantRecord, role: AccountRole.READONLY },
      { address: args.protocolConfig,    role: AccountRole.READONLY },
      { address: args.treasury,          role: AccountRole.WRITABLE },
    ],
    data: RELEASE_DISC,
  } as Instruction;
}

// ── tyche-auction builders ──────────────────────────────────────────────────

/**
 * CreateAuction — 48 bytes (8 disc + 32 asset_mint + 8 min_bid_increment).
 *
 * CreateAuctionArgs (#[repr(C)] bytemuck::Pod, 40 bytes):
 *   off  8..40  asset_mint        Address [u8;32]
 *   off 40..48  min_bid_increment u64 LE
 *
 * Accounts: [auction_state(w), competition(r), authority(rs), payer(ws), system_program(r)]
 */
export function buildCreateAuctionIx(args: {
  auctionState:    Address;
  competition:     Address;
  authority:       TransactionSigner;
  payer:           TransactionSigner;
  assetMint:       Address;
  minBidIncrement: bigint;
}): Instruction {
  const data = new Uint8Array(48);
  const view = new DataView(data.buffer);
  data.set(CREATE_AUCTION_DISC, 0);
  data.set(enc.encode(args.assetMint), 8);          // asset_mint [u8;32] at off 8
  view.setBigUint64(40, args.minBidIncrement, true); // min_bid_increment u64 at off 40

  return {
    programAddress: TYCHE_AUCTION_PROGRAM_ADDRESS,
    accounts: [
      { address: args.auctionState,      role: AccountRole.WRITABLE },
      { address: args.competition,       role: AccountRole.READONLY },
      { address: args.authority.address, role: AccountRole.READONLY_SIGNER, signer: args.authority },
      { address: args.payer.address,     role: AccountRole.WRITABLE_SIGNER, signer: args.payer },
      { address: SYSTEM_PROGRAM,         role: AccountRole.READONLY },
    ],
    data,
  } as Instruction;
}

/**
 * ActivateAuction — 8 bytes (discriminator only, no args).
 *
 * Accounts: [auction_state(w), competition(r), authority(rs), buffer(w),
 *            delegation_record(w), delegation_metadata(w), delegation_program(r),
 *            system_program(r), validator(r)]
 */
export function buildActivateAuctionIx(args: {
  auctionState:       Address;
  competition:        Address;
  authority:          TransactionSigner;
  buffer:             Address;
  delegationRecord:   Address;
  delegationMetadata: Address;
  delegationProgram:  Address;
  validator:          Address;
}): Instruction {
  return {
    programAddress: TYCHE_AUCTION_PROGRAM_ADDRESS,
    accounts: [
      { address: args.auctionState,       role: AccountRole.WRITABLE },
      { address: args.competition,        role: AccountRole.READONLY },
      { address: args.authority.address,  role: AccountRole.READONLY_SIGNER, signer: args.authority },
      { address: args.buffer,             role: AccountRole.WRITABLE },
      { address: args.delegationRecord,   role: AccountRole.WRITABLE },
      { address: args.delegationMetadata, role: AccountRole.WRITABLE },
      { address: args.delegationProgram,  role: AccountRole.READONLY },
      { address: SYSTEM_PROGRAM,          role: AccountRole.READONLY },
      { address: args.validator,          role: AccountRole.READONLY },
    ],
    data: ACTIVATE_AUCTION_DISC,
  } as Instruction;
}

/**
 * PlaceBid — 16 bytes (8 disc + 8 u64 amount).
 *
 * PlaceBidArgs (#[repr(C)] bytemuck::Pod, 8 bytes):
 *   off  8..16  amount  u64 LE
 *
 * Accounts: [auction_state(w), competition(w), bid_record(w), vault(r),
 *            bidder(rs), payer(ws), tyche_core_program(r),
 *            competition_participant_record(w), system_program(r)]
 */
export function buildPlaceBidIx(args: {
  auctionState:                Address;
  competition:                 Address;
  bidRecord:                   Address;
  vault:                       Address;
  bidder:                      TransactionSigner;
  payer:                       TransactionSigner;
  tycheCoreProgram:            Address;
  competitionParticipantRecord: Address;
  amount:                      bigint;
}): Instruction {
  const data = new Uint8Array(16);
  data.set(PLACE_BID_DISC, 0);
  new DataView(data.buffer).setBigUint64(8, args.amount, true);

  return {
    programAddress: TYCHE_AUCTION_PROGRAM_ADDRESS,
    accounts: [
      { address: args.auctionState,                 role: AccountRole.WRITABLE },
      { address: args.competition,                  role: AccountRole.WRITABLE },
      { address: args.bidRecord,                    role: AccountRole.WRITABLE },
      { address: args.vault,                        role: AccountRole.READONLY },
      { address: args.bidder.address,               role: AccountRole.READONLY_SIGNER, signer: args.bidder },
      { address: args.payer.address,                role: AccountRole.WRITABLE_SIGNER, signer: args.payer },
      { address: args.tycheCoreProgram,             role: AccountRole.READONLY },
      { address: args.competitionParticipantRecord, role: AccountRole.WRITABLE },
      { address: SYSTEM_PROGRAM,                    role: AccountRole.READONLY },
    ],
    data,
  } as Instruction;
}

/**
 * CancelAuction — 8 bytes (discriminator only, no args).
 *
 * Accounts: [auction_state(w), competition(r), authority(rs), rent_recipient(w)]
 */
export function buildCancelAuctionIx(args: {
  auctionState:  Address;
  competition:   Address;
  authority:     TransactionSigner;
  rentRecipient: Address;
}): Instruction {
  return {
    programAddress: TYCHE_AUCTION_PROGRAM_ADDRESS,
    accounts: [
      { address: args.auctionState,      role: AccountRole.WRITABLE },
      { address: args.competition,       role: AccountRole.READONLY },
      { address: args.authority.address, role: AccountRole.READONLY_SIGNER, signer: args.authority },
      { address: args.rentRecipient,     role: AccountRole.WRITABLE },
    ],
    data: CANCEL_AUCTION_DISC,
  } as Instruction;
}
