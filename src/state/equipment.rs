use anchor_lang::prelude::*;


#[account]
pub struct EquipmentState {
    pub owner: Pubkey,
    pub miner: Pubkey,

    pub hand_level: u8,
    pub hand_power_bps: u16,
    pub hand_is_remelted: bool,

    pub head_level: u8,
    pub head_recharge_discount_bps: u16,
    pub head_is_remelted: bool,

    pub bump: u8,
}

impl EquipmentState {
    pub const LEN: usize = 32 + 32 + 1 + 2 + 1 + 1 + 2 + 1 + 1;
}
