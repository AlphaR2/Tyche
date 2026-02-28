use shank::ShankInstruction;
use crate::instruction_args::{
    activate::ActivateCompetitionArgs,
    create_competition::CreateCompetitionArgs,
    initialize_protocol_config::InitializeProtocolConfigArgs,
    settle::SettleCompetitionArgs,
    update_crank_authority::UpdateCrankAuthorityArgs,
    update_protocol_config::UpdateProtocolConfigArgs,
};

/// All instructions exposed by `tyche-core`.
#[derive(ShankInstruction)]
pub enum TycheCoreInstruction {
    #[account(0, writable, name = "competition",
        desc = "CompetitionState PDA. Seeds: [competition, authority, id]")]
    #[account(1, signer, name = "authority",
        desc = "Creator and owner of this competition")]
    #[account(2, signer, writable, name = "payer",
        desc = "Funds rent for the CompetitionState account")]
    #[account(3, name = "system_program",
        desc = "Required for the CreateAccount CPI")]
    #[account(4, name = "protocol_config",
        desc = "ProtocolConfig PDA (read-only) — validates reserve_price, duration, soft-close cap")]
    CreateCompetition(CreateCompetitionArgs),

    #[account(0, writable, name = "competition",
        desc = "CompetitionState PDA — delegated to PER after this instruction")]
    #[account(1, signer, name = "authority",
        desc = "Must match state.authority")]
    #[account(2, signer, writable, name = "payer",
        desc = "Funds permission account creation; may equal authority")]
    #[account(3, writable, name = "permission",
        desc = "MagicBlock ACL permission PDA for CompetitionState")]
    #[account(4, writable, name = "delegation_buffer",
        desc = "MagicBlock delegation buffer PDA")]
    #[account(5, writable, name = "delegation_record",
        desc = "MagicBlock delegation record PDA")]
    #[account(6, writable, name = "delegation_metadata",
        desc = "MagicBlock delegation metadata PDA")]
    #[account(7, name = "delegation_program",
        desc = "MagicBlock delegation program")]
    #[account(8, name = "permission_program",
        desc = "MagicBlock ACL permission program")]
    #[account(9, name = "system_program",
        desc = "Required for permission account creation CPI")]
    #[account(10, name = "validator",
        desc = "MagicBlock validator that will host the PER session")]
    ActivateCompetition(ActivateCompetitionArgs),

    #[account(0, writable, name = "competition",
        desc = "CompetitionState PDA — currently delegated to PER")]
    #[account(1, signer, name = "crank",
        desc = "Protocol crank keypair — must equal config.crank_authority")]
    #[account(2, writable, name = "magic_context",
        desc = "MagicBlock context account for commit_accounts CPI")]
    #[account(3, name = "magic_program",
        desc = "MagicBlock program")]
    #[account(4, name = "protocol_config",
        desc = "ProtocolConfig PDA (read-only) — provides crank_authority")]
    ExtendCompetition,

    #[account(0, writable, name = "competition",
        desc = "CompetitionState PDA — currently delegated to PER")]
    #[account(1, signer, name = "crank",
        desc = "Protocol crank keypair — must equal config.crank_authority")]
    #[account(2, writable, name = "permission",
        desc = "MagicBlock ACL permission PDA — undelegated alongside competition")]
    #[account(3, writable, name = "magic_context",
        desc = "MagicBlock context account for commit_and_undelegate_accounts CPI")]
    #[account(4, name = "magic_program",
        desc = "MagicBlock program")]
    #[account(5, name = "protocol_config",
        desc = "ProtocolConfig PDA (read-only) — provides crank_authority")]
    CloseCompetition,

    #[account(0, writable, name = "competition",
        desc = "CompetitionState PDA — back on mainnet after undelegation")]
    #[account(1, signer, name = "crank",
        desc = "Protocol crank keypair — must equal config.crank_authority")]
    #[account(2, name = "delegation_record",
        desc = "MagicBlock delegation record PDA — must have zero lamports")]
    #[account(3, name = "protocol_config",
        desc = "ProtocolConfig PDA (read-only) — provides crank_authority")]
    #[account(4, writable, name = "winner_participant_record",
        desc = "ParticipantRecord PDA of the winner — IS_WINNER written when args.winner is non-zero")]
    SettleCompetition(SettleCompetitionArgs),

    #[account(0, writable, name = "competition",
        desc = "CompetitionState PDA")]
    #[account(1, signer, name = "authority",
        desc = "Must match state.authority")]
    #[account(2, writable, name = "permission",
        desc = "MagicBlock ACL permission PDA — undelegated on Active path")]
    #[account(3, writable, name = "magic_context",
        desc = "MagicBlock context account — used on Active path only")]
    #[account(4, name = "magic_program",
        desc = "MagicBlock program — used on Active path only")]
    CancelCompetition,

    #[account(0, writable, name = "competition",
        desc = "CompetitionState PDA — participant_count incremented on first bid")]
    #[account(1, writable, name = "participant_record",
        desc = "ParticipantRecord PDA. Seeds: [participant, competition, bidder]")]
    #[account(2, signer, name = "bidder",
        desc = "The wallet placing the bid")]
    #[account(3, signer, writable, name = "payer",
        desc = "Funds rent for ParticipantRecord on first bid")]
    #[account(4, name = "system_program",
        desc = "Required for ParticipantRecord CreateAccount CPI on first bid")]
    RegisterBid,

    #[account(0, writable, name = "protocol_config",
        desc = "ProtocolConfig PDA. Seeds: [protocol_config]")]
    #[account(1, signer, name = "authority",
        desc = "Becomes config.authority; controls future updates")]
    #[account(2, signer, writable, name = "payer",
        desc = "Funds rent for the ProtocolConfig account")]
    #[account(3, name = "system_program",
        desc = "Required for the CreateAccount CPI")]
    InitializeProtocolConfig(InitializeProtocolConfigArgs),

    #[account(0, writable, name = "protocol_config",
        desc = "ProtocolConfig PDA — mutated in place")]
    #[account(1, signer, name = "authority",
        desc = "Must match config.authority")]
    UpdateProtocolConfig(UpdateProtocolConfigArgs),

    #[account(0, writable, name = "protocol_config",
        desc = "ProtocolConfig PDA — crank_authority mutated in place")]
    #[account(1, signer, name = "authority",
        desc = "Must match config.authority")]
    UpdateCrankAuthority(UpdateCrankAuthorityArgs),
}
