use spl_governance_addin_api::voter_weight::VoterWeightAction;
use pinocchio::error::ProgramError;

/// Instruction arguments for `UpdateVoterWeightRecord`.
pub struct UpdateVoterWeightRecordArgs {
    /// The action this record is being prepared for.
    /// Defaults to `CastVote` if not provided (backward compatibility).
    pub action: VoterWeightAction,
}

impl UpdateVoterWeightRecordArgs {
    pub fn load(data: &[u8]) -> Result<Self, ProgramError> {
        // Backward compatibility: no args = CastVote
        if data.is_empty() {
            return Ok(Self {
                action: VoterWeightAction::CastVote,
            });
        }

        // We expect exactly one byte
        let action = match data[0] {
            0 => VoterWeightAction::CastVote,
            1 => VoterWeightAction::CommentProposal,
            2 => VoterWeightAction::CreateGovernance,
            3 => VoterWeightAction::CreateProposal,
            4 => VoterWeightAction::SignOffProposal,
            _ => return Err(ProgramError::InvalidInstructionData),
        };

        Ok(Self { action })
    }
}