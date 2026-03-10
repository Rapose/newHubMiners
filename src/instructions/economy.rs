use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::{
    constants::{BPS_DENOM, SEED_ECONOMY, SEED_REWARDS_AUTH},
    errors::MoeError,
    state::{Allocation3, Allocation7, EconomyConfig},
    utils::{token_burn, token_transfer},
};

#[event]
pub struct EconomySpent {
    pub mechanism: u8,
    pub user: Pubkey,
    pub mint: Pubkey,
    pub total_amount: u64,
    pub burn_amount: u64,
    pub recipient_amount: u64,
    pub recipient_wallet: Pubkey,
}

fn mul_bps(amount: u64, bps: u16) -> u64 {
    (amount as u128)
        .saturating_mul(bps as u128)
        .checked_div(BPS_DENOM as u128)
        .unwrap_or(0) as u64
}

fn validate_alloc7(a: &Allocation7) -> Result<()> {
    require!(a.sum() == 10_000, MoeError::InvalidBpsSum);
    Ok(())
}

fn validate_alloc3(a: &Allocation3) -> Result<()> {
    require!(a.sum() == 10_000, MoeError::InvalidBpsSum);
    Ok(())
}

#[derive(Accounts)]
pub struct EconomySetRewardsVault<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_ECONOMY],
        bump = economy.bump,
        constraint = economy.admin == admin.key() @ MoeError::Unauthorized
    )]
    pub economy: Account<'info, EconomyConfig>,

    /// CHECK: PDA derivada do programa
    #[account(
        seeds = [SEED_REWARDS_AUTH],
        bump
    )]
    pub rewards_authority: UncheckedAccount<'info>,

    #[account(
        constraint = rewards_vault.owner == rewards_authority.key() @ MoeError::Unauthorized,
        constraint = rewards_vault.mint == economy.ess_mint @ MoeError::MintMismatch
    )]
    pub rewards_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler_set_rewards_vault(ctx: Context<EconomySetRewardsVault>) -> Result<()> {
    let economy = &mut ctx.accounts.economy;
    economy.rewards_authority = ctx.accounts.rewards_authority.key();
    economy.rewards_vault = ctx.accounts.rewards_vault.key();
    Ok(())
}

#[derive(Accounts)]
pub struct EconomyInit<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    pub ess_mint: Account<'info, Mint>,

    /// CHECK: apenas armazenado
    pub recipient_wallet: UncheckedAccount<'info>,

    #[account(
        init,
        payer = admin,
        space = EconomyConfig::LEN,
        seeds = [SEED_ECONOMY],
        bump
    )]
    pub economy: Account<'info, EconomyConfig>,

    /// CHECK: PDA validada por seed
    #[account(
        seeds = [SEED_REWARDS_AUTH],
        bump
    )]
    pub rewards_authority: UncheckedAccount<'info>,

    #[account(
        constraint = rewards_vault.mint == ess_mint.key() @ MoeError::MintMismatch,
        constraint = rewards_vault.owner == rewards_authority.key() @ MoeError::Unauthorized
    )]
    pub rewards_vault: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
}
pub fn handler_economy_init(ctx: Context<EconomyInit>) -> Result<()> {
    let economy = &mut ctx.accounts.economy;

    economy.admin = ctx.accounts.admin.key();
    economy.ess_mint = ctx.accounts.ess_mint.key();
    economy.recipient_wallet = ctx.accounts.recipient_wallet.key();
    economy.rewards_authority = ctx.accounts.rewards_authority.key();
    economy.rewards_vault = ctx.accounts.rewards_vault.key();
    economy.bump = ctx.bumps.economy;

    economy.buy = Allocation7 {
        burn_bps: 3500,
        liquidity_bps: 2500,
        mining_pool_bps: 1000,
        marketing_bps: 1000,
        dev_infra_bps: 1000,
        forges_bps: 500,
        treasury_bps: 500,
    };
    economy.trade_fee = economy.buy;

    economy.send = Allocation7 {
        burn_bps: 6000,
        liquidity_bps: 500,
        mining_pool_bps: 500,
        marketing_bps: 500,
        dev_infra_bps: 1000,
        forges_bps: 500,
        treasury_bps: 1000,
    };

    economy.recharge = Allocation3 {
        burn_bps: 4500,
        forges_bps: 4500,
        treasury_bps: 1000,
    };

    validate_alloc7(&economy.buy)?;
    validate_alloc7(&economy.trade_fee)?;
    validate_alloc7(&economy.send)?;
    validate_alloc3(&economy.recharge)?;

    Ok(())
}

#[derive(Accounts)]
pub struct EconomySetRecipient<'info> {
    pub admin: Signer<'info>,

    /// CHECK: apenas armazenado como pubkey
    pub recipient_wallet: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [SEED_ECONOMY],
        bump = economy.bump,
        constraint = economy.admin == admin.key() @ MoeError::Unauthorized
    )]
    pub economy: Account<'info, EconomyConfig>,
}

pub fn handler_set_recipient(ctx: Context<EconomySetRecipient>) -> Result<()> {
    ctx.accounts.economy.recipient_wallet = ctx.accounts.recipient_wallet.key();
    Ok(())
}

#[derive(Accounts)]
pub struct EconomySetMint<'info> {
    pub admin: Signer<'info>,
    pub ess_mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [SEED_ECONOMY],
        bump = economy.bump,
        constraint = economy.admin == admin.key() @ MoeError::Unauthorized
    )]
    pub economy: Account<'info, EconomyConfig>,
}

pub fn handler_set_mint(ctx: Context<EconomySetMint>) -> Result<()> {
    ctx.accounts.economy.ess_mint = ctx.accounts.ess_mint.key();
    Ok(())
}

#[derive(Accounts)]
pub struct RewardsDeposit<'info> {
    #[account(mut)]
    pub depositor: Signer<'info>,

    pub ess_mint: Account<'info, Mint>,

    #[account(
        mut,
        constraint = depositor_ata.mint == ess_mint.key() @ MoeError::MintMismatch,
        constraint = depositor_ata.owner == depositor.key() @ MoeError::Unauthorized
    )]
    pub depositor_ata: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [SEED_ECONOMY],
        bump = economy.bump
    )]
    pub economy: Account<'info, EconomyConfig>,

    /// CHECK: PDA derivada do programa
    #[account(
        seeds = [SEED_REWARDS_AUTH],
        bump
    )]
    pub rewards_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = rewards_vault.key() == economy.rewards_vault @ MoeError::Unauthorized,
        constraint = rewards_vault.owner == rewards_authority.key() @ MoeError::Unauthorized,
        constraint = rewards_vault.mint == ess_mint.key() @ MoeError::MintMismatch
    )]
    pub rewards_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler_rewards_deposit(ctx: Context<RewardsDeposit>, amount: u64) -> Result<()> {
    require!(amount > 0, MoeError::InvalidSlots);

    require!(
        ctx.accounts.economy.ess_mint == ctx.accounts.ess_mint.key(),
        MoeError::MintMismatch
    );

    require!(
        ctx.accounts.rewards_vault.key() == ctx.accounts.economy.rewards_vault,
        MoeError::Unauthorized
    );

    token_transfer(
        &ctx.accounts.token_program,
        &ctx.accounts.depositor_ata,
        &ctx.accounts.rewards_vault,
        &ctx.accounts.depositor,
        amount,
    )
}

#[derive(Accounts)]
pub struct SpendEss<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut)]
    pub ess_mint: Account<'info, Mint>,

    #[account(
        mut,
        constraint = user_ata.mint == ess_mint.key() @ MoeError::MintMismatch,
        constraint = user_ata.owner == user.key() @ MoeError::Unauthorized
    )]
    pub user_ata: Account<'info, TokenAccount>,

    /// CHECK: validado contra economy.recipient_wallet
    pub recipient_wallet: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = recipient_ata.mint == ess_mint.key() @ MoeError::MintMismatch,
        constraint = recipient_ata.owner == recipient_wallet.key() @ MoeError::RecipientMismatch
    )]
    pub recipient_ata: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [SEED_ECONOMY],
        bump = economy.bump
    )]
    pub economy: Account<'info, EconomyConfig>,

    pub token_program: Program<'info, Token>,
}

fn assert_economy(ctx: &SpendEss) -> Result<()> {
    require!(
        ctx.economy.ess_mint == ctx.ess_mint.key(),
        MoeError::MintMismatch
    );
    require!(
        ctx.economy.recipient_wallet == ctx.recipient_wallet.key(),
        MoeError::RecipientMismatch
    );
    Ok(())
}

pub fn handler_spend_buy(ctx: Context<SpendEss>, amount: u64) -> Result<()> {
    assert_economy(&ctx.accounts)?;
    spend_7(ctx, amount, 1)
}

pub fn handler_spend_send(ctx: Context<SpendEss>, amount: u64) -> Result<()> {
    assert_economy(&ctx.accounts)?;
    spend_7(ctx, amount, 2)
}

pub fn handler_spend_trade_fee(ctx: Context<SpendEss>, fee_amount: u64) -> Result<()> {
    assert_economy(&ctx.accounts)?;
    spend_7(ctx, fee_amount, 4)
}

pub fn handler_spend_recharge(ctx: Context<SpendEss>, amount: u64) -> Result<()> {
    assert_economy(&ctx.accounts)?;
    spend_3(ctx, amount, 3)
}

fn spend_7(ctx: Context<SpendEss>, amount: u64, mechanism: u8) -> Result<()> {
    require!(amount > 0, MoeError::InvalidSlots);

    let economy = &mut ctx.accounts.economy;

    let alloc = match mechanism {
        1 => economy.buy,
        2 => economy.send,
        4 => economy.trade_fee,
        _ => economy.buy,
    };
    validate_alloc7(&alloc)?;

    let burn_amt = mul_bps(amount, alloc.burn_bps);
    let liq_amt = mul_bps(amount, alloc.liquidity_bps);
    let mining_amt = mul_bps(amount, alloc.mining_pool_bps);
    let mkt_amt = mul_bps(amount, alloc.marketing_bps);
    let dev_amt = mul_bps(amount, alloc.dev_infra_bps);
    let forge_amt = mul_bps(amount, alloc.forges_bps);
    let tres_amt = mul_bps(amount, alloc.treasury_bps);

    let non_burn = liq_amt
        .saturating_add(mining_amt)
        .saturating_add(mkt_amt)
        .saturating_add(dev_amt)
        .saturating_add(forge_amt)
        .saturating_add(tres_amt);

    token_burn(
        &ctx.accounts.token_program,
        &ctx.accounts.ess_mint,
        &ctx.accounts.user_ata,
        &ctx.accounts.user,
        burn_amt,
    )?;

    token_transfer(
        &ctx.accounts.token_program,
        &ctx.accounts.user_ata,
        &ctx.accounts.recipient_ata,
        &ctx.accounts.user,
        non_burn,
    )?;

    match mechanism {
        1 => {
            economy.totals_buy.burn = economy.totals_buy.burn.saturating_add(burn_amt);
            economy.totals_buy.liquidity = economy.totals_buy.liquidity.saturating_add(liq_amt);
            economy.totals_buy.mining_pool =
                economy.totals_buy.mining_pool.saturating_add(mining_amt);
            economy.totals_buy.marketing = economy.totals_buy.marketing.saturating_add(mkt_amt);
            economy.totals_buy.dev_infra = economy.totals_buy.dev_infra.saturating_add(dev_amt);
            economy.totals_buy.forges = economy.totals_buy.forges.saturating_add(forge_amt);
            economy.totals_buy.treasury = economy.totals_buy.treasury.saturating_add(tres_amt);
        }
        2 => {
            economy.totals_send.burn = economy.totals_send.burn.saturating_add(burn_amt);
            economy.totals_send.liquidity = economy.totals_send.liquidity.saturating_add(liq_amt);
            economy.totals_send.mining_pool =
                economy.totals_send.mining_pool.saturating_add(mining_amt);
            economy.totals_send.marketing = economy.totals_send.marketing.saturating_add(mkt_amt);
            economy.totals_send.dev_infra = economy.totals_send.dev_infra.saturating_add(dev_amt);
            economy.totals_send.forges = economy.totals_send.forges.saturating_add(forge_amt);
            economy.totals_send.treasury = economy.totals_send.treasury.saturating_add(tres_amt);
        }
        4 => {
            economy.totals_trade_fee.burn = economy.totals_trade_fee.burn.saturating_add(burn_amt);
            economy.totals_trade_fee.liquidity =
                economy.totals_trade_fee.liquidity.saturating_add(liq_amt);
            economy.totals_trade_fee.mining_pool = economy
                .totals_trade_fee
                .mining_pool
                .saturating_add(mining_amt);
            economy.totals_trade_fee.marketing =
                economy.totals_trade_fee.marketing.saturating_add(mkt_amt);
            economy.totals_trade_fee.dev_infra =
                economy.totals_trade_fee.dev_infra.saturating_add(dev_amt);
            economy.totals_trade_fee.forges =
                economy.totals_trade_fee.forges.saturating_add(forge_amt);
            economy.totals_trade_fee.treasury =
                economy.totals_trade_fee.treasury.saturating_add(tres_amt);
        }
        _ => {}
    }

    emit!(EconomySpent {
        mechanism,
        user: ctx.accounts.user.key(),
        mint: ctx.accounts.ess_mint.key(),
        total_amount: amount,
        burn_amount: burn_amt,
        recipient_amount: non_burn,
        recipient_wallet: ctx.accounts.recipient_wallet.key(),
    });

    Ok(())
}

fn spend_3(ctx: Context<SpendEss>, amount: u64, mechanism: u8) -> Result<()> {
    require!(amount > 0, MoeError::InvalidSlots);

    let economy = &mut ctx.accounts.economy;
    let alloc = economy.recharge;
    validate_alloc3(&alloc)?;

    let burn_amt = mul_bps(amount, alloc.burn_bps);
    let forges_amt = mul_bps(amount, alloc.forges_bps);
    let tres_amt = mul_bps(amount, alloc.treasury_bps);

    let non_burn = forges_amt.saturating_add(tres_amt);

    token_burn(
        &ctx.accounts.token_program,
        &ctx.accounts.ess_mint,
        &ctx.accounts.user_ata,
        &ctx.accounts.user,
        burn_amt,
    )?;

    token_transfer(
        &ctx.accounts.token_program,
        &ctx.accounts.user_ata,
        &ctx.accounts.recipient_ata,
        &ctx.accounts.user,
        non_burn,
    )?;

    economy.totals_recharge.burn = economy.totals_recharge.burn.saturating_add(burn_amt);
    economy.totals_recharge.forges = economy.totals_recharge.forges.saturating_add(forges_amt);
    economy.totals_recharge.treasury = economy.totals_recharge.treasury.saturating_add(tres_amt);

    emit!(EconomySpent {
        mechanism,
        user: ctx.accounts.user.key(),
        mint: ctx.accounts.ess_mint.key(),
        total_amount: amount,
        burn_amount: burn_amt,
        recipient_amount: non_burn,
        recipient_wallet: ctx.accounts.recipient_wallet.key(),
    });

    Ok(())
}
