use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};

use crate::errors::MoeError;
use crate::state::{EconomyConfig, MinerProgress, MinerState, ProgressionConfig};
use crate::utils::progression_math::{ess_cost, exp_required};
use crate::utils::seeds::{SEED_ECONOMY, SEED_MINER_PROGRESS, SEED_PROGRESSION};

#[derive(Accounts)]
pub struct MinerLevelUp<'info> {
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

    #[account(
        mut,
        seeds = [SEED_ECONOMY],
        bump = economy.bump
    )]
    pub economy: Account<'info, EconomyConfig>,

    #[account(mut)]
    pub ess_mint: Account<'info, Mint>,

    #[account(
        mut,
        constraint = user_ata.mint == ess_mint.key() @ MoeError::MintMismatch,
        constraint = user_ata.owner == owner.key() @ MoeError::Unauthorized
    )]
    pub user_ata: Account<'info, TokenAccount>,

    /// CHECK: recipient_wallet is only used as a destination pubkey reference and is validated against economy.recipient_wallet before transfer.
    pub recipient_wallet: UncheckedAccount<'info>,

#[account(
    mut,
    constraint = recipient_ata.mint == ess_mint.key() @ MoeError::MintMismatch,
    constraint = recipient_ata.owner == recipient_wallet.key() @ MoeError::Unauthorized
)]
pub recipient_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<MinerLevelUp>) -> Result<()> {
    let owner = ctx.accounts.owner.key();
    let miner = &ctx.accounts.miner_state;
    let cfg = &ctx.accounts.progression;
    let prog = &mut ctx.accounts.miner_progress;

    require!(miner.owner == owner, MoeError::Unauthorized);
    require!(!miner.listed, MoeError::AssetListedLocked);

    require!(prog.owner == owner, MoeError::Unauthorized);
    require!(prog.miner == miner.key(), MoeError::InvalidMinerProgress);

    let r = miner.rarity as usize;
    require!(r < 5, MoeError::InvalidRarity);

    let max_level = cfg.max_level_by_rarity[r];
    require!(prog.level < max_level, MoeError::MaxLevelReached);

    let need_exp = exp_required(cfg, r, prog.level);
    require!(prog.exp >= need_exp, MoeError::NotEnoughExp);

    let cost = ess_cost(cfg, r, prog.level);

    require!(
        ctx.accounts.economy.ess_mint == ctx.accounts.ess_mint.key(),
        MoeError::MintMismatch
    );

    require!(
        ctx.accounts.economy.recipient_wallet == ctx.accounts.recipient_wallet.key(),
        MoeError::RecipientMismatch
    );

    require!(
        ctx.accounts.recipient_ata.owner == ctx.accounts.recipient_wallet.key(),
        MoeError::Unauthorized
    );

    let burn_amt = cost.saturating_mul(35).saturating_div(100);
    let transfer_amt = cost.saturating_sub(burn_amt);

    if burn_amt > 0 {
        token::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Burn {
                    mint: ctx.accounts.ess_mint.to_account_info(),
                    from: ctx.accounts.user_ata.to_account_info(),
                    authority: ctx.accounts.owner.to_account_info(),
                },
            ),
            burn_amt,
        )?;
    }

    if transfer_amt > 0 {
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_ata.to_account_info(),
                    to: ctx.accounts.recipient_ata.to_account_info(),
                    authority: ctx.accounts.owner.to_account_info(),
                },
            ),
            transfer_amt,
        )?;
    }

    prog.level = prog.level.saturating_add(1);
    prog.exp = 0;

    Ok(())
}
