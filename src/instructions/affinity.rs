use anchor_lang::prelude::*;
use crate::{
    constants::*,
    state::AffinityResult,
    utils::affinity::compute_affinity_bps,
};

#[derive(Accounts)]
#[instruction(land_element: u8, miner_element: u8)]
pub struct ComputeAffinity<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        payer = user,
        space = 8 + AffinityResult::LEN,
        seeds = [SEED_AFF, user.key().as_ref(), &[land_element], &[miner_element]],
        bump
    )]
    pub result: Account<'info, AffinityResult>,

    pub system_program: Program<'info, System>,
}

pub fn handler_compute_affinity(
    ctx: Context<ComputeAffinity>,
    land_element: u8,
    miner_element: u8,
) -> Result<()> {
    let r = &mut ctx.accounts.result;
    r.user = ctx.accounts.user.key();
    r.land_element = land_element;
    r.miner_element = miner_element;
    r.bps = compute_affinity_bps(miner_element, land_element)?;
    r.bump = 0;

    Ok(())
}