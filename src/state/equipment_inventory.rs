use crate::constants::MAX_ITEM_LEVEL;
use anchor_lang::prelude::*;

#[account]
pub struct EquipmentInventoryState {
    pub owner: Pubkey,

    pub new_hand: [u16; MAX_ITEM_LEVEL + 1],
    pub new_head: [u16; MAX_ITEM_LEVEL + 1],

    pub broken_hand: [u16; MAX_ITEM_LEVEL + 1],
    pub broken_head: [u16; MAX_ITEM_LEVEL + 1],

    pub new_hand_remelted: [u16; MAX_ITEM_LEVEL + 1],
    pub new_head_remelted: [u16; MAX_ITEM_LEVEL + 1],

    pub broken_hand_remelted: [u16; MAX_ITEM_LEVEL + 1],
    pub broken_head_remelted: [u16; MAX_ITEM_LEVEL + 1],

    pub bump: u8,
}

impl EquipmentInventoryState {
    pub const LEN: usize = 32 +
        (8 * (MAX_ITEM_LEVEL + 1) * 2) + // 8 arrays * u16
        1;
}
