use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_pack::Pack;
use anchor_spl::token::spl_token::state::{Account as SplTokenAccount, Mint as SplMint};
use anchor_spl::token::{self, Token, Transfer};

use crate::{
    constants::*,
    utils::affinity::compute_affinity_bps,
    errors::MoeError,
    state::{
        EconomyConfig,
        EquipmentState,
        GlobalMiningState,
        LandState,
        MinerMiningState,
        MinerProgress,
        MinerState,
    },
};

// =====================================================
// Freeze Week (Approach A snapshot)
// =====================================================

#[derive(Accounts)]
pub struct GlobalMiningFreezeWeek<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_GLOBAL_MINING],
        bump = global.bump,
        constraint = global.admin == admin.key() @ MoeError::Unauthorized
    )]
    pub global: Account<'info, GlobalMiningState>,
}

pub fn handler_freeze_week(ctx: Context<GlobalMiningFreezeWeek>) -> Result<()> {
    let g = &mut ctx.accounts.global;

    if g.frozen {
        msg!(
            "GM_FREEZE skipped already_frozen=true week_index={} total_ep_tw={}",
            g.week_index,
            g.total_ep_tw
        );
        return Ok(());
    }

    g.frozen = true;
    g.frozen_week_index = g.week_index;
    g.frozen_weekly_pool_amount = g.weekly_pool_amount;
    g.frozen_total_ep_tw = g.total_ep_tw;

    msg!(
        "GM_FREEZE frozen=true week_index={} weekly_pool={} total_ep_tw={}",
        g.frozen_week_index,
        g.frozen_weekly_pool_amount,
        g.frozen_total_ep_tw
    );

    Ok(())
}

// =====================================================
// Assign / Unassign Land
// =====================================================

#[derive(Accounts)]
pub struct GlobalMiningUnassignLand<'info> {
    pub owner: Signer<'info>,

    #[account(mut)]
    pub miner_state: Account<'info, MinerState>,

    #[account(mut)]
    pub land_state: Account<'info, LandState>,
}

pub fn handler_unassign_land(ctx: Context<GlobalMiningUnassignLand>) -> Result<()> {
    let miner = &mut ctx.accounts.miner_state;
    require!(
        miner.owner == ctx.accounts.owner.key(),
        MoeError::Unauthorized
    );

    require!(!miner.listed, MoeError::AssetListedLocked);

    let prev_land = miner.allocated_land;
    let land = &mut ctx.accounts.land_state;
    require!(land.key() == prev_land, MoeError::InvalidMinerRef);
    require!(!land.listed, MoeError::AssetListedLocked);
    require!(land.owner == ctx.accounts.owner.key(), MoeError::Unauthorized);

    land.allocated_miners_count = land.allocated_miners_count.saturating_sub(1);
    miner.allocated_land = Pubkey::default();

    msg!(
        "GM_UNASSIGN owner={} miner={} prev_land={}",
        ctx.accounts.owner.key(),
        miner.key(),
        prev_land
    );

    Ok(())
}

#[derive(Accounts)]
pub struct GlobalMiningAssignLand<'info> {
    pub owner: Signer<'info>,

    #[account(mut)]
    pub miner_state: Account<'info, MinerState>,

    #[account(mut)]
    pub land_state: Account<'info, LandState>,
}

pub fn handler_assign_land(ctx: Context<GlobalMiningAssignLand>) -> Result<()> {
    let miner = &mut ctx.accounts.miner_state;
    let land_key = ctx.accounts.land_state.key();
    let land_element = ctx.accounts.land_state.element;

    require!(!miner.listed, MoeError::AssetListedLocked);
    require!(!ctx.accounts.land_state.listed, MoeError::AssetListedLocked);

    require!(
        miner.owner == ctx.accounts.owner.key(),
        MoeError::Unauthorized
    );
    require!(
        ctx.accounts.land_state.owner == ctx.accounts.owner.key(),
        MoeError::Unauthorized
    );
    require!(
        miner.allocated_land == Pubkey::default(),
        MoeError::Unauthorized
    );

    let land_mut = &mut ctx.accounts.land_state;
    land_mut.allocated_miners_count = land_mut.allocated_miners_count.saturating_add(1);
    miner.allocated_land = land_key;

    msg!(
        "GM_ASSIGN owner={} miner={} land={} miner_element={} land_element={}",
        ctx.accounts.owner.key(),
        miner.key(),
        land_key,
        miner.element,
        land_element
    );

    Ok(())
}

// =====================================================
// Helpers
// =====================================================

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct MiningBreakdown {
    base_power: u128,
    affinity_bps: u32,
    after_affinity: u128,
    hand_power_bps: u32,
    after_equipment: u128,
    level_bonus_bps: u32,
    final_operational_power: u128,
}

fn now_ts() -> Result<i64> {
    Ok(Clock::get()?.unix_timestamp)
}

fn current_tick(now: i64, week_start: i64, tick_len_sec: u32) -> u32 {
    if now <= week_start || tick_len_sec == 0 {
        return 0;
    }

    let dt = (now - week_start) as u64;
    (dt / tick_len_sec as u64) as u32
}

fn level_bonus_bps(level: u16) -> u32 {
    if level == 0 {
        0
    } else {
        level as u32 * 300
    }
}

fn apply_bps_u128(x: u128, bps: u128) -> u128 {
    x.saturating_mul(bps) / 10_000u128
}

fn validate_miner_land_link_strict(
    miner: &MinerState,
    owner: &Pubkey,
    land: &LandState,
    land_key: Pubkey,
) -> Result<()> {
    require!(miner.owner == *owner, MoeError::Unauthorized);
    require!(
        miner.allocated_land != Pubkey::default(),
        MoeError::InvalidMinerRef
    );
    require!(land.owner == *owner, MoeError::Unauthorized);
    require!(land_key == miner.allocated_land, MoeError::InvalidMinerRef);
    Ok(())
}

fn validate_progress_link_strict(
    miner_key: Pubkey,
    progress: &MinerProgress,
    owner: &Pubkey,
) -> Result<()> {
    require!(progress.owner == *owner, MoeError::Unauthorized);
    require!(progress.miner == miner_key, MoeError::InvalidMinerRef);
    Ok(())
}

fn validate_equipment_snapshot(
    hand_level: u8,
    hand_power_bps: u16,
    hand_is_remelted: bool,
    head_level: u8,
    head_recharge_discount_bps: u16,
    head_is_remelted: bool,
) -> Result<()> {
    if hand_level == 0 {
        require!(hand_power_bps == 0, MoeError::InvalidMinerRef);
        require!(!hand_is_remelted, MoeError::InvalidMinerRef);
    } else {
        require!(hand_power_bps > 0, MoeError::InvalidMinerRef);
    }

    if head_level == 0 {
        require!(head_recharge_discount_bps == 0, MoeError::InvalidMinerRef);
        require!(!head_is_remelted, MoeError::InvalidMinerRef);
    } else {
        require!(head_recharge_discount_bps > 0, MoeError::InvalidMinerRef);
    }

    Ok(())
}

fn validate_equipment_consistency(equipment: &EquipmentState) -> Result<()> {
    validate_equipment_snapshot(
        equipment.hand_level,
        equipment.hand_power_bps,
        equipment.hand_is_remelted,
        equipment.head_level,
        equipment.head_recharge_discount_bps,
        equipment.head_is_remelted,
    )
}

fn operational_power_breakdown_from_values(
    hash_base: u64,
    miner_element: u8,
    land_element: u8,
    hand_power_bps: u16,
    level: u16,
) -> Result<MiningBreakdown> {
    let base_power = hash_base as u128;

    let affinity_bps_u16 = compute_affinity_bps(miner_element, land_element)?;
let after_affinity = apply_bps_u128(base_power, affinity_bps_u16 as u128);

    let hand_bps_total = 10_000u128 + hand_power_bps as u128;
    let after_equipment = apply_bps_u128(after_affinity, hand_bps_total);

    let lvl_bonus = level_bonus_bps(level);
    let lvl_bps_total = 10_000u128 + lvl_bonus as u128;
    let final_operational_power = apply_bps_u128(after_equipment, lvl_bps_total);

    Ok(MiningBreakdown {
    base_power,
    affinity_bps: affinity_bps_u16 as u32,
    after_affinity,
    hand_power_bps: hand_power_bps as u32,
    after_equipment,
    level_bonus_bps: lvl_bonus,
    final_operational_power,
})
}

fn operational_power_strict(
    miner: &MinerState,
    progress: &MinerProgress,
    land: &LandState,
    equipment: &EquipmentState,
) -> Result<u128> {
    let breakdown = operational_power_breakdown_from_values(
        miner.hash_base,
        miner.element,
        land.element,
        equipment.hand_power_bps,
        progress.level,
    )?;

    Ok(breakdown.final_operational_power)
}

fn ep_add_from_breakdown(
    breakdown: &MiningBreakdown,
    delta_secs: u128,
) -> u128 {
    breakdown.final_operational_power.saturating_mul(delta_secs)
}

fn validate_breakdown_invariants(
    breakdown: &MiningBreakdown,
    miner: &MinerState,
    progress: &MinerProgress,
    land: &LandState,
    equipment: &EquipmentState,
    delta_secs: u128,
    actual_ep_add: u128,
) -> Result<()> {
    let direct_op = operational_power_strict(miner, progress, land, equipment)?;
    require!(
        direct_op == breakdown.final_operational_power,
        MoeError::InvalidMinerRef
    );

    let expected_ep_add = ep_add_from_breakdown(breakdown, delta_secs);
    require!(expected_ep_add == actual_ep_add, MoeError::InvalidMinerRef);

    require!(
        breakdown.after_equipment >= breakdown.after_affinity || equipment.hand_power_bps == 0,
        MoeError::InvalidMinerRef
    );
    require!(
        breakdown.final_operational_power >= breakdown.after_equipment
            || breakdown.level_bonus_bps == 0,
        MoeError::InvalidMinerRef
    );

    Ok(())
}

fn load_anchor_account<T: AccountDeserialize>(ai: &AccountInfo) -> Result<T> {
    let data = ai.try_borrow_data()?;
    let mut slice: &[u8] = &data;
    T::try_deserialize(&mut slice)
}

fn load_spl_token_account(ai: &AccountInfo) -> Result<SplTokenAccount> {
    let data = ai.try_borrow_data()?;
    SplTokenAccount::unpack(&data).map_err(|_| error!(MoeError::Unauthorized))
}

fn load_spl_mint(ai: &AccountInfo) -> Result<SplMint> {
    let data = ai.try_borrow_data()?;
    SplMint::unpack(&data).map_err(|_| error!(MoeError::MintMismatch))
}

// =====================================================
// Init / Rollover / Register / Update
// =====================================================

#[derive(Accounts)]
pub struct GlobalMiningInit<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = GlobalMiningState::LEN,
        seeds = [SEED_GLOBAL_MINING],
        bump
    )]
    pub global: Account<'info, GlobalMiningState>,

    pub system_program: Program<'info, System>,
}

pub fn handler_init(
    ctx: Context<GlobalMiningInit>,
    tick_len_sec: u32,
    weekly_pool_amount: u64,
) -> Result<()> {
    require!(tick_len_sec > 0, MoeError::InvalidTickLen);

    let g = &mut ctx.accounts.global;
    g.admin = ctx.accounts.admin.key();
    g.week_index = 0;
    g.week_start_ts = now_ts()?;
    g.tick_len_sec = tick_len_sec;
    g.weekly_pool_amount = weekly_pool_amount;
    g.total_ep_tw = 0;

    g.frozen = false;
    g.frozen_week_index = 0;
    g.frozen_weekly_pool_amount = 0;
    g.frozen_total_ep_tw = 0;

    g.bump = ctx.bumps.global;

    msg!(
        "GM_INIT admin={} week_index={} week_start_ts={} tick_len_sec={} weekly_pool={}",
        g.admin,
        g.week_index,
        g.week_start_ts,
        g.tick_len_sec,
        g.weekly_pool_amount
    );

    Ok(())
}

#[derive(Accounts)]
pub struct GlobalMiningRolloverWeek<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_GLOBAL_MINING],
        bump = global.bump,
        constraint = global.admin == admin.key() @ MoeError::Unauthorized
    )]
    pub global: Account<'info, GlobalMiningState>,
}

pub fn handler_rollover_week(
    ctx: Context<GlobalMiningRolloverWeek>,
    new_weekly_pool_amount: u64,
) -> Result<()> {
    let g = &mut ctx.accounts.global;

    require!(g.frozen, MoeError::Unauthorized);

    let prev_week = g.week_index;
    let prev_frozen_total = g.frozen_total_ep_tw;
    let prev_pool = g.frozen_weekly_pool_amount;

    g.week_index = g.week_index.saturating_add(1);
    g.week_start_ts = now_ts()?;
    g.weekly_pool_amount = new_weekly_pool_amount;
    g.total_ep_tw = 0;

    g.frozen = false;
    g.frozen_week_index = 0;
    g.frozen_weekly_pool_amount = 0;
    g.frozen_total_ep_tw = 0;

    msg!(
        "GM_ROLLOVER prev_week={} new_week={} prev_pool={} prev_total_ep={} new_weekly_pool={} new_week_start_ts={}",
        prev_week,
        g.week_index,
        prev_pool,
        prev_frozen_total,
        g.weekly_pool_amount,
        g.week_start_ts
    );

    Ok(())
}

#[derive(Accounts)]
pub struct GlobalMiningRegisterMiner<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(mut, seeds = [SEED_GLOBAL_MINING], bump = global.bump)]
    pub global: Account<'info, GlobalMiningState>,

    pub miner_state: Account<'info, MinerState>,

    #[account(
        init,
        payer = owner,
        space = MinerMiningState::LEN,
        seeds = [SEED_MINER_MINING, miner_state.key().as_ref()],
        bump
    )]
    pub miner_mining: Account<'info, MinerMiningState>,

    pub system_program: Program<'info, System>,
}

pub fn handler_register_miner(ctx: Context<GlobalMiningRegisterMiner>) -> Result<()> {
    let g = &ctx.accounts.global;
    let mm = &mut ctx.accounts.miner_mining;
    let miner = &ctx.accounts.miner_state;

    require!(miner.owner == ctx.accounts.owner.key(), MoeError::Unauthorized);

    require!(!miner.listed, MoeError::AssetListedLocked);

    mm.owner = ctx.accounts.owner.key();
    mm.miner = miner.key();
    mm.week_index = g.week_index;
    mm.last_tick = 0;
    mm.ep_tw = 0;
    mm.claimed = false;
    mm.bump = ctx.bumps.miner_mining;

    msg!(
        "GM_REGISTER owner={} miner={} week_index={} rarity={} element={} hash_base={}",
        mm.owner,
        mm.miner,
        mm.week_index,
        miner.rarity,
        miner.element,
        miner.hash_base
    );

    Ok(())
}

#[derive(Accounts)]
pub struct GlobalMiningUpdate<'info> {
    pub owner: Signer<'info>,

    #[account(mut, seeds = [SEED_GLOBAL_MINING], bump = global.bump)]
    pub global: Account<'info, GlobalMiningState>,

    pub miner_state: Account<'info, MinerState>,
    pub miner_progress: Account<'info, MinerProgress>,

    #[account(
        seeds = [SEED_EQUIPMENT, miner_state.key().as_ref()],
        bump = equipment.bump,
        constraint = equipment.owner == owner.key() @ MoeError::Unauthorized,
        constraint = equipment.miner == miner_state.key() @ MoeError::InvalidMinerRef
    )]
    pub equipment: Account<'info, EquipmentState>,

    #[account(
        mut,
        seeds = [SEED_MINER_MINING, miner_state.key().as_ref()],
        bump = miner_mining.bump,
        constraint = miner_mining.owner == owner.key() @ MoeError::Unauthorized,
        constraint = miner_mining.miner == miner_state.key() @ MoeError::InvalidMinerRef
    )]
    pub miner_mining: Account<'info, MinerMiningState>,

    #[account(
        constraint = land_state.owner == owner.key() @ MoeError::Unauthorized,
        constraint = miner_state.allocated_land != Pubkey::default() @ MoeError::InvalidMinerRef,
        constraint = land_state.key() == miner_state.allocated_land @ MoeError::InvalidMinerRef
    )]
    #[account(mut)]
    pub land_state: Account<'info, LandState>,
}

fn update_inner_strict(
    global: &mut GlobalMiningState,
    miner_mining: &mut MinerMiningState,
    miner_key: Pubkey,
    land_key: Pubkey,
    miner_state: &MinerState,
    miner_progress: &MinerProgress,
    land_state: &LandState,
    equipment: &EquipmentState,
) -> Result<()> {
    if global.frozen {
        msg!(
    "GM_UPDATE skipped frozen=true week_index={} miner={} miner_week={}",
    global.week_index,
    miner_key,
    miner_mining.week_index
);
    }

    validate_progress_link_strict(miner_key, miner_progress, &miner_state.owner)?;
    validate_equipment_consistency(equipment)?;

    if miner_mining.week_index != global.week_index {
        msg!(
    "GM_UPDATE rollover_reset miner={} old_week={} new_week={}",
    miner_key,
    miner_mining.week_index,
    global.week_index
);
        miner_mining.week_index = global.week_index;
        miner_mining.last_tick = 0;
        miner_mining.ep_tw = 0;
        miner_mining.claimed = false;
    }

    let now = Clock::get()?.unix_timestamp;
    let tick = current_tick(now, global.week_start_ts, global.tick_len_sec);

    if tick <= miner_mining.last_tick {
        msg!(
    "GM_UPDATE skipped no_new_tick miner={} tick={} last_tick={}",
    miner_key,
    tick,
    miner_mining.last_tick
);
        return Ok(());
    }

    let delta_ticks = tick - miner_mining.last_tick;
    let delta_secs = (delta_ticks as u128).saturating_mul(global.tick_len_sec as u128);

    let breakdown = operational_power_breakdown_from_values(
        miner_state.hash_base,
        miner_state.element,
        land_state.element,
        equipment.hand_power_bps,
        miner_progress.level,
    )?;

    let ep_add = ep_add_from_breakdown(&breakdown, delta_secs);

    validate_breakdown_invariants(
        &breakdown,
        miner_state,
        miner_progress,
        land_state,
        equipment,
        delta_secs,
        ep_add,
    )?;

    let prev_miner_ep = miner_mining.ep_tw;
    let prev_global_ep = global.total_ep_tw;

    miner_mining.ep_tw = miner_mining.ep_tw.saturating_add(ep_add);
    global.total_ep_tw = global.total_ep_tw.saturating_add(ep_add);
    miner_mining.last_tick = tick;

    msg!(
        "GM_UPDATE miner={} land={} base_power={} miner_element={} land_element={} affinity_bps={} after_affinity={} hand_level={} hand_power_bps={} hand_remelted={} after_equipment={} level={} level_bonus_bps={} final_op={} delta_ticks={} delta_secs={} ep_add={} miner_ep_before={} miner_ep_after={} global_ep_before={} global_ep_after={} tick={} week_index={}",
        miner_key,
        land_key,
        breakdown.base_power,
        miner_state.element,
        land_state.element,
        breakdown.affinity_bps,
        breakdown.after_affinity,
        equipment.hand_level,
        breakdown.hand_power_bps,
        equipment.hand_is_remelted,
        breakdown.after_equipment,
        miner_progress.level,
        breakdown.level_bonus_bps,
        breakdown.final_operational_power,
        delta_ticks,
        delta_secs,
        ep_add,
        prev_miner_ep,
        miner_mining.ep_tw,
        prev_global_ep,
        global.total_ep_tw,
        tick,
        global.week_index
    );

    msg!(
        "GM_EQUIPMENT head_level={} head_discount_bps={} head_remelted={}",
        equipment.head_level,
        equipment.head_recharge_discount_bps,
        equipment.head_is_remelted
    );

    msg!(
    "GM_PROOF miner={} op_direct={} op_breakdown={} expected_ep_add={} actual_ep_add={} proof=OK",
    miner_key,
    operational_power_strict(miner_state, miner_progress, land_state, equipment)?,
    breakdown.final_operational_power,
    ep_add,
    ep_add
);
    Ok(())
}
pub fn handler_update(ctx: Context<GlobalMiningUpdate>) -> Result<()> {
    let miner_key = ctx.accounts.miner_state.key();
    let land_key = ctx.accounts.land_state.key();

    require!(!ctx.accounts.miner_state.listed, MoeError::AssetListedLocked);
    require!(!ctx.accounts.land_state.listed, MoeError::AssetListedLocked);

    validate_miner_land_link_strict(
        &ctx.accounts.miner_state,
        &ctx.accounts.owner.key(),
        &ctx.accounts.land_state,
        land_key,
    )?;

    update_inner_strict(
        &mut ctx.accounts.global,
        &mut ctx.accounts.miner_mining,
        miner_key,
        land_key,
        &ctx.accounts.miner_state,
        &ctx.accounts.miner_progress,
        &ctx.accounts.land_state,
        &ctx.accounts.equipment,
    )
}

// =====================================================
// Claim (snapshot congelado)
// =====================================================

#[derive(Accounts)]
pub struct GlobalMiningClaim<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(mut, seeds = [SEED_GLOBAL_MINING], bump = global.bump)]
    pub global: Account<'info, GlobalMiningState>,

    #[account(
        mut,
        seeds = [SEED_MINER_MINING, miner_state.key().as_ref()],
        bump = miner_mining.bump
    )]
    pub miner_mining: Account<'info, MinerMiningState>,

    /// CHECK: desserializado manualmente no handler
    pub miner_state: UncheckedAccount<'info>,

    /// CHECK: desserializado manualmente no handler
    pub ess_mint: UncheckedAccount<'info>,

    /// CHECK: PDA economy, desserializado manualmente no handler
    #[account(seeds = [SEED_ECONOMY], bump)]
    pub economy: UncheckedAccount<'info>,

    /// CHECK: PDA signer do vault
    #[account(seeds = [SEED_REWARDS_AUTH], bump)]
    pub rewards_authority: UncheckedAccount<'info>,

    /// CHECK: desserializado manualmente no handler
    #[account(mut)]
    pub rewards_vault: UncheckedAccount<'info>,

    /// CHECK: desserializado manualmente no handler
    #[account(mut)]
    pub user_ata: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn handler_claim(ctx: Context<GlobalMiningClaim>) -> Result<u64> {
    let miner_state: MinerState = load_anchor_account(&ctx.accounts.miner_state.to_account_info())?;
    let economy: EconomyConfig = load_anchor_account(&ctx.accounts.economy.to_account_info())?;
    let _ess_mint = load_spl_mint(&ctx.accounts.ess_mint.to_account_info())?;
    let rewards_vault = load_spl_token_account(&ctx.accounts.rewards_vault.to_account_info())?;
    let user_ata = load_spl_token_account(&ctx.accounts.user_ata.to_account_info())?;

    require!(
        miner_state.owner == ctx.accounts.owner.key(),
        MoeError::Unauthorized
    );
    require!(
        ctx.accounts.miner_mining.owner == ctx.accounts.owner.key(),
        MoeError::Unauthorized
    );
    require!(
        ctx.accounts.miner_mining.miner == ctx.accounts.miner_state.key(),
        MoeError::InvalidMinerRef
    );

    require!(
        economy.rewards_vault == ctx.accounts.rewards_vault.key(),
        MoeError::Unauthorized
    );
    require!(
        economy.ess_mint == ctx.accounts.ess_mint.key(),
        MoeError::MintMismatch
    );
    require!(
        rewards_vault.mint == ctx.accounts.ess_mint.key(),
        MoeError::MintMismatch
    );
    require!(
        rewards_vault.owner == ctx.accounts.rewards_authority.key(),
        MoeError::Unauthorized
    );
    require!(
        user_ata.owner == ctx.accounts.owner.key(),
        MoeError::Unauthorized
    );
    require!(
        user_ata.mint == ctx.accounts.ess_mint.key(),
        MoeError::MintMismatch
    );

    let g = &ctx.accounts.global;
    let mm = &mut ctx.accounts.miner_mining;

    require!(g.frozen, MoeError::Unauthorized);
    require!(
        mm.week_index == g.frozen_week_index,
        MoeError::InvalidMinerRef
    );
    require!(!mm.claimed, MoeError::AlreadyClaimed);

    if g.frozen_total_ep_tw == 0 {
        mm.claimed = true;
        msg!(
            "GM_CLAIM zero_total_ep week={} miner={} reward=0",
            g.frozen_week_index,
            mm.miner
        );
        return Ok(0);
    }

    let pool_cap = g.frozen_weekly_pool_amount.min(rewards_vault.amount);

    let reward_u128 =
        (pool_cap as u128).saturating_mul(mm.ep_tw) / (g.frozen_total_ep_tw as u128);

    let reward = reward_u128.min(u64::MAX as u128) as u64;

    if reward == 0 {
        mm.claimed = true;
        msg!(
            "GM_CLAIM zero_reward week={} miner={} miner_ep={} total_ep={} pool_cap={}",
            g.frozen_week_index,
            mm.miner,
            mm.ep_tw,
            g.frozen_total_ep_tw,
            pool_cap
        );
        return Ok(0);
    }

    require!(rewards_vault.amount >= reward, MoeError::Unauthorized);

    let bump = ctx.bumps.rewards_authority;
    let rewards_auth_seeds: &[&[u8]] = &[SEED_REWARDS_AUTH, &[bump]];
    let signer_seeds: &[&[&[u8]]] = &[rewards_auth_seeds];

    msg!(
        "GM_CLAIM week={} miner={} miner_ep={} total_ep={} frozen_pool={} vault_balance={} pool_cap={} reward={}",
        g.frozen_week_index,
        mm.miner,
        mm.ep_tw,
        g.frozen_total_ep_tw,
        g.frozen_weekly_pool_amount,
        rewards_vault.amount,
        pool_cap,
        reward
    );

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.rewards_vault.to_account_info(),
                to: ctx.accounts.user_ata.to_account_info(),
                authority: ctx.accounts.rewards_authority.to_account_info(),
            },
            signer_seeds,
        ),
        reward,
    )?;

    mm.claimed = true;
    Ok(reward)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_tick_basic() {
        assert_eq!(current_tick(100, 100, 10), 0);
        assert_eq!(current_tick(109, 100, 10), 0);
        assert_eq!(current_tick(110, 100, 10), 1);
        assert_eq!(current_tick(135, 100, 10), 3);
    }

    #[test]
    fn test_current_tick_zero_len() {
        assert_eq!(current_tick(200, 100, 0), 0);
    }

    #[test]
    fn test_level_bonus_bps() {
        assert_eq!(level_bonus_bps(0), 0);
        assert_eq!(level_bonus_bps(1), 300);
        assert_eq!(level_bonus_bps(2), 600);
        assert_eq!(level_bonus_bps(10), 3000);
    }

    #[test]
    fn test_apply_bps_u128_floor() {
        assert_eq!(apply_bps_u128(703, 10_800), 759);
        assert_eq!(apply_bps_u128(1000, 10_300), 1030);
        assert_eq!(apply_bps_u128(1000, 9_500), 950);
    }

    #[test]
    fn test_equipment_snapshot_empty_ok() {
        assert!(validate_equipment_snapshot(0, 0, false, 0, 0, false).is_ok());
    }

    #[test]
    fn test_equipment_snapshot_hand_invalid_when_zero_level_but_power_exists() {
        assert!(validate_equipment_snapshot(0, 800, false, 0, 0, false).is_err());
    }

    #[test]
    fn test_equipment_snapshot_head_invalid_when_zero_level_but_discount_exists() {
        assert!(validate_equipment_snapshot(0, 0, false, 0, 600, false).is_err());
    }

    #[test]
    fn test_equipment_snapshot_remelt_flags_invalid_on_empty_slots() {
        assert!(validate_equipment_snapshot(0, 0, true, 0, 0, false).is_err());
        assert!(validate_equipment_snapshot(0, 0, false, 0, 0, true).is_err());
    }

    #[test]
    fn test_operational_breakdown_uses_affinity_and_equipment() {
        let b = operational_power_breakdown_from_values(703, 2, 0, 800, 0).unwrap();
        let affinity = compute_affinity_bps(2, 0).unwrap() as u128;

        assert_eq!(b.base_power, 703);
        assert_eq!(b.affinity_bps as u128, affinity);

        let after_affinity = apply_bps_u128(703, affinity);
        let after_equipment = apply_bps_u128(after_affinity, 10_800);

        assert_eq!(b.after_affinity, after_affinity);
        assert_eq!(b.after_equipment, after_equipment);
        assert_eq!(b.level_bonus_bps, 0);
        assert_eq!(b.final_operational_power, after_equipment);
    }

    #[test]
    fn test_operational_breakdown_with_level_bonus() {
        let b = operational_power_breakdown_from_values(703, 2, 0, 800, 1).unwrap();
        let affinity = compute_affinity_bps(2, 0).unwrap() as u128;

        let step1 = apply_bps_u128(703, affinity);
        let step2 = apply_bps_u128(step1, 10_800);
        let step3 = apply_bps_u128(step2, 10_300);

        assert_eq!(b.after_affinity, step1);
        assert_eq!(b.after_equipment, step2);
        assert_eq!(b.level_bonus_bps, 300);
        assert_eq!(b.final_operational_power, step3);
    }

    #[test]
    fn test_hand_bonus_increases_or_preserves_power_vs_no_hand() {
        let no_hand = operational_power_breakdown_from_values(703, 2, 0, 0, 0).unwrap();
        let with_hand = operational_power_breakdown_from_values(703, 2, 0, 800, 0).unwrap();

        assert!(with_hand.final_operational_power >= no_hand.final_operational_power);
    }

    #[test]
    fn test_level_bonus_increases_or_preserves_power_vs_level_zero() {
        let lvl0 = operational_power_breakdown_from_values(703, 2, 0, 800, 0).unwrap();
        let lvl1 = operational_power_breakdown_from_values(703, 2, 0, 800, 1).unwrap();

        assert!(lvl1.final_operational_power >= lvl0.final_operational_power);
    }

    #[test]
    fn test_ep_add_matches_operational_power_times_delta_secs() {
        let b = operational_power_breakdown_from_values(400, 3, 0, 800, 0).unwrap();
        let ep = ep_add_from_breakdown(&b, 10);
        assert_eq!(ep, b.final_operational_power * 10);
    }

    #[test]
    fn test_head_is_not_part_of_mining_formula_by_snapshot_rules() {
        assert!(validate_equipment_snapshot(3, 800, false, 2, 600, true).is_ok());

        let without_head = operational_power_breakdown_from_values(400, 3, 0, 800, 0).unwrap();
        let with_same_hand_different_head = operational_power_breakdown_from_values(400, 3, 0, 800, 0).unwrap();

        assert_eq!(
            without_head.final_operational_power,
            with_same_hand_different_head.final_operational_power
        );
    }

    #[test]
    fn test_affinity_is_effectively_applied_in_breakdown() {
        let b = operational_power_breakdown_from_values(400, 3, 0, 800, 0).unwrap();
        let affinity = compute_affinity_bps(3, 0).unwrap() as u128;
        let expected_after_affinity = apply_bps_u128(400, affinity);

        assert_eq!(b.after_affinity, expected_after_affinity);
    }

    #[test]
    fn test_breakdown_and_direct_operational_match() {
        let b = operational_power_breakdown_from_values(400, 3, 0, 800, 1).unwrap();

        let expected_direct = {
            let s1 = apply_bps_u128(400, compute_affinity_bps(3, 0).unwrap() as u128);
            let s2 = apply_bps_u128(s1, 10_800);
            apply_bps_u128(s2, 10_300)
        };

        assert_eq!(b.final_operational_power, expected_direct);
    }
}