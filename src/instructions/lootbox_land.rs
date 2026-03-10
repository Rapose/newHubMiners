use crate::{
    constants::*,
    errors::MoeError,
    state::{Config, LandState, LootboxLandState},
    utils::{commitment, rng32, u64_le_bytes},
};
use anchor_lang::prelude::*;

fn build_entropy(owner: &Pubkey, slot: u64) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[..8].copy_from_slice(&slot.to_le_bytes());
    out[8..32].copy_from_slice(&owner.to_bytes()[..24]);
    out
}

fn pick_land_rarity(roll: u16) -> u8 {
    if roll < 7_000 {
        0
    } else if roll < 9_000 {
        1
    } else if roll < 9_700 {
        2
    } else if roll < 9_950 {
        3
    } else {
        4
    }
}

fn slots_by_rarity(r: u8, dna_byte: u8) -> u8 {
    match r {
        0 => 2 + (dna_byte % 2),
        1 => 4 + (dna_byte % 2),
        2 => 7 + (dna_byte % 2),
        3 => 10 + (dna_byte % 2),
        4 => 13 + (dna_byte % 2),
        _ => 2,
    }
}

#[derive(Accounts)]
#[instruction(lootbox_id: u64)]
pub struct LootboxLandInit<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        seeds = [SEED_CONFIG],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,

    #[account(
        init,
        payer = owner,
        space = LootboxLandState::LEN,
        seeds = [SEED_LB_LAND, owner.key().as_ref(), &u64_le_bytes(lootbox_id)],
        bump
    )]
    pub lootbox: Account<'info, LootboxLandState>,

    pub system_program: Program<'info, System>,
}

pub fn handler_init(ctx: Context<LootboxLandInit>, lootbox_id: u64) -> Result<()> {
    require!(!ctx.accounts.config.paused, MoeError::Paused);
    let lb = &mut ctx.accounts.lootbox;
    lb.lootbox_id = lootbox_id;
    lb.owner = ctx.accounts.owner.key();
    lb.committed = false;
    lb.revealed = false;
    lb.commit_slot = 0;
    lb.commitment = [0u8; 32];
    lb.rarity = 0;
    lb.element = 0;
    lb.slots = 0;
    lb.bump = ctx.bumps.lootbox;
    Ok(())
}

#[derive(Accounts)]
#[instruction(lootbox_id: u64)]
pub struct LootboxLandCommit<'info> {
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_LB_LAND, owner.key().as_ref(), &u64_le_bytes(lootbox_id)],
        bump = lootbox.bump,
        constraint = lootbox.owner == owner.key() @ MoeError::Unauthorized
    )]
    pub lootbox: Account<'info, LootboxLandState>,
}

pub fn handler_commit(
    ctx: Context<LootboxLandCommit>,
    lootbox_id: u64,
    salt32: [u8; 32],
) -> Result<()> {
    let lb = &mut ctx.accounts.lootbox;
    require!(lb.lootbox_id == lootbox_id, MoeError::LootboxIdMismatch);
    require!(!lb.committed, MoeError::AlreadyCommitted);
    require!(!lb.revealed, MoeError::AlreadyRevealed);

    lb.commitment = commitment(DOMAIN_LAND, lootbox_id, &salt32);
    lb.committed = true;
    lb.commit_slot = Clock::get()?.slot;
    Ok(())
}

#[derive(Accounts)]
#[instruction(lootbox_id: u64)]
pub struct LootboxLandReveal<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_CONFIG],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [SEED_LB_LAND, owner.key().as_ref(), &u64_le_bytes(lootbox_id)],
        bump = lootbox.bump,
        constraint = lootbox.owner == owner.key() @ MoeError::Unauthorized
    )]
    pub lootbox: Account<'info, LootboxLandState>,

    #[account(
        init,
        payer = owner,
        space = LandState::LEN,
        seeds = [SEED_LAND, owner.key().as_ref(), &u64_le_bytes(config.next_land_id)],
        bump
    )]
    pub land_state: Account<'info, LandState>,

    pub system_program: Program<'info, System>,
}

pub fn handler_reveal(
    ctx: Context<LootboxLandReveal>,
    lootbox_id: u64,
    salt32: [u8; 32],
) -> Result<()> {
    let cfg = &mut ctx.accounts.config;
    require!(!cfg.paused, MoeError::Paused);

    let now_slot = Clock::get()?.slot;
    let lb = &mut ctx.accounts.lootbox;
    require!(lb.lootbox_id == lootbox_id, MoeError::LootboxIdMismatch);
    require!(lb.committed, MoeError::NotCommitted);
    require!(!lb.revealed, MoeError::AlreadyRevealed);
    require!(
        now_slot.saturating_sub(lb.commit_slot) >= MIN_REVEAL_DELAY_SLOTS,
        MoeError::RevealTooEarly
    );
    require!(
        now_slot.saturating_sub(lb.commit_slot) <= MAX_COMMIT_AGE_SLOTS,
        MoeError::RevealExpired
    );

    let expected = commitment(DOMAIN_LAND, lootbox_id, &salt32);
    require!(expected == lb.commitment, MoeError::CommitmentMismatch);

    let entropy = build_entropy(&ctx.accounts.owner.key(), now_slot);
    let rarity_bytes = rng32(&entropy, &lb.commitment, lootbox_id);
    let rarity_roll = u16::from_le_bytes([rarity_bytes[0], rarity_bytes[1]]) % 10_000;
    let rarity = pick_land_rarity(rarity_roll);

    let dna = rng32(&entropy, &lb.commitment, lootbox_id ^ 0x4d4f455f4c414e44);
    let element = dna[0] % 5;
    let slots = slots_by_rarity(rarity, dna[1]);

    lb.rarity = rarity;
    lb.element = element;
    lb.slots = slots;
    lb.revealed = true;

    let land = &mut ctx.accounts.land_state;
    land.id = cfg.next_land_id;
    land.owner = ctx.accounts.owner.key();
    land.rarity = rarity;
    land.element = element;
    land.slots = slots;
    land.created_at = Clock::get()?.unix_timestamp;
    land.bump = ctx.bumps.land_state;

    cfg.next_land_id = cfg.next_land_id.saturating_add(1);
    Ok(())
}
