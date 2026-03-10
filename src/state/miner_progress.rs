use anchor_lang::prelude::*;

#[account]
pub struct MinerProgress {
    pub miner: Pubkey,
    pub owner: Pubkey,
    pub level: u16,
    pub exp: u64,
    pub last_exp_claim_ts: i64,
    pub bump: u8,
}

impl MinerProgress {
    pub const LEN: usize = 8  // discriminator
        + 32 // miner
        + 32 // owner
        + 2  // level
        + 8  // exp
        + 8  // last_exp_claim_ts
        + 1; // bump
}
