use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::{
    constants::{
        BPS_DENOM, MARKETPLACE_FEE_BPS, MAX_ITEM_LEVEL, SEED_CONFIG, SEED_ECONOMY, SEED_EQUIPMENT,
        SEED_EQUIPMENT_INVENTORY, SEED_LISTING, SEED_MINER_MINING,
    },
    errors::MoeError,
    state::{
        Config, EconomyConfig, EquipmentInventoryBucket, EquipmentInventoryState, EquipmentState,
        LandState, ListingAssetKind, ListingState, MinerMiningState, MinerProgress, MinerState,
    },
    utils::{seeds::SEED_MINER_PROGRESS, u64_le_bytes},
};

fn now_ts() -> Result<i64> {
    Ok(Clock::get()?.unix_timestamp)
}

fn marketplace_fee(price: u64) -> Result<u64> {
    ((price as u128)
        .saturating_mul(MARKETPLACE_FEE_BPS as u128)
        .checked_div(BPS_DENOM as u128)
        .ok_or(error!(MoeError::MathOverflow))?)
    .try_into()
    .map_err(|_| error!(MoeError::MathOverflow))
}

fn mul_bps(amount: u64, bps: u16) -> u64 {
    (amount as u128)
        .saturating_mul(bps as u128)
        .checked_div(BPS_DENOM as u128)
        .unwrap_or(0) as u64
}

fn accumulate_trade_fee_totals(economy: &mut EconomyConfig, fee_amount: u64) {
    let alloc = economy.trade_fee;
    economy.totals_trade_fee.burn = economy
        .totals_trade_fee
        .burn
        .saturating_add(mul_bps(fee_amount, alloc.burn_bps));
    economy.totals_trade_fee.liquidity = economy
        .totals_trade_fee
        .liquidity
        .saturating_add(mul_bps(fee_amount, alloc.liquidity_bps));
    economy.totals_trade_fee.mining_pool = economy
        .totals_trade_fee
        .mining_pool
        .saturating_add(mul_bps(fee_amount, alloc.mining_pool_bps));
    economy.totals_trade_fee.marketing = economy
        .totals_trade_fee
        .marketing
        .saturating_add(mul_bps(fee_amount, alloc.marketing_bps));
    economy.totals_trade_fee.dev_infra = economy
        .totals_trade_fee
        .dev_infra
        .saturating_add(mul_bps(fee_amount, alloc.dev_infra_bps));
    economy.totals_trade_fee.forges = economy
        .totals_trade_fee
        .forges
        .saturating_add(mul_bps(fee_amount, alloc.forges_bps));
    economy.totals_trade_fee.treasury = economy
        .totals_trade_fee
        .treasury
        .saturating_add(mul_bps(fee_amount, alloc.treasury_bps));
}

fn bucket_mut(inv: &mut EquipmentInventoryState, bucket: u8) -> Result<&mut [u16]> {
    match bucket {
        x if x == EquipmentInventoryBucket::NewHand as u8 => Ok(&mut inv.new_hand),
        x if x == EquipmentInventoryBucket::NewHead as u8 => Ok(&mut inv.new_head),
        x if x == EquipmentInventoryBucket::BrokenHand as u8 => Ok(&mut inv.broken_hand),
        x if x == EquipmentInventoryBucket::BrokenHead as u8 => Ok(&mut inv.broken_head),
        x if x == EquipmentInventoryBucket::NewHandRemelted as u8 => Ok(&mut inv.new_hand_remelted),
        x if x == EquipmentInventoryBucket::NewHeadRemelted as u8 => Ok(&mut inv.new_head_remelted),
        x if x == EquipmentInventoryBucket::BrokenHandRemelted as u8 => {
            Ok(&mut inv.broken_hand_remelted)
        }
        x if x == EquipmentInventoryBucket::BrokenHeadRemelted as u8 => {
            Ok(&mut inv.broken_head_remelted)
        }
        _ => err!(MoeError::EquipmentBucketMismatch),
    }
}

fn reserve_inventory(
    inv: &mut EquipmentInventoryState,
    bucket: u8,
    level: u8,
    amount: u16,
) -> Result<()> {
    require!(amount > 0, MoeError::InvalidEquipmentAmount);
    let idx = level as usize;
    require!(
        idx >= 1 && idx <= MAX_ITEM_LEVEL,
        MoeError::InvalidBaseLevel
    );

    let arr = bucket_mut(inv, bucket)?;
    let current = *arr
        .get(idx)
        .ok_or(error!(MoeError::EquipmentBucketMismatch))?;
    require!(current >= amount, MoeError::InsufficientEquipmentInventory);
    arr[idx] = current.saturating_sub(amount);
    Ok(())
}

fn release_inventory(
    inv: &mut EquipmentInventoryState,
    bucket: u8,
    level: u8,
    amount: u16,
) -> Result<()> {
    require!(amount > 0, MoeError::InvalidEquipmentAmount);
    let idx = level as usize;
    require!(
        idx >= 1 && idx <= MAX_ITEM_LEVEL,
        MoeError::InvalidBaseLevel
    );

    let arr = bucket_mut(inv, bucket)?;
    let current = *arr
        .get(idx)
        .ok_or(error!(MoeError::EquipmentBucketMismatch))?;
    arr[idx] = current.saturating_add(amount);
    Ok(())
}

fn fill_listing_common(
    listing: &mut Account<ListingState>,
    id: u64,
    seller: Pubkey,
    asset_kind: ListingAssetKind,
    price_ess: u64,
    bump: u8,
) -> Result<()> {
    require!(price_ess > 0, MoeError::ListingPriceInvalid);

    listing.id = id;
    listing.seller = seller;
    listing.active = true;
    listing.asset_kind = asset_kind as u8;
    listing.price_ess = price_ess;
    listing.created_at = now_ts()?;
    listing.miner = Pubkey::default();
    listing.land = Pubkey::default();
    listing.inventory_owner = Pubkey::default();
    listing.equipment_bucket = 0;
    listing.equipment_level = 0;
    listing.equipment_amount = 0;
    listing.bump = bump;
    Ok(())
}

fn validate_buy_common<'info>(
    buyer: Pubkey,
    seller: &UncheckedAccount<'info>,
    listing: &Account<'info, ListingState>,
    economy: &Account<'info, EconomyConfig>,
    ess_mint: &Account<'info, Mint>,
    buyer_ata: &Account<'info, TokenAccount>,
    seller_ata: &Account<'info, TokenAccount>,
    recipient_ata: &Account<'info, TokenAccount>,
) -> Result<()> {
    require!(listing.active, MoeError::ListingInactive);
    require!(listing.seller != buyer, MoeError::SelfPurchaseNotAllowed);
    require!(seller.key() == listing.seller, MoeError::InvalidListing);

    require!(economy.ess_mint == ess_mint.key(), MoeError::MintMismatch);
    require!(buyer_ata.owner == buyer, MoeError::Unauthorized);
    require!(buyer_ata.mint == ess_mint.key(), MoeError::MintMismatch);
    require!(seller_ata.owner == seller.key(), MoeError::Unauthorized);
    require!(seller_ata.mint == ess_mint.key(), MoeError::MintMismatch);
    require!(
        recipient_ata.owner == economy.recipient_wallet,
        MoeError::RecipientMismatch
    );
    require!(recipient_ata.mint == ess_mint.key(), MoeError::MintMismatch);

    Ok(())
}

fn settle_buy_transfers<'info>(
    token_program: &Program<'info, Token>,
    buyer: &Signer<'info>,
    buyer_ata: &Account<'info, TokenAccount>,
    seller_ata: &Account<'info, TokenAccount>,
    recipient_ata: &Account<'info, TokenAccount>,
    economy: &mut Account<'info, EconomyConfig>,
    listing: &Account<'info, ListingState>,
) -> Result<()> {
    let fee = marketplace_fee(listing.price_ess)?;
    let seller_amount = listing.price_ess.saturating_sub(fee);

    if fee > 0 {
        token::transfer(
            CpiContext::new(
                token_program.to_account_info(),
                Transfer {
                    from: buyer_ata.to_account_info(),
                    to: recipient_ata.to_account_info(),
                    authority: buyer.to_account_info(),
                },
            ),
            fee,
        )?;
        accumulate_trade_fee_totals(economy, fee);
    }

    if seller_amount > 0 {
        token::transfer(
            CpiContext::new(
                token_program.to_account_info(),
                Transfer {
                    from: buyer_ata.to_account_info(),
                    to: seller_ata.to_account_info(),
                    authority: buyer.to_account_info(),
                },
            ),
            seller_amount,
        )?;
    }

    Ok(())
}

#[derive(Accounts)]
pub struct MarketplaceCreateMinerListing<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,

    #[account(mut, seeds = [SEED_CONFIG], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(mut, constraint = miner.owner == seller.key() @ MoeError::Unauthorized)]
    pub miner: Account<'info, MinerState>,

    #[account(
        init,
        payer = seller,
        space = ListingState::LEN,
        seeds = [SEED_LISTING, &u64_le_bytes(config.next_listing_id)],
        bump
    )]
    pub listing: Account<'info, ListingState>,

    pub system_program: Program<'info, System>,
}

pub fn handler_create_miner_listing(
    ctx: Context<MarketplaceCreateMinerListing>,
    price_ess: u64,
) -> Result<()> {
    let miner = &mut ctx.accounts.miner;
    require!(!miner.listed, MoeError::AssetAlreadyListed);
    require!(
        miner.allocated_land == Pubkey::default(),
        MoeError::MinerMustBeUnassignedForListing
    );

    let cfg = &mut ctx.accounts.config;
    fill_listing_common(
        &mut ctx.accounts.listing,
        cfg.next_listing_id,
        ctx.accounts.seller.key(),
        ListingAssetKind::Miner,
        price_ess,
        ctx.bumps.listing,
    )?;

    ctx.accounts.listing.miner = miner.key();
    miner.listed = true;

    cfg.next_listing_id = cfg.next_listing_id.saturating_add(1);
    Ok(())
}

#[derive(Accounts)]
pub struct MarketplaceCancelMinerListing<'info> {
    pub seller: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_LISTING, &u64_le_bytes(listing.id)],
        bump = listing.bump,
        constraint = listing.seller == seller.key() @ MoeError::NotSeller,
        constraint = listing.asset_kind == ListingAssetKind::Miner as u8 @ MoeError::InvalidListing
    )]
    pub listing: Account<'info, ListingState>,

    #[account(mut, constraint = miner.key() == listing.miner @ MoeError::InvalidListing)]
    pub miner: Account<'info, MinerState>,
}

pub fn handler_cancel_miner_listing(ctx: Context<MarketplaceCancelMinerListing>) -> Result<()> {
    require!(ctx.accounts.listing.active, MoeError::ListingInactive);

    ctx.accounts.miner.listed = false;
    ctx.accounts.listing.active = false;
    Ok(())
}

#[derive(Accounts)]
pub struct MarketplaceBuyMinerListing<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_LISTING, &u64_le_bytes(listing.id)],
        bump = listing.bump,
        constraint = listing.asset_kind == ListingAssetKind::Miner as u8 @ MoeError::InvalidListing
    )]
    pub listing: Account<'info, ListingState>,

    #[account(mut)]
    pub miner: Account<'info, MinerState>,

    #[account(
        mut,
        seeds = [SEED_MINER_PROGRESS, miner.key().as_ref()],
        bump = miner_progress.bump,
        constraint = miner_progress.miner == miner.key() @ MoeError::InvalidMinerProgress
    )]
    pub miner_progress: Account<'info, MinerProgress>,

    #[account(
        mut,
        seeds = [SEED_MINER_MINING, miner.key().as_ref()],
        bump = miner_mining.bump,
        constraint = miner_mining.miner == miner.key() @ MoeError::InvalidMinerRef
    )]
    pub miner_mining: Account<'info, MinerMiningState>,

    #[account(
        mut,
        seeds = [SEED_EQUIPMENT, miner.key().as_ref()],
        bump = equipment.bump,
        constraint = equipment.miner == miner.key() @ MoeError::InvalidMinerRef
    )]
    pub equipment: Account<'info, EquipmentState>,

    /// CHECK: checked against listing.seller
    pub seller: UncheckedAccount<'info>,

    #[account(mut, seeds = [SEED_ECONOMY], bump = economy.bump)]
    pub economy: Account<'info, EconomyConfig>,

    pub ess_mint: Account<'info, Mint>,

    #[account(mut)]
    pub buyer_ata: Account<'info, TokenAccount>,

    #[account(mut)]
    pub seller_ata: Account<'info, TokenAccount>,

    #[account(mut)]
    pub recipient_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler_buy_miner_listing(ctx: Context<MarketplaceBuyMinerListing>) -> Result<()> {
    validate_buy_common(
        ctx.accounts.buyer.key(),
        &ctx.accounts.seller,
        &ctx.accounts.listing,
        &ctx.accounts.economy,
        &ctx.accounts.ess_mint,
        &ctx.accounts.buyer_ata,
        &ctx.accounts.seller_ata,
        &ctx.accounts.recipient_ata,
    )?;

    require!(
        ctx.accounts.miner.key() == ctx.accounts.listing.miner,
        MoeError::InvalidListing
    );
    require!(ctx.accounts.miner.listed, MoeError::InvalidListing);
    require!(
        ctx.accounts.miner.owner == ctx.accounts.seller.key(),
        MoeError::InvalidListing
    );
    require!(
        ctx.accounts.miner_progress.owner == ctx.accounts.seller.key(),
        MoeError::InvalidListing
    );
    require!(
        ctx.accounts.miner_mining.owner == ctx.accounts.seller.key(),
        MoeError::InvalidListing
    );
    require!(
        ctx.accounts.equipment.owner == ctx.accounts.seller.key(),
        MoeError::InvalidListing
    );

    settle_buy_transfers(
        &ctx.accounts.token_program,
        &ctx.accounts.buyer,
        &ctx.accounts.buyer_ata,
        &ctx.accounts.seller_ata,
        &ctx.accounts.recipient_ata,
        &mut ctx.accounts.economy,
        &ctx.accounts.listing,
    )?;

    let buyer_key = ctx.accounts.buyer.key();
    ctx.accounts.miner.owner = buyer_key;
    ctx.accounts.miner.listed = false;
    ctx.accounts.miner_progress.owner = buyer_key;
    ctx.accounts.miner_mining.owner = buyer_key;
    ctx.accounts.equipment.owner = buyer_key;

    ctx.accounts.listing.active = false;
    Ok(())
}

#[derive(Accounts)]
pub struct MarketplaceCreateLandListing<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,

    #[account(mut, seeds = [SEED_CONFIG], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(mut, constraint = land.owner == seller.key() @ MoeError::Unauthorized)]
    pub land: Account<'info, LandState>,

    #[account(
        init,
        payer = seller,
        space = ListingState::LEN,
        seeds = [SEED_LISTING, &u64_le_bytes(config.next_listing_id)],
        bump
    )]
    pub listing: Account<'info, ListingState>,

    pub system_program: Program<'info, System>,
}

pub fn handler_create_land_listing(
    ctx: Context<MarketplaceCreateLandListing>,
    price_ess: u64,
) -> Result<()> {
    let land = &mut ctx.accounts.land;
    require!(!land.listed, MoeError::AssetAlreadyListed);
    require!(
        land.allocated_miners_count == 0,
        MoeError::LandHasAllocatedMiners
    );

    let cfg = &mut ctx.accounts.config;
    fill_listing_common(
        &mut ctx.accounts.listing,
        cfg.next_listing_id,
        ctx.accounts.seller.key(),
        ListingAssetKind::Land,
        price_ess,
        ctx.bumps.listing,
    )?;

    ctx.accounts.listing.land = land.key();
    land.listed = true;

    cfg.next_listing_id = cfg.next_listing_id.saturating_add(1);
    Ok(())
}

#[derive(Accounts)]
pub struct MarketplaceCancelLandListing<'info> {
    pub seller: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_LISTING, &u64_le_bytes(listing.id)],
        bump = listing.bump,
        constraint = listing.seller == seller.key() @ MoeError::NotSeller,
        constraint = listing.asset_kind == ListingAssetKind::Land as u8 @ MoeError::InvalidListing
    )]
    pub listing: Account<'info, ListingState>,

    #[account(mut, constraint = land.key() == listing.land @ MoeError::InvalidListing)]
    pub land: Account<'info, LandState>,
}

pub fn handler_cancel_land_listing(ctx: Context<MarketplaceCancelLandListing>) -> Result<()> {
    require!(ctx.accounts.listing.active, MoeError::ListingInactive);

    ctx.accounts.land.listed = false;
    ctx.accounts.listing.active = false;
    Ok(())
}

#[derive(Accounts)]
pub struct MarketplaceBuyLandListing<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_LISTING, &u64_le_bytes(listing.id)],
        bump = listing.bump,
        constraint = listing.asset_kind == ListingAssetKind::Land as u8 @ MoeError::InvalidListing
    )]
    pub listing: Account<'info, ListingState>,

    #[account(mut)]
    pub land: Account<'info, LandState>,

    /// CHECK: checked against listing.seller
    pub seller: UncheckedAccount<'info>,

    #[account(mut, seeds = [SEED_ECONOMY], bump = economy.bump)]
    pub economy: Account<'info, EconomyConfig>,

    pub ess_mint: Account<'info, Mint>,

    #[account(mut)]
    pub buyer_ata: Account<'info, TokenAccount>,

    #[account(mut)]
    pub seller_ata: Account<'info, TokenAccount>,

    #[account(mut)]
    pub recipient_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler_buy_land_listing(ctx: Context<MarketplaceBuyLandListing>) -> Result<()> {
    validate_buy_common(
        ctx.accounts.buyer.key(),
        &ctx.accounts.seller,
        &ctx.accounts.listing,
        &ctx.accounts.economy,
        &ctx.accounts.ess_mint,
        &ctx.accounts.buyer_ata,
        &ctx.accounts.seller_ata,
        &ctx.accounts.recipient_ata,
    )?;

    require!(
        ctx.accounts.land.key() == ctx.accounts.listing.land,
        MoeError::InvalidListing
    );
    require!(ctx.accounts.land.listed, MoeError::InvalidListing);
    require!(
        ctx.accounts.land.owner == ctx.accounts.seller.key(),
        MoeError::InvalidListing
    );
    require!(
        ctx.accounts.land.allocated_miners_count == 0,
        MoeError::LandHasAllocatedMiners
    );

    settle_buy_transfers(
        &ctx.accounts.token_program,
        &ctx.accounts.buyer,
        &ctx.accounts.buyer_ata,
        &ctx.accounts.seller_ata,
        &ctx.accounts.recipient_ata,
        &mut ctx.accounts.economy,
        &ctx.accounts.listing,
    )?;

    ctx.accounts.land.owner = ctx.accounts.buyer.key();
    ctx.accounts.land.listed = false;
    ctx.accounts.listing.active = false;
    Ok(())
}

#[derive(Accounts)]
pub struct MarketplaceCreateEquipmentListing<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,

    #[account(mut, seeds = [SEED_CONFIG], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [SEED_EQUIPMENT_INVENTORY, seller.key().as_ref()],
        bump = inventory.bump,
        constraint = inventory.owner == seller.key() @ MoeError::Unauthorized
    )]
    pub inventory: Account<'info, EquipmentInventoryState>,

    #[account(
        init,
        payer = seller,
        space = ListingState::LEN,
        seeds = [SEED_LISTING, &u64_le_bytes(config.next_listing_id)],
        bump
    )]
    pub listing: Account<'info, ListingState>,

    pub system_program: Program<'info, System>,
}

pub fn handler_create_equipment_listing(
    ctx: Context<MarketplaceCreateEquipmentListing>,
    bucket: u8,
    level: u8,
    amount: u16,
    price_ess: u64,
) -> Result<()> {
    reserve_inventory(&mut ctx.accounts.inventory, bucket, level, amount)?;

    let cfg = &mut ctx.accounts.config;
    fill_listing_common(
        &mut ctx.accounts.listing,
        cfg.next_listing_id,
        ctx.accounts.seller.key(),
        ListingAssetKind::EquipmentInventory,
        price_ess,
        ctx.bumps.listing,
    )?;

    let listing = &mut ctx.accounts.listing;
    listing.inventory_owner = ctx.accounts.seller.key();
    listing.equipment_bucket = bucket;
    listing.equipment_level = level;
    listing.equipment_amount = amount;

    cfg.next_listing_id = cfg.next_listing_id.saturating_add(1);
    Ok(())
}

#[derive(Accounts)]
pub struct MarketplaceCancelEquipmentListing<'info> {
    pub seller: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_LISTING, &u64_le_bytes(listing.id)],
        bump = listing.bump,
        constraint = listing.seller == seller.key() @ MoeError::NotSeller,
        constraint = listing.asset_kind == ListingAssetKind::EquipmentInventory as u8 @ MoeError::InvalidListing
    )]
    pub listing: Account<'info, ListingState>,

    #[account(
        mut,
        seeds = [SEED_EQUIPMENT_INVENTORY, seller.key().as_ref()],
        bump = inventory.bump,
        constraint = inventory.owner == seller.key() @ MoeError::Unauthorized,
        constraint = listing.inventory_owner == seller.key() @ MoeError::InvalidListing
    )]
    pub inventory: Account<'info, EquipmentInventoryState>,
}

pub fn handler_cancel_equipment_listing(
    ctx: Context<MarketplaceCancelEquipmentListing>,
) -> Result<()> {
    require!(ctx.accounts.listing.active, MoeError::ListingInactive);

    release_inventory(
        &mut ctx.accounts.inventory,
        ctx.accounts.listing.equipment_bucket,
        ctx.accounts.listing.equipment_level,
        ctx.accounts.listing.equipment_amount,
    )?;

    ctx.accounts.listing.active = false;
    Ok(())
}

#[derive(Accounts)]
pub struct MarketplaceBuyEquipmentListing<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_LISTING, &u64_le_bytes(listing.id)],
        bump = listing.bump,
        constraint = listing.asset_kind == ListingAssetKind::EquipmentInventory as u8 @ MoeError::InvalidListing
    )]
    pub listing: Account<'info, ListingState>,

    /// CHECK: checked against listing.seller
    pub seller: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [SEED_EQUIPMENT_INVENTORY, buyer.key().as_ref()],
        bump = buyer_inventory.bump,
        constraint = buyer_inventory.owner == buyer.key() @ MoeError::Unauthorized
    )]
    pub buyer_inventory: Account<'info, EquipmentInventoryState>,

    #[account(mut, seeds = [SEED_ECONOMY], bump = economy.bump)]
    pub economy: Account<'info, EconomyConfig>,

    pub ess_mint: Account<'info, Mint>,

    #[account(mut)]
    pub buyer_ata: Account<'info, TokenAccount>,

    #[account(mut)]
    pub seller_ata: Account<'info, TokenAccount>,

    #[account(mut)]
    pub recipient_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler_buy_equipment_listing(ctx: Context<MarketplaceBuyEquipmentListing>) -> Result<()> {
    validate_buy_common(
        ctx.accounts.buyer.key(),
        &ctx.accounts.seller,
        &ctx.accounts.listing,
        &ctx.accounts.economy,
        &ctx.accounts.ess_mint,
        &ctx.accounts.buyer_ata,
        &ctx.accounts.seller_ata,
        &ctx.accounts.recipient_ata,
    )?;

    require!(
        ctx.accounts.listing.inventory_owner == ctx.accounts.seller.key(),
        MoeError::InvalidListing
    );

    settle_buy_transfers(
        &ctx.accounts.token_program,
        &ctx.accounts.buyer,
        &ctx.accounts.buyer_ata,
        &ctx.accounts.seller_ata,
        &ctx.accounts.recipient_ata,
        &mut ctx.accounts.economy,
        &ctx.accounts.listing,
    )?;

    release_inventory(
        &mut ctx.accounts.buyer_inventory,
        ctx.accounts.listing.equipment_bucket,
        ctx.accounts.listing.equipment_level,
        ctx.accounts.listing.equipment_amount,
    )?;

    ctx.accounts.listing.active = false;
    Ok(())
}
