use anchor_lang::prelude::*;

#[account]
pub struct MinerState {
    pub id: u64,
    pub owner: Pubkey,
    pub rarity: u8,  // 0..4
    pub element: u8, // 0..4
    pub hash_base: u64,
    pub face: u8,
    pub helmet: u8,
    pub backpack: u8,
    pub jacket: u8,
    pub item: u8,
    pub background: u8,

    pub allocated_land: Pubkey,
    pub listed: bool,

    pub created_at: i64,
    pub bump: u8,
}

impl MinerState {
    pub const LEN: usize = 8 + 8 + 32 + 1 + 1 + 8 + 1 + 1 + 1 + 1 + 1 + 1 + 32 + 1 + 8 + 1;
}
