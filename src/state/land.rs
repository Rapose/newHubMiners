use anchor_lang::prelude::*;

#[account]
pub struct LandState {
    pub id: u64,
    pub owner: Pubkey,
    pub rarity: u8,
    pub element: u8,
    pub slots: u8,
    pub listed: bool,
    pub allocated_miners_count: u16,
    pub created_at: i64,
    pub bump: u8,
}

impl LandState {
    pub const LEN: usize = 8 + 8 + 32 + 1 + 1 + 1 + 1 + 2 + 8 + 1;
}
