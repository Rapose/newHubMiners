use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};

use crate::constants::*;
use crate::errors::MoeError;
use crate::state::{EquipmentInventoryState, EquipmentState, MinerState};
use crate::utils::equipment_balance::{
    hand_power_bps_by_tier, head_discount_bps_by_tier, remelt_cost_ess, SLOT_HAND, SLOT_HEAD,
};

fn idx(level: u8) -> Result<usize> {
    if level < 1 {
        return err!(MoeError::InvalidBaseLevel);
    }
    let i = level as usize;
    if i > MAX_ITEM_LEVEL {
        return err!(MoeError::InvalidBaseLevel);
    }
    Ok(i)
}

fn inv_add_broken_hand(inv: &mut EquipmentInventoryState, level: u8, remelted: bool) -> Result<()> {
    let i = idx(level)?;
    if remelted {
        inv.broken_hand_remelted[i] = inv.broken_hand_remelted[i].saturating_add(1);
    } else {
        inv.broken_hand[i] = inv.broken_hand[i].saturating_add(1);
    }
    Ok(())
}

fn inv_add_broken_head(inv: &mut EquipmentInventoryState, level: u8, remelted: bool) -> Result<()> {
    let i = idx(level)?;
    if remelted {
        inv.broken_head_remelted[i] = inv.broken_head_remelted[i].saturating_add(1);
    } else {
        inv.broken_head[i] = inv.broken_head[i].saturating_add(1);
    }
    Ok(())
}

fn inv_consume_4_hand_non_remelted(
    inv: &mut EquipmentInventoryState,
    base_level: u8,
) -> Result<()> {
    let i = idx(base_level)?;

    let total = (inv.new_hand[i] as u32) + (inv.broken_hand[i] as u32);
    require!(total >= 4, MoeError::NotEnoughForRemelt);

    let mut need: u16 = 4;

    let take_broken = inv.broken_hand[i].min(need);
    inv.broken_hand[i] -= take_broken;
    need -= take_broken;

    if need > 0 {
        let take_new = inv.new_hand[i].min(need);
        inv.new_hand[i] -= take_new;
        need -= take_new;
    }

    require!(need == 0, MoeError::NotEnoughForRemelt);
    Ok(())
}

fn inv_consume_4_head_non_remelted(
    inv: &mut EquipmentInventoryState,
    base_level: u8,
) -> Result<()> {
    let i = idx(base_level)?;

    let total = (inv.new_head[i] as u32) + (inv.broken_head[i] as u32);
    require!(total >= 4, MoeError::NotEnoughForRemelt);

    let mut need: u16 = 4;

    let take_broken = inv.broken_head[i].min(need);
    inv.broken_head[i] -= take_broken;
    need -= take_broken;

    if need > 0 {
        let take_new = inv.new_head[i].min(need);
        inv.new_head[i] -= take_new;
        need -= take_new;
    }

    require!(need == 0, MoeError::NotEnoughForRemelt);
    Ok(())
}

pub fn handler_init(ctx: Context<EquipmentInit>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.miner_state.owner,
        ctx.accounts.owner.key(),
        MoeError::Unauthorized
    );
    require!(
        !ctx.accounts.miner_state.listed,
        MoeError::AssetListedLocked
    );

    let eq = &mut ctx.accounts.equipment;

    eq.owner = ctx.accounts.owner.key();
    eq.miner = ctx.accounts.miner_state.key();

    eq.hand_level = 0;
    eq.hand_power_bps = 0;
    eq.hand_is_remelted = false;

    eq.head_level = 0;
    eq.head_recharge_discount_bps = 0;
    eq.head_is_remelted = false;

    eq.bump = ctx.bumps.equipment;

    Ok(())
}

#[derive(Accounts)]
pub struct EquipmentInit<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    pub miner_state: Account<'info, MinerState>,

    #[account(
        init,
        payer = owner,
        space = 8 + EquipmentState::LEN,
        seeds = [SEED_EQUIPMENT, miner_state.key().as_ref()],
        bump
    )]
    pub equipment: Account<'info, EquipmentState>,

    pub system_program: Program<'info, System>,
}

// =========================
// REPLACE HAND
// =========================
fn inv_consume_1_hand_equippable(inv: &mut EquipmentInventoryState, level: u8) -> Result<bool> {
    let i = idx(level)?;

    if inv.new_hand[i] > 0 {
        inv.new_hand[i] -= 1;
        return Ok(false); // não remelted
    }

    if inv.new_hand_remelted[i] > 0 {
        inv.new_hand_remelted[i] -= 1;
        return Ok(true); // remelted
    }

    err!(MoeError::NoItemAvailable)
}

fn inv_consume_1_head_equippable(inv: &mut EquipmentInventoryState, level: u8) -> Result<bool> {
    let i = idx(level)?;

    if inv.new_head[i] > 0 {
        inv.new_head[i] -= 1;
        return Ok(false); // não remelted
    }

    if inv.new_head_remelted[i] > 0 {
        inv.new_head_remelted[i] -= 1;
        return Ok(true); // remelted
    }

    err!(MoeError::NoItemAvailable)
}

pub fn handler_replace_hand(ctx: Context<EquipmentReplaceHand>, new_level: u8) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.miner_state.owner,
        ctx.accounts.owner.key(),
        MoeError::Unauthorized
    );
    require_keys_eq!(
        ctx.accounts.equipment.owner,
        ctx.accounts.owner.key(),
        MoeError::Unauthorized
    );
    require_keys_eq!(
        ctx.accounts.equipment.miner,
        ctx.accounts.miner_state.key(),
        MoeError::InvalidMinerRef
    );

    require!(
        !ctx.accounts.miner_state.listed,
        MoeError::AssetListedLocked
    );

    let eq = &mut ctx.accounts.equipment;
    let inv = &mut ctx.accounts.inventory;

    // Regra: só substitui se nível maior
    require!(new_level > eq.hand_level, MoeError::InvalidUpgrade);

    // precisa ter 1 item NOVO não-remelted nesse nível
    let was_remelted = inv_consume_1_hand_equippable(inv, new_level)?;

    // item anterior vira quebrado (mantém flag remelted do item antigo)
    if eq.hand_level > 0 {
        inv_add_broken_hand(inv, eq.hand_level, eq.hand_is_remelted)?;
    }

    // equipa o novo (sempre não-remelted via replace)
    eq.hand_level = new_level;
    eq.hand_power_bps = hand_power_bps_by_tier(new_level)?;
    eq.hand_is_remelted = was_remelted;

    Ok(())
}

#[derive(Accounts)]
pub struct EquipmentReplaceHand<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    pub miner_state: Account<'info, MinerState>,

    #[account(
        mut,
        seeds = [SEED_EQUIPMENT, miner_state.key().as_ref()],
        bump = equipment.bump
    )]
    pub equipment: Account<'info, EquipmentState>,

    #[account(
        mut,
        seeds = [SEED_EQUIPMENT_INVENTORY, owner.key().as_ref()],
        bump = inventory.bump
    )]
    pub inventory: Account<'info, EquipmentInventoryState>,
}

// =========================
// REPLACE HEAD
// =========================

pub fn handler_replace_head(ctx: Context<EquipmentReplaceHead>, new_level: u8) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.miner_state.owner,
        ctx.accounts.owner.key(),
        MoeError::Unauthorized
    );
    require_keys_eq!(
        ctx.accounts.equipment.owner,
        ctx.accounts.owner.key(),
        MoeError::Unauthorized
    );
    require_keys_eq!(
        ctx.accounts.equipment.miner,
        ctx.accounts.miner_state.key(),
        MoeError::InvalidMinerRef
    );

    require!(
        !ctx.accounts.miner_state.listed,
        MoeError::AssetListedLocked
    );

    let eq = &mut ctx.accounts.equipment;
    let inv = &mut ctx.accounts.inventory;

    require!(new_level > eq.head_level, MoeError::InvalidUpgrade);

    let was_remelted = inv_consume_1_head_equippable(inv, new_level)?;

    if eq.head_level > 0 {
        inv_add_broken_head(inv, eq.head_level, eq.head_is_remelted)?;
    }

    eq.head_level = new_level;
    eq.head_recharge_discount_bps = head_discount_bps_by_tier(new_level)?;
    eq.head_is_remelted = was_remelted;

    Ok(())
}

#[derive(Accounts)]
pub struct EquipmentReplaceHead<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    pub miner_state: Account<'info, MinerState>,

    #[account(
        mut,
        seeds = [SEED_EQUIPMENT, miner_state.key().as_ref()],
        bump = equipment.bump
    )]
    pub equipment: Account<'info, EquipmentState>,

    #[account(
        mut,
        seeds = [SEED_EQUIPMENT_INVENTORY, owner.key().as_ref()],
        bump = inventory.bump
    )]
    pub inventory: Account<'info, EquipmentInventoryState>,
}

// =========================
// REMELT HAND
// =========================

pub fn handler_remelt_hand(ctx: Context<EquipmentRemeltHand>, base_level: u8) -> Result<()> {
    let ess_cost = remelt_cost_ess(SLOT_HAND, base_level)?;

    let _ = idx(base_level)?;
    require!(
        (base_level as usize) < MAX_ITEM_LEVEL,
        MoeError::InvalidBaseLevel
    );

    require_keys_eq!(
        ctx.accounts.miner_state.owner,
        ctx.accounts.owner.key(),
        MoeError::Unauthorized
    );
    require_keys_eq!(
        ctx.accounts.equipment.owner,
        ctx.accounts.owner.key(),
        MoeError::Unauthorized
    );
    require_keys_eq!(
        ctx.accounts.equipment.miner,
        ctx.accounts.miner_state.key(),
        MoeError::InvalidMinerRef
    );

    require!(
        !ctx.accounts.miner_state.listed,
        MoeError::AssetListedLocked
    );

    let inv = &mut ctx.accounts.inventory;
    let eq = &mut ctx.accounts.equipment;

    // consome 4 itens não-remelted do mesmo slot+base_level
    // (mistura NEW+BROKEN liberada)
    inv_consume_4_hand_non_remelted(inv, base_level)?;

    // nível de saída é SEMPRE base+1
    let new_level = base_level + 1;

    // regra: não deixa "remelt" virar downgrade/mesmo nível do equipado
    require!(new_level > eq.hand_level, MoeError::InvalidUpgrade);

    // cobra ESS (user_ata -> rewards_vault)
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.user_ata.to_account_info(),
                to: ctx.accounts.rewards_vault.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        ),
        ess_cost,
    )?;

    let i_new = idx(new_level)?;
    inv.new_hand_remelted[i_new] = inv.new_hand_remelted[i_new].saturating_add(1);

    Ok(())
}

#[derive(Accounts)]
pub struct EquipmentRemeltHand<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    pub miner_state: Account<'info, MinerState>,

    #[account(
        mut,
        seeds = [SEED_EQUIPMENT, miner_state.key().as_ref()],
        bump = equipment.bump
    )]
    pub equipment: Account<'info, EquipmentState>,

    #[account(
        mut,
        seeds = [SEED_EQUIPMENT_INVENTORY, owner.key().as_ref()],
        bump = inventory.bump
    )]
    pub inventory: Account<'info, EquipmentInventoryState>,

    #[account(mut)]
    pub user_ata: Account<'info, TokenAccount>,

    #[account(mut)]
    pub rewards_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

// =========================
// REMELT HEAD
// =========================

pub fn handler_remelt_head(ctx: Context<EquipmentRemeltHead>, base_level: u8) -> Result<()> {
    let ess_cost = remelt_cost_ess(SLOT_HEAD, base_level)?;

    let _ = idx(base_level)?;
    require!(
        (base_level as usize) < MAX_ITEM_LEVEL,
        MoeError::InvalidBaseLevel
    );

    require_keys_eq!(
        ctx.accounts.miner_state.owner,
        ctx.accounts.owner.key(),
        MoeError::Unauthorized
    );
    require_keys_eq!(
        ctx.accounts.equipment.owner,
        ctx.accounts.owner.key(),
        MoeError::Unauthorized
    );
    require_keys_eq!(
        ctx.accounts.equipment.miner,
        ctx.accounts.miner_state.key(),
        MoeError::InvalidMinerRef
    );

    require!(
        !ctx.accounts.miner_state.listed,
        MoeError::AssetListedLocked
    );

    let inv = &mut ctx.accounts.inventory;
    let eq = &mut ctx.accounts.equipment;

    inv_consume_4_head_non_remelted(inv, base_level)?;

    let new_level = base_level + 1;
    require!(new_level > eq.head_level, MoeError::InvalidUpgrade);

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.user_ata.to_account_info(),
                to: ctx.accounts.rewards_vault.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        ),
        ess_cost,
    )?;

    let i_new = idx(new_level)?;
    inv.new_head_remelted[i_new] = inv.new_head_remelted[i_new].saturating_add(1);

    Ok(())
}

#[derive(Accounts)]
pub struct EquipmentRemeltHead<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    pub miner_state: Account<'info, MinerState>,

    #[account(
        mut,
        seeds = [SEED_EQUIPMENT, miner_state.key().as_ref()],
        bump = equipment.bump
    )]
    pub equipment: Account<'info, EquipmentState>,

    #[account(
        mut,
        seeds = [SEED_EQUIPMENT_INVENTORY, owner.key().as_ref()],
        bump = inventory.bump
    )]
    pub inventory: Account<'info, EquipmentInventoryState>,

    #[account(mut)]
    pub user_ata: Account<'info, TokenAccount>,

    #[account(mut)]
    pub rewards_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}
