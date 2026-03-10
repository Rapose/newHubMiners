use crate::state::ProgressionConfig;
use crate::utils::seeds::SEED_PROGRESSION;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct ProgressionInit<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        seeds = [SEED_PROGRESSION],
        bump,
        space = ProgressionConfig::LEN
    )]
    pub progression: Account<'info, ProgressionConfig>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<ProgressionInit>) -> Result<()> {
    let p = &mut ctx.accounts.progression;

    p.admin = ctx.accounts.admin.key();
    p.mining_window_secs = 21600;
    p.max_accrual_windows = 28;
    p.linear_hash_k_bps = 200;

    p.max_level_by_rarity = [20, 30, 40, 50, 60];

    p.exp_base_by_rarity = [10, 15, 25, 40, 60];
    p.exp_growth_bps = 12000; // 1.2x

    p.ess_base_cost_by_rarity = [5, 8, 12, 20, 35];
    p.ess_growth_bps = 12500; // 1.25x

    p.bump = ctx.bumps.progression;
    Ok(())
}
