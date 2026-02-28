/**
 * Typed error codes for all three Tyche programs.
 *
 * Each program reserves a non-overlapping range:
 *   tyche-core   → 0x1400 – 0x14FF
 *   tyche-escrow → 0x2400 – 0x24FF
 *   tyche-auction → 0x3000 – 0x30FF
 */

// ── tyche-core error codes ───────────────────────────────────────────────────

export const TycheCoreErrorCode = {
  InvalidPhase: 0x1400,
  NotAuthority: 0x1401,
  AuctionNotStarted: 0x1402,
  AuctionNotExpired: 0x1403,
  SoftCloseCapReached: 0x1404,
  SoftCloseNotArmed: 0x1405,
  NotUndelegated: 0x1406,
  HasParticipants: 0x1407,
  ArithmeticOverflow: 0x1408,
  InvalidDiscriminator: 0x1409,
  InvalidCrankAuthority: 0x140a,
  InvalidAccountData: 0x1410,
  AuctionEnded: 0x1411,
  UnauthorizedCaller: 0x1412,
  ParticipantCapReached: 0x1413,
  ConfigAlreadyInitialized: 0x1414,
  FeeTooHigh: 0x1415,
  NotConfigAuthority: 0x1416,
  ProtocolPaused: 0x1417,
} as const;

export type TycheCoreErrorCode =
  (typeof TycheCoreErrorCode)[keyof typeof TycheCoreErrorCode];

// ── tyche-escrow error codes ─────────────────────────────────────────────────

export const TycheEscrowErrorCode = {
  InvalidPhase: 0x2400,
  InvalidCrankAuthority: 0x2401,
  InvalidDiscriminator: 0x2402,
  ArithmeticOverflow: 0x2403,
  NotWinner: 0x2404,
  WinnerCannotRefund: 0x2405,
  InvalidAmount: 0x2406,
  InvalidTreasury: 0x2407,
  InvalidProtocolConfig: 0x2408,
} as const;

export type TycheEscrowErrorCode =
  (typeof TycheEscrowErrorCode)[keyof typeof TycheEscrowErrorCode];

// ── tyche-auction error codes ────────────────────────────────────────────────

export const TycheAuctionErrorCode = {
  InvalidDiscriminator: 0x3000,
  InvalidPhase: 0x3001,
  NotAuthority: 0x3002,
  AuctionAlreadyExists: 0x3003,
  BidTooLow: 0x3004,
  InsufficientVault: 0x3005,
  NoWinner: 0x3006,
  NotCrank: 0x3007,
  NotDepositor: 0x3008,
  NotEscrowProgram: 0x3009,
  ArithmeticOverflow: 0x300a,
  InvalidCompetition: 0x300b,
  InvalidVault: 0x300c,
  InvalidPda: 0x300d,
} as const;

export type TycheAuctionErrorCode =
  (typeof TycheAuctionErrorCode)[keyof typeof TycheAuctionErrorCode];

// ── Typed error class ────────────────────────────────────────────────────────

type TycheProgram = 'tyche-core' | 'tyche-escrow' | 'tyche-auction';

const CORE_CODES = new Set<number>(Object.values(TycheCoreErrorCode));
const ESCROW_CODES = new Set<number>(Object.values(TycheEscrowErrorCode));
const AUCTION_CODES = new Set<number>(Object.values(TycheAuctionErrorCode));

const CORE_NAME_MAP = Object.fromEntries(
  Object.entries(TycheCoreErrorCode).map(([k, v]) => [v, k]),
) as Record<number, string>;

const ESCROW_NAME_MAP = Object.fromEntries(
  Object.entries(TycheEscrowErrorCode).map(([k, v]) => [v, k]),
) as Record<number, string>;

const AUCTION_NAME_MAP = Object.fromEntries(
  Object.entries(TycheAuctionErrorCode).map(([k, v]) => [v, k]),
) as Record<number, string>;

/** A typed on-chain error returned by one of the Tyche programs. */
export class TycheError extends Error {
  readonly program: TycheProgram;
  readonly code: number;
  readonly codeName: string;

  constructor(program: TycheProgram, code: number, codeName: string) {
    super(`[${program}] ${codeName} (0x${code.toString(16)})`);
    this.name = 'TycheError';
    this.program = program;
    this.code = code;
    this.codeName = codeName;
  }
}

/**
 * Attempts to parse an error thrown during a Solana transaction into a typed
 * `TycheError`. Returns `null` if the error is not from a Tyche program.
 *
 * @example
 * ```ts
 * try {
 *   await sendTransaction(tx);
 * } catch (err) {
 *   const tyche = parseTycheError(err);
 *   if (tyche?.codeName === 'BidTooLow') { ... }
 * }
 * ```
 */
export function parseTycheError(err: unknown): TycheError | null {
  // @solana/kit wraps custom program errors as objects with a `context.code`
  // or as `SendTransactionError` with logs. We extract the raw u32 code.
  const code = extractCustomCode(err);
  if (code === null) return null;

  if (CORE_CODES.has(code)) {
    return new TycheError('tyche-core', code, CORE_NAME_MAP[code] ?? 'Unknown');
  }
  if (ESCROW_CODES.has(code)) {
    return new TycheError('tyche-escrow', code, ESCROW_NAME_MAP[code] ?? 'Unknown');
  }
  if (AUCTION_CODES.has(code)) {
    return new TycheError('tyche-auction', code, AUCTION_NAME_MAP[code] ?? 'Unknown');
  }

  return null;
}

/** Returns true if `err` is a TycheError with the given code name. */
export function isTycheError(
  err: unknown,
  codeName: keyof typeof TycheCoreErrorCode | keyof typeof TycheEscrowErrorCode | keyof typeof TycheAuctionErrorCode,
): err is TycheError {
  if (!(err instanceof TycheError)) return false;
  return err.codeName === codeName;
}

// ── Internal helpers ─────────────────────────────────────────────────────────

function extractCustomCode(err: unknown): number | null {
  if (typeof err !== 'object' || err === null) return null;

  // @solana/kit SolanaError format
  const e = err as Record<string, unknown>;
  if (typeof e['code'] === 'number') return e['code'];

  // JSON-RPC error with nested InstructionError
  const msg = String((e as { message?: unknown })['message'] ?? '');
  const match = msg.match(/custom program error: (0x[0-9a-fA-F]+|\d+)/);
  if (match) {
    return parseInt(match[1], match[1].startsWith('0x') ? 16 : 10);
  }

  // Logs-based: "Program log: Error: Custom(12345)"
  const logsMatch = msg.match(/Custom\((\d+)\)/);
  if (logsMatch) return parseInt(logsMatch[1], 10);

  return null;
}
