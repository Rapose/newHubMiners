use anchor_lang::prelude::*;

use crate::{
    errors::MoeError,
    state::{Config, MinerProgress, MinerState, ProgressionConfig},
    utils::{
        progression_math::exp_required,
        seeds::{SEED_MINER_PROGRESS, SEED_PROGRESSION},
    },
};

#[derive(Accounts)]
pub struct AdminGrantExp<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [b"config"],
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,

    #[account(
        seeds = [SEED_PROGRESSION],
        bump = progression.bump,
    )]
    pub progression: Account<'info, ProgressionConfig>,

    pub miner_state: Account<'info, MinerState>,

    #[account(
        mut,
        seeds = [SEED_MINER_PROGRESS, miner_state.key().as_ref()],
        bump = miner_progress.bump,
    )]
    pub miner_progress: Account<'info, MinerProgress>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<AdminGrantExp>, amount: u64) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.config.admin,
        ctx.accounts.admin.key(),
        MoeError::Unauthorized
    );

    let mp = &mut ctx.accounts.miner_progress;
    let miner = &ctx.accounts.miner_state;

    require_keys_eq!(mp.owner, miner.owner, MoeError::Unauthorized);
    require_keys_eq!(mp.miner, miner.key(), MoeError::InvalidMinerProgress);

    let r = miner.rarity as usize;
    require!(r < 5, MoeError::InvalidRarity);

    let need_exp = exp_required(&ctx.accounts.progression, r, mp.level);

    if mp.exp >= need_exp {
        return Ok(());
    }

    let added = mp
        .exp
        .checked_add(amount)
        .ok_or(MoeError::MathOverflow)?;

    mp.exp = added.min(need_exp);

    Ok(())
}