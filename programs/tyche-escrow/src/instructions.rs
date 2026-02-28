use shank::ShankInstruction;
use crate::args::deposit::DepositArgs;

/// All instructions exposed by `tyche-escrow`.
///
/// Shank reads this enum to generate the IDL `"instructions"` array.
/// Discriminators follow the Anchor `sha256("global:<InstructionName>")[0..8]`
/// convention — the same values computed in `discriminator.rs`.
///
/// Account ordering here mirrors the processor `*Accounts` structs exactly.
#[derive(ShankInstruction)]
pub enum TycheEscrowInstruction {
    // ── Deposit

    /// Create a new `EscrowVault` and lock `amount` SOL (first call),
    /// or top up an existing vault with additional SOL (subsequent calls).
    ///
    /// The vault holds `rent + cumulative_amount` lamports.
    /// `EscrowVault::amount` tracks only the bid total — never the rent portion.
    #[account(0, writable, name = "vault",
        desc = "EscrowVault PDA. Seeds: [vault, competition, depositor]")]
    #[account(1, signer, writable, name = "depositor",
        desc = "Bidder — pays the bid amount; vault owner")]
    #[account(2, signer, writable, name = "payer",
        desc = "Funds vault rent on first deposit; may equal depositor")]
    #[account(3, name = "competition",
        desc = "CompetitionState PDA (read-only) — phase verified as Active")]
    #[account(4, name = "system_program",
        desc = "Required for CreateAccount and Transfer CPIs")]
    Deposit(DepositArgs),

    // ── Release

    /// Release vault funds after a competition settles.
    ///
    /// Crank-only. Requires competition to be `Settled` and the depositor to be
    /// the confirmed winner (`ParticipantRecord::is_winner == IS_WINNER`).
    ///
    /// No caller-supplied args. All values are read from on-chain state:
    /// - `vault.amount` is the canonical purchase price
    /// - fee rate and treasury address come from `ProtocolConfig`
    ///
    /// Lamport distribution:
    /// - protocol fee (`vault.amount × fee_basis_points / 10_000`) → treasury
    /// - net bid (`vault.amount` − fee) → competition authority (seller)
    /// - rent reserve (`vault.lamports()` − `vault.amount`) → original depositor
    ///
    /// Vault is closed (data zeroed, lamports drained) after this instruction.
    #[account(0, writable, name = "vault",
        desc = "EscrowVault PDA — drained and closed by this instruction")]
    #[account(1, writable, name = "authority",
        desc = "Competition authority — receives net bid amount after protocol fee")]
    #[account(2, writable, name = "depositor",
        desc = "Original depositor — receives excess collateral and rent reserve back")]
    #[account(3, signer, name = "crank",
        desc = "Protocol crank keypair — must equal config.crank_authority")]
    #[account(4, name = "competition",
        desc = "CompetitionState PDA (read-only) — phase verified as Settled")]
    #[account(5, name = "participant_record",
        desc = "ParticipantRecord PDA (read-only) — is_winner verified as IS_WINNER")]
    #[account(6, name = "protocol_config",
        desc = "ProtocolConfig PDA (read-only) — provides fee_basis_points, crank_authority, treasury")]
    #[account(7, writable, name = "treasury",
        desc = "Protocol treasury — receives the protocol fee; must match config.treasury")]
    Release,

    // ── Refund

    /// Return the full vault balance (bid + rent) to the depositor.
    ///
    /// Valid when:
    /// - Competition is `Cancelled` (any depositor may claim regardless of winner status), OR
    /// - Competition is `Settled` and depositor is **not** the winner.
    ///
    /// Winners must use `Release` instead. Vault is closed after this instruction.
    #[account(0, writable, name = "vault",
        desc = "EscrowVault PDA — drained and closed by this instruction")]
    #[account(1, signer, writable, name = "depositor",
        desc = "Original depositor — receives all lamports back")]
    #[account(2, name = "competition",
        desc = "CompetitionState PDA (read-only) — phase verified as Cancelled or Settled")]
    #[account(3, name = "participant_record",
        desc = "ParticipantRecord PDA (read-only) — is_winner checked on Settled path; ignored on Cancelled path")]
    Refund,
}
