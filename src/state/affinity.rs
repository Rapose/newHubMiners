use anchor_lang::prelude::*;

#[account]
pub struct AffinityResult {
    pub user: Pubkey,
    pub land_element: u8,
    pub miner_element: u8,
    pub bps: u16,
    pub bump: u8,
}

impl AffinityResult {
    pub const LEN: usize = 8 + 32 + 1 + 1 + 2 + 1;
}
