use anchor_lang::prelude::*;

#[account]
pub struct LootboxLandState {
    pub lootbox_id: u64,
    pub owner: Pubkey,

    pub committed: bool,
    pub revealed: bool,

    pub commit_slot: u64,
    pub commitment: [u8; 32],

    pub rarity: u8,
    pub element: u8,
    pub slots: u8,

    pub bump: u8,
}

impl LootboxLandState {
    pub const LEN: usize = 8 + 8 + 32 + 1 + 1 + 8 + 32 + 1 + 1 + 1 + 1;
}
