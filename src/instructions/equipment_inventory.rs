use crate::constants::*;
use crate::errors::MoeError;
use crate::state::{Config, EquipmentInventoryState};
use anchor_lang::prelude::*;

pub fn handler(ctx: Context<EquipmentInventoryInit>) -> Result<()> {
    let inv = &mut ctx.accounts.inventory;
    inv.owner = ctx.accounts.owner.key();

    inv.new_hand = [0u16; MAX_ITEM_LEVEL + 1];
    inv.new_head = [0u16; MAX_ITEM_LEVEL + 1];

    inv.broken_hand = [0u16; MAX_ITEM_LEVEL + 1];
    inv.broken_head = [0u16; MAX_ITEM_LEVEL + 1];

    inv.new_hand_remelted = [0u16; MAX_ITEM_LEVEL + 1];
    inv.new_head_remelted = [0u16; MAX_ITEM_LEVEL + 1];

    inv.broken_hand_remelted = [0u16; MAX_ITEM_LEVEL + 1];
    inv.broken_head_remelted = [0u16; MAX_ITEM_LEVEL + 1];

    inv.bump = ctx.bumps.inventory;
    Ok(())
}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum EquipmentSlotKind {
    Hand,
    Head,
}




#[derive(Accounts)]
pub struct EquipmentInventoryInit<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        space = 8 + EquipmentInventoryState::LEN,
        seeds = [SEED_EQUIPMENT_INVENTORY, owner.key().as_ref()],
        bump
    )]
    pub inventory: Account<'info, EquipmentInventoryState>,

    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct EquipmentInventoryGrantItem<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [b"config"],
        bump = config.bump,
        constraint = config.admin == admin.key() @ MoeError::Unauthorized
    )]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [SEED_EQUIPMENT_INVENTORY, owner.key().as_ref()],
        bump = inventory.bump
    )]
    pub inventory: Account<'info, EquipmentInventoryState>,

    /// CHECK: usado apenas como seed owner do inventário
    pub owner: UncheckedAccount<'info>,
}

pub fn handler_grant_item(
    ctx: Context<EquipmentInventoryGrantItem>,
    slot: EquipmentSlotKind,
    level: u8,
    amount: u16,
    remelted: bool,
    broken: bool,
) -> Result<()> {
    require!(amount > 0, MoeError::NoItemAvailable);

    let i = level as usize;
    require!(level >= 1 && i <= MAX_ITEM_LEVEL, MoeError::InvalidBaseLevel);

    let inv = &mut ctx.accounts.inventory;

    match (slot, remelted, broken) {
        (EquipmentSlotKind::Hand, false, false) => {
            inv.new_hand[i] = inv.new_hand[i].saturating_add(amount);
        }
        (EquipmentSlotKind::Hand, false, true) => {
            inv.broken_hand[i] = inv.broken_hand[i].saturating_add(amount);
        }
        (EquipmentSlotKind::Hand, true, false) => {
            inv.new_hand_remelted[i] = inv.new_hand_remelted[i].saturating_add(amount);
        }
        (EquipmentSlotKind::Hand, true, true) => {
            inv.broken_hand_remelted[i] = inv.broken_hand_remelted[i].saturating_add(amount);
        }

        (EquipmentSlotKind::Head, false, false) => {
            inv.new_head[i] = inv.new_head[i].saturating_add(amount);
        }
        (EquipmentSlotKind::Head, false, true) => {
            inv.broken_head[i] = inv.broken_head[i].saturating_add(amount);
        }
        (EquipmentSlotKind::Head, true, false) => {
            inv.new_head_remelted[i] = inv.new_head_remelted[i].saturating_add(amount);
        }
        (EquipmentSlotKind::Head, true, true) => {
            inv.broken_head_remelted[i] = inv.broken_head_remelted[i].saturating_add(amount);
        }
    }

    Ok(())
}