use pinocchio::Address;
use shank::ShankAccount;
use bytemuck::{Pod, Zeroable};

/// On-chain state for a single Tyche CEE competition.
///
/// Owned by `tyche-core`. Asset-type agnostic that holds lifecycle state only.
/// Settlement result (`settlement_ref`) is written by `SettleCompetition`
/// and points to the vertical's outcome account as the canonical proof
/// once the competition reaches `Settled`.
///
/// # PDA
///
/// Seeds: `[b"competition", authority_pubkey, id_bytes]`
///
/// # Lifecycle
///
/// Created by `CreateCompetition` in the `Scheduled` phase. `end_time` is written 0 at
/// creation and computed as `clock.unix_timestamp + duration_secs` by `ActivateCompetition`.
/// Delegated to the MagicBlock PER by `ActivateCompetition`.
/// Sealed fields (`current_high_bid`, `current_winner` in `AuctionState`) are
/// unreadable outside the TEE while this account is delegated.
/// Undelegated and settled by `SettleCompetition`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, ShankAccount)]
pub struct CompetitionState {
  pub discriminator:        [u8; 8],
  pub id:                   Address,
  pub authority:            Address,
  pub asset_type:           u8,
  pub phase:                u8,
  pub _padding:             [u8; 6], // 6 bytes — align next i64 to 8-byte boundary
  pub start_time:           i64,
  pub end_time:             i64,
  pub soft_close_window:    i64,
  pub soft_close_extension: i64,
  pub soft_close_count:     u8,
  pub max_soft_closes:      u8,
  pub _padding2:            [u8; 6], // 6 bytes — align next u64 to 8-byte boundary
  pub reserve_price:        u64,
  pub participant_count:    u32,
  pub bump:                 u8,
  pub _padding3:            [u8; 3], // offset 133→136: keeps final_amount at 8-byte aligned offset (168)
  pub settlement_ref: Address,
  pub duration_secs:        i64,  // stored at creation; used by ActivateCompetition to compute end_time
}

impl CompetitionState {
    pub const LEN: usize = core::mem::size_of::<CompetitionState>();
}
