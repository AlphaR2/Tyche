/// Instruction interface documentation for `tyche-voter-weight-plugin`.
///
/// Three instructions are exposed. All use the Anchor-compatible 8-byte discriminator
/// convention: `SHA256("global:<instruction_name>")[0..8]`.
///
/// # CreateRegistrar
///
/// One-time setup, called by the realm authority after deploying this plugin.
/// Creates the Registrar PDA that stores the plugin's configuration.
///
/// Account order:
/// ```text
/// 0: registrar           writable  PDA — seeds: [realm, "registrar", governing_token_mint]
/// 1: realm               read-only SPL Governance Realm account
/// 2: governing_token_mint read-only Community token mint of the realm
/// 3: realm_authority     signer    Must be the realm's authority
/// 4: payer               signer    writable — funds rent for the registrar account
/// 5: system_program      read-only
/// ```
///
/// Instruction data: `[discriminator: [u8; 8]] ++ [CreateRegistrarArgs: 96 bytes]`
/// Args layout: `[governance_program_id: [u8; 32]] ++ [competition: [u8; 32]] ++ [tyche_escrow_program: [u8; 32]]`
///
/// # CreateVoterWeightRecord
///
/// Creates the VoterWeightRecord PDA for a voter. Called once per voter, after
/// calling `tyche-escrow::Deposit`. The initial record has `voter_weight = 0`
/// and `voter_weight_expiry = Some(0)` (immediately expired).
///
/// Account order:
/// ```text
/// 0: voter_weight_record  writable  PDA — seeds: ["voter-weight-record", realm, mint, voter]
/// 1: registrar            read-only Plugin registrar for this realm/mint
/// 2: realm                read-only SPL Governance Realm account
/// 3: governing_token_mint read-only Community token mint of the realm
/// 4: voter_authority      signer    The voter creating their weight record
/// 5: payer                signer    writable — funds rent for the VWR account
/// 6: system_program       read-only
/// ```
///
/// Instruction data: `[discriminator: [u8; 8]]` (no additional args)
///
/// # UpdateVoterWeightRecord
///
/// Refreshes the VoterWeightRecord for the current slot by reading the voter's
/// EscrowVault balance. Must be called in the same transaction as `CastVote`.
///
/// `voter_weight` is set to `EscrowVault::amount` (deposited SOL in lamports).
/// `voter_weight_expiry` is set to `current_slot` — forcing same-tx update.
/// `weight_action` is set to `CastVote`.
/// `weight_action_target` is set to the proposal pubkey.
///
/// Account order:
/// ```text
/// 0: voter_weight_record  writable  The voter's VWR PDA to update
/// 1: registrar            read-only Plugin registrar — provides competition + escrow program
/// 2: escrow_vault         read-only EscrowVault PDA from tyche-escrow for this voter + competition
/// 3: voter_authority      signer    Must equal EscrowVault::depositor
/// 4: proposal             read-only The proposal being voted on (stored as weight_action_target)
/// ```
///
/// Instruction data: `[discriminator: [u8; 8]]` (no additional args)
pub struct PluginInstructions;
