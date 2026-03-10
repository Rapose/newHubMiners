use anchor_lang::prelude::*;

#[account]
pub struct Config {
    pub admin: Pubkey,
    pub paused: bool,
    pub next_miner_id: u64,
    pub next_land_id: u64,
    pub next_listing_id: u64,
    pub bump: u8,
}

impl Config {
    pub const LEN: usize = 8 + 32 + 1 + 8 + 8 + 8 + 1;
}
