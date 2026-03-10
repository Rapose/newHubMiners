use crate::{constants::*, errors::MoeError, state::Config};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = Config::LEN,
        seeds = [SEED_CONFIG],
        bump
    )]
    pub config: Account<'info, Config>,

    pub system_program: Program<'info, System>,
}

pub fn handler_initialize_config(ctx: Context<InitializeConfig>) -> Result<()> {
    let cfg = &mut ctx.accounts.config;
    cfg.admin = ctx.accounts.admin.key();
    cfg.paused = false;
    cfg.next_miner_id = 0;
    cfg.next_land_id = 0;
    cfg.next_listing_id = 0;
    cfg.bump = ctx.bumps.config;
    Ok(())
}

#[derive(Accounts)]
pub struct SetPaused<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_CONFIG],
        bump = config.bump,
        constraint = config.admin == admin.key() @ MoeError::Unauthorized
    )]
    pub config: Account<'info, Config>,
}

pub fn handler_set_paused(ctx: Context<SetPaused>, paused: bool) -> Result<()> {
    ctx.accounts.config.paused = paused;
    Ok(())
}
