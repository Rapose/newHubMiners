use anchor_lang::prelude::*;

#[repr(u8)]
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListingAssetKind {
    Miner = 1,
    Land = 2,
    EquipmentInventory = 3,
}

#[repr(u8)]
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum EquipmentInventoryBucket {
    NewHand = 1,
    NewHead = 2,
    BrokenHand = 3,
    BrokenHead = 4,
    NewHandRemelted = 5,
    NewHeadRemelted = 6,
    BrokenHandRemelted = 7,
    BrokenHeadRemelted = 8,
}

#[account]
pub struct ListingState {
    pub id: u64,
    pub seller: Pubkey,
    pub active: bool,
    pub asset_kind: u8,
    pub price_ess: u64,
    pub created_at: i64,

    pub miner: Pubkey,
    pub land: Pubkey,

    pub inventory_owner: Pubkey,
    pub equipment_bucket: u8,
    pub equipment_level: u8,
    pub equipment_amount: u16,

    pub bump: u8,
}

impl ListingState {
    pub const LEN: usize = 8 + 8 + 32 + 1 + 1 + 8 + 8 + 32 + 32 + 32 + 1 + 1 + 2 + 1;
}
