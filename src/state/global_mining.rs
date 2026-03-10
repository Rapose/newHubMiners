use anchor_lang::prelude::*;

#[account]
pub struct GlobalMiningState {
    pub admin: Pubkey,

    pub week_index: u64,
    pub week_start_ts: i64,

    pub tick_len_sec: u32,
    pub weekly_pool_amount: u64,
    pub total_ep_tw: u128,

    pub frozen: bool,
    pub frozen_week_index: u64,
    pub frozen_weekly_pool_amount: u64,
    pub frozen_total_ep_tw: u128,

    pub bump: u8,
}

impl GlobalMiningState {
    pub const LEN: usize = 8 + 32 + 8 + 8 + 4 + 8 + 16 + 1 + 8 + 8 + 16 + 1;
}

#[account]
pub struct MinerMiningState {
    pub owner: Pubkey,
    pub miner: Pubkey, // MinerState PDA
    pub week_index: u64,

    pub last_tick: u32,
    pub ep_tw: u128,
    pub claimed: bool,

    pub bump: u8,
}

impl MinerMiningState {
    pub const LEN: usize = 8   // disc
        + 32   // owner
        + 32   // miner
        + 8    // week_index
        + 4    // last_tick
        + 16   // ep_tw
        + 1    // claimed
        + 1; // bump
}
