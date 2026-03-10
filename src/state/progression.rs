use anchor_lang::prelude::*;

#[account]
pub struct ProgressionConfig {
    pub admin: Pubkey,

    pub mining_window_secs: i64,
    pub max_accrual_windows: u16,
    pub linear_hash_k_bps: u16,

    pub max_level_by_rarity: [u16; 5],

    pub exp_base_by_rarity: [u64; 5],
    pub exp_growth_bps: u16,

    pub ess_base_cost_by_rarity: [u64; 5],
    pub ess_growth_bps: u16,

    pub bump: u8,
}

impl ProgressionConfig {
    pub const LEN: usize = 8 + 32 + 8 + 2 + 2 + (2 * 5) + (8 * 5) + 2 + (8 * 5) + 2 + 1;
}
