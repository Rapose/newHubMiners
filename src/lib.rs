#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;
pub mod utils;

use instructions::*;
use crate::instructions::affinity::ComputeAffinity;
use instructions::equipment_inventory::EquipmentSlotKind;

declare_id!("5LRTVxfrJmec3PWmNVAnis3ZgAYVYJEMsqJN3Xnhnq79");

#[program]
pub mod moe_anchor_v1 {
    use super::*;

    pub fn equipment_inventory_init(ctx: Context<EquipmentInventoryInit>) -> Result<()> {
        instructions::equipment_inventory::handler(ctx)
    }

    pub fn equipment_init(ctx: Context<EquipmentInit>) -> Result<()> {
        instructions::equipment::handler_init(ctx)
    }

    pub fn equipment_replace_hand(ctx: Context<EquipmentReplaceHand>, new_level: u8) -> Result<()> {
    instructions::equipment::handler_replace_hand(ctx, new_level)
}

    pub fn equipment_replace_head(ctx: Context<EquipmentReplaceHead>, new_level: u8) -> Result<()> {
    instructions::equipment::handler_replace_head(ctx, new_level)
}

    pub fn equipment_remelt_hand(ctx: Context<EquipmentRemeltHand>, base_level: u8) -> Result<()> {
    instructions::equipment::handler_remelt_hand(ctx, base_level)
}

    pub fn equipment_remelt_head(ctx: Context<EquipmentRemeltHead>, base_level: u8) -> Result<()> {
    instructions::equipment::handler_remelt_head(ctx, base_level)
}

pub fn equipment_inventory_grant_item(
    ctx: Context<EquipmentInventoryGrantItem>,
    slot: EquipmentSlotKind,
    level: u8,
    amount: u16,
    remelted: bool,
    broken: bool,
) -> Result<()> {
    instructions::equipment_inventory::handler_grant_item(
        ctx,
        slot,
        level,
        amount,
        remelted,
        broken,
    )
}
    pub fn global_mining_freeze_week(ctx: Context<GlobalMiningFreezeWeek>) -> Result<()> {
        instructions::global_mining::handler_freeze_week(ctx)
    }

    pub fn global_mining_init(
        ctx: Context<GlobalMiningInit>,
        tick_len_sec: u32,
        weekly_pool_amount: u64,
    ) -> Result<()> {
        instructions::global_mining::handler_init(ctx, tick_len_sec, weekly_pool_amount)
    }

    pub fn global_mining_rollover_week(
        ctx: Context<GlobalMiningRolloverWeek>,
        new_weekly_pool_amount: u64,
    ) -> Result<()> {
        instructions::global_mining::handler_rollover_week(ctx, new_weekly_pool_amount)
    }

    pub fn global_mining_register_miner(ctx: Context<GlobalMiningRegisterMiner>) -> Result<()> {
        instructions::global_mining::handler_register_miner(ctx)
    }

    pub fn global_mining_update(ctx: Context<GlobalMiningUpdate>) -> Result<()> {
        instructions::global_mining::handler_update(ctx)
    }

    pub fn global_mining_claim(ctx: Context<GlobalMiningClaim>) -> Result<u64> {
        instructions::global_mining::handler_claim(ctx)
    }

    pub fn global_mining_assign_land(ctx: Context<GlobalMiningAssignLand>) -> Result<()> {
        instructions::global_mining::handler_assign_land(ctx)
    }

    pub fn global_mining_unassign_land(ctx: Context<GlobalMiningUnassignLand>) -> Result<()> {
        instructions::global_mining::handler_unassign_land(ctx)
    }

    pub fn admin_grant_exp(ctx: Context<AdminGrantExp>, amount: u64) -> Result<()> {
        instructions::admin_grant_exp::handler(ctx, amount)
    }

    pub fn rewards_deposit(ctx: Context<RewardsDeposit>, amount: u64) -> Result<()> {
        instructions::economy::handler_rewards_deposit(ctx, amount)
    }

    pub fn economy_set_rewards_vault(ctx: Context<EconomySetRewardsVault>) -> Result<()> {
        instructions::economy::handler_set_rewards_vault(ctx)
    }

    pub fn progression_init(ctx: Context<ProgressionInit>) -> Result<()> {
        instructions::progression_init::handler(ctx)
    }

    pub fn claim_mining_exp(ctx: Context<ClaimMiningExp>) -> Result<()> {
        instructions::claim_mining_exp::handler(ctx)
    }

    pub fn miner_level_up(ctx: Context<MinerLevelUp>) -> Result<()> {
        instructions::miner_level_up::handler(ctx)
    }

    pub fn economy_init(ctx: Context<EconomyInit>) -> Result<()> {
        instructions::economy::handler_economy_init(ctx)
    }

    pub fn economy_set_recipient(ctx: Context<EconomySetRecipient>) -> Result<()> {
        instructions::economy::handler_set_recipient(ctx)
    }

    pub fn economy_set_mint(ctx: Context<EconomySetMint>) -> Result<()> {
        instructions::economy::handler_set_mint(ctx)
    }

    pub fn spend_buy(ctx: Context<SpendEss>, amount: u64) -> Result<()> {
        instructions::economy::handler_spend_buy(ctx, amount)
    }

    pub fn spend_send(ctx: Context<SpendEss>, amount: u64) -> Result<()> {
        instructions::economy::handler_spend_send(ctx, amount)
    }

    pub fn spend_recharge(ctx: Context<SpendEss>, amount: u64) -> Result<()> {
        instructions::economy::handler_spend_recharge(ctx, amount)
    }

    pub fn spend_trade_fee(ctx: Context<SpendEss>, fee_amount: u64) -> Result<()> {
        instructions::economy::handler_spend_trade_fee(ctx, fee_amount)
    }

    pub fn initialize_config(ctx: Context<InitializeConfig>) -> Result<()> {
        instructions::config::handler_initialize_config(ctx)
    }

    pub fn set_paused(ctx: Context<SetPaused>, paused: bool) -> Result<()> {
        instructions::config::handler_set_paused(ctx, paused)
    }

    pub fn create_miner_debug(
        ctx: Context<CreateMinerDebug>,
        rarity: u8,
        element: u8,
        hash_base: u64,
    ) -> Result<()> {
        instructions::debug::handler_create_miner_debug(ctx, rarity, element, hash_base)
    }

    pub fn create_land_debug(
        ctx: Context<CreateLandDebug>,
        rarity: u8,
        element: u8,
        slots: u8,
    ) -> Result<()> {
        instructions::debug::handler_create_land_debug(ctx, rarity, element, slots)
    }

    pub fn compute_affinity(
    ctx: Context<ComputeAffinity>,
    land_element: u8,
    miner_element: u8,
) -> Result<()> {
    instructions::affinity::handler_compute_affinity(ctx, land_element, miner_element)
}

    pub fn lootbox_miner_init(ctx: Context<LootboxMinerInit>, lootbox_id: u64) -> Result<()> {
        instructions::lootbox_miner::handler_init(ctx, lootbox_id)
    }

    pub fn lootbox_miner_commit(
        ctx: Context<LootboxMinerCommit>,
        lootbox_id: u64,
        salt: [u8; 32],
    ) -> Result<()> {
        instructions::lootbox_miner::handler_commit(ctx, lootbox_id, salt)
    }

    pub fn lootbox_miner_reveal(
        ctx: Context<LootboxMinerReveal>,
        lootbox_id: u64,
        salt: [u8; 32],
    ) -> Result<()> {
        instructions::lootbox_miner::handler_reveal(ctx, lootbox_id, salt)
    }

    pub fn lootbox_land_init(ctx: Context<LootboxLandInit>, lootbox_id: u64) -> Result<()> {
        instructions::lootbox_land::handler_init(ctx, lootbox_id)
    }

    pub fn lootbox_land_commit(
        ctx: Context<LootboxLandCommit>,
        lootbox_id: u64,
        salt: [u8; 32],
    ) -> Result<()> {
        instructions::lootbox_land::handler_commit(ctx, lootbox_id, salt)
    }

    pub fn lootbox_land_reveal(
        ctx: Context<LootboxLandReveal>,
        lootbox_id: u64,
        salt: [u8; 32],
    ) -> Result<()> {
        instructions::lootbox_land::handler_reveal(ctx, lootbox_id, salt)
    }
}
