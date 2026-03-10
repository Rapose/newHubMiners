use anchor_lang::prelude::*;

use crate::errors::MoeError;
use crate::state::{MinerProgress, MinerState, ProgressionConfig};
use crate::utils::progression_math::exp_required;
use crate::utils::seeds::{SEED_MINER_PROGRESS, SEED_PROGRESSION};

#[derive(Accounts)]
pub struct ClaimMiningExp<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    pub miner_state: Account<'info, MinerState>,

    #[account(
        seeds = [SEED_PROGRESSION],
        bump = progression.bump
    )]
    pub progression: Account<'info, ProgressionConfig>,

    #[account(
        mut,
        seeds = [SEED_MINER_PROGRESS, miner_state.key().as_ref()],
        bump = miner_progress.bump
    )]
    pub miner_progress: Account<'info, MinerProgress>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<ClaimMiningExp>) -> Result<()> {
    let owner = ctx.accounts.owner.key();
    let miner = &ctx.accounts.miner_state;
    let cfg = &ctx.accounts.progression;
    let prog = &mut ctx.accounts.miner_progress;

    require!(miner.owner == owner, MoeError::Unauthorized);
    require!(!miner.listed, MoeError::AssetListedLocked);

    require!(prog.owner == owner, MoeError::Unauthorized);
    require!(prog.miner == miner.key(), MoeError::InvalidMinerProgress);

    let now = Clock::get()?.unix_timestamp;

    if now <= prog.last_exp_claim_ts {
        return Ok(());
    }

    let window = cfg.mining_window_secs.max(1);
    let mut windows = ((now - prog.last_exp_claim_ts) / window) as u64;

    if windows == 0 {
        return Ok(());
    }

    let cap = cfg.max_accrual_windows as u64;
    if cap > 0 && windows > cap {
        windows = cap;
    }

    let r = miner.rarity as usize;
    require!(r < 5, MoeError::InvalidRarity);

    let need_exp = exp_required(cfg, r, prog.level);

    if prog.exp >= need_exp {
        prog.last_exp_claim_ts = now;
        return Ok(());
    }

    let new_exp = prog.exp.saturating_add(windows);
    prog.exp = new_exp.min(need_exp);

    prog.last_exp_claim_ts = prog
        .last_exp_claim_ts
        .saturating_add((windows as i64).saturating_mul(window));

    Ok(())
}
