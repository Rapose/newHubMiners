use anchor_lang::prelude::*;

#[account]
pub struct LootboxMinerState {
    pub lootbox_id: u64,
    pub owner: Pubkey,

    pub committed: bool,
    pub revealed: bool,

    pub commit_slot: u64,
    pub commitment: [u8; 32],

    pub rarity: u8,
    pub element: u8,
    pub hash_base: u64,

    pub face: u8,
    pub helmet: u8,
    pub backpack: u8,
    pub jacket: u8,
    pub item: u8,
    pub background: u8,

    pub bump: u8,
}

impl LootboxMinerState {
    pub const LEN: usize = 8  // discriminator
        + 8 // lootbox_id
        + 32 // owner
        + 1 // committed
        + 1 // revealed
        + 8 // commit_slot
        + 32 // commitment
        + 1 // rarity
        + 1 // element
        + 8 // hash_base
        + 1 // face
        + 1 // helmet
        + 1 // backpack
        + 1 // jacket
        + 1 // item
        + 1 // background
        + 1; // bump
}
