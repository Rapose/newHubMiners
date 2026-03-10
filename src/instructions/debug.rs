use crate::{
    constants::*,
    state::{Config, LandState, MinerState},
    utils::{require_element, require_not_paused, require_rarity, require_slots, u64_le_bytes},
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CreateMinerDebug<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_CONFIG],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,

    #[account(
        init,
        payer = owner,
        space = MinerState::LEN,
        seeds = [SEED_MINER, owner.key().as_ref(), &u64_le_bytes(config.next_miner_id)],
        bump
    )]
    pub miner_state: Account<'info, MinerState>,

    pub system_program: Program<'info, System>,
}

pub fn handler_create_miner_debug(
    ctx: Context<CreateMinerDebug>,
    rarity: u8,
    element: u8,
    hash_base: u64,
) -> Result<()> {
    let cfg = &mut ctx.accounts.config;
    require_not_paused(cfg.paused)?;
    require_rarity(rarity)?;
    require_element(element)?;

    let miner = &mut ctx.accounts.miner_state;
    miner.id = cfg.next_miner_id;
    miner.owner = ctx.accounts.owner.key();
    miner.rarity = rarity;
    miner.element = element;
    miner.hash_base = hash_base;
    miner.face = 0;
    miner.helmet = 0;
    miner.backpack = 0;
    miner.jacket = 0;
    miner.item = 0;
    miner.background = 0;
    miner.allocated_land = Pubkey::default();
    miner.listed = false;
    miner.created_at = Clock::get()?.unix_timestamp;
    miner.bump = ctx.bumps.miner_state;

    cfg.next_miner_id = cfg.next_miner_id.saturating_add(1);
    Ok(())
}

#[derive(Accounts)]
pub struct CreateLandDebug<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_CONFIG],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,

    #[account(
        init,
        payer = owner,
        space = LandState::LEN,
        seeds = [SEED_LAND, owner.key().as_ref(), &u64_le_bytes(config.next_land_id)],
        bump
    )]
    pub land_state: Account<'info, LandState>,

    pub system_program: Program<'info, System>,
}

pub fn handler_create_land_debug(
    ctx: Context<CreateLandDebug>,
    rarity: u8,
    element: u8,
    slots: u8,
) -> Result<()> {
    let cfg = &mut ctx.accounts.config;
    require_not_paused(cfg.paused)?;
    require_rarity(rarity)?;
    require_element(element)?;
    require_slots(slots)?;

    let land = &mut ctx.accounts.land_state;
    land.id = cfg.next_land_id;
    land.owner = ctx.accounts.owner.key();
    land.rarity = rarity;
    land.element = element;
    land.slots = slots;
    land.listed = false;
    land.allocated_miners_count = 0;
    land.created_at = Clock::get()?.unix_timestamp;
    land.bump = ctx.bumps.land_state;

    cfg.next_land_id = cfg.next_land_id.saturating_add(1);
    Ok(())
}
