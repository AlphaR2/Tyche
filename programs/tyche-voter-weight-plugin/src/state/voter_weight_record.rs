/// SPL Governance voter-weight addin — VoterWeightRecord account.
///
/// Account layout (Borsh-compatible, 164 bytes max when all Option fields are Some):
///
/// ```text
///  [0..8]     account_discriminator  [u8; 8]
///  [8..40]    realm                  [u8; 32]
///  [40..72]   governing_token_mint   [u8; 32]
///  [72..104]  governing_token_owner  [u8; 32]
///  [104..112] voter_weight           u64 LE
///  [112]      expiry tag             u8
///  [113..121] expiry value           u64 LE (if Some)
///  [121]      action tag             u8
///  [122]      action value           u8 enum (if Some)
///  [123]      target tag             u8
///  [124..156] target value           [u8; 32] (if Some)
///  [156..164] reserved               [u8; 8]
/// ```
///
/// Total max size: 164 bytes.
pub const VOTER_WEIGHT_RECORD_MAX_SIZE: usize = 164;

use spl_governance_addin_api::voter_weight::{
    VoterWeightRecord as SplVoterWeightRecord,
    VoterWeightAction as SplVoterWeightAction,
};

use crate::discriminator::VOTER_WEIGHT_RECORD;

/// Wrapper around the SPL Governance VoterWeightRecord
/// providing deterministic low-level serialization.
pub struct VoterWeightRecord {
    pub inner: SplVoterWeightRecord,
}

impl VoterWeightRecord {
    /// Serializes the record into the provided byte buffer.
    pub fn write_to(&self, data: &mut [u8]) {
        let main = &self.inner;

        write_voter_weight_record(
            data,
            &VOTER_WEIGHT_RECORD,
            main.realm.as_array(),
            main.governing_token_mint.as_array(),
            main.governing_token_owner.as_array(),
            main.voter_weight,
            main.voter_weight_expiry,
            map_action(main.weight_action.clone()),
            main.weight_action_target.map(|t| t.to_bytes()),
        );
    }
}

/// Action for which the voter weight is being provided.
///
/// Ordinal values MUST match spl-governance-addin-api exactly.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoterWeightAction {
    CastVote         = 0,
    CommentProposal  = 1,
    CreateGovernance = 2,
    CreateProposal   = 3,
    SignOffProposal  = 4,
}

/// Safe mapping from SPL enum → local repr(u8) enum.
#[inline]
fn map_action(action: Option<SplVoterWeightAction>) -> Option<VoterWeightAction> {
    action.map(|a| match a {
        SplVoterWeightAction::CastVote         => VoterWeightAction::CastVote,
        SplVoterWeightAction::CommentProposal  => VoterWeightAction::CommentProposal,
        SplVoterWeightAction::CreateGovernance => VoterWeightAction::CreateGovernance,
        SplVoterWeightAction::CreateProposal   => VoterWeightAction::CreateProposal,
        SplVoterWeightAction::SignOffProposal  => VoterWeightAction::SignOffProposal,
    })
}

/// Writes a Borsh-compatible VoterWeightRecord into `data`.
///
/// `data` must be at least `VOTER_WEIGHT_RECORD_MAX_SIZE` bytes.
#[inline]
pub fn write_voter_weight_record(
    data:          &mut [u8],
    discriminator: &[u8; 8],
    realm:         &[u8; 32],
    mint:          &[u8; 32],
    owner:         &[u8; 32],
    voter_weight:  u64,
    expiry:        Option<u64>,
    action:        Option<VoterWeightAction>,
    target:        Option<[u8; 32]>,
) {
    debug_assert!(data.len() >= VOTER_WEIGHT_RECORD_MAX_SIZE);

    let mut c: usize = 0;

    // discriminator
    data[c..c + 8].copy_from_slice(discriminator);
    c += 8;

    // realm
    data[c..c + 32].copy_from_slice(realm);
    c += 32;

    // governing_token_mint
    data[c..c + 32].copy_from_slice(mint);
    c += 32;

    // governing_token_owner
    data[c..c + 32].copy_from_slice(owner);
    c += 32;

    // voter_weight (u64 LE)
    data[c..c + 8].copy_from_slice(&voter_weight.to_le_bytes());
    c += 8;

    // expiry Option<u64>
    match expiry {
        None => {
            data[c] = 0;
            c += 1;
        }
        Some(slot) => {
            data[c] = 1;
            c += 1;
            data[c..c + 8].copy_from_slice(&slot.to_le_bytes());
            c += 8;
        }
    }

    // action Option<VoterWeightAction>
    match action {
        None => {
            data[c] = 0;
            c += 1;
        }
        Some(a) => {
            data[c] = 1;
            c += 1;
            data[c] = a as u8;
            c += 1;
        }
    }

    // target Option<[u8; 32]>
    match target {
        None => {
            data[c] = 0;
            c += 1;
        }
        Some(t) => {
            data[c] = 1;
            c += 1;
            data[c..c + 32].copy_from_slice(&t);
            c += 32;
        }
    }

    // reserved [u8; 8]
    data[c..c + 8].copy_from_slice(&[0u8; 8]);
    c += 8;

    debug_assert!(c <= VOTER_WEIGHT_RECORD_MAX_SIZE);
}