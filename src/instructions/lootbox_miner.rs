use anchor_lang::prelude::*;

use crate::{
    constants::*,
    errors::MoeError,
    state::{Config, LootboxMinerState, MinerProgress, MinerState},
    utils::{commitment, rng32, u64_le_bytes, SEED_MINER_PROGRESS},
};

fn build_entropy(owner: &Pubkey, slot: u64) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[..8].copy_from_slice(&slot.to_le_bytes());
    out[8..32].copy_from_slice(&owner.to_bytes()[..24]);
    out
}

fn pick_miner_rarity(roll: u16) -> u8 {
    if roll < 5_500 {
        0
    } else if roll < 8_000 {
        1
    } else if roll < 9_200 {
        2
    } else if roll < 9_800 {
        3
    } else {
        4
    }
}

fn pick_background(rarity: u8, dna_byte: u8) -> u8 {
    match rarity {
        0 => dna_byte % 3,
        1 => 3 + (dna_byte % 3),
        2 => 6 + (dna_byte % 3),
        3 => 9 + (dna_byte % 2),
        4 => 11,
        _ => 0,
    }
}

fn pick_hash_base(rarity: u8, dna: &[u8; 32]) -> u64 {
    match rarity {
        0 => 60 + (dna[2] as u64 % 41),
        1 => 120 + (u16::from_le_bytes([dna[2], dna[3]]) as u64 % 61),
        2 => 300 + (u16::from_le_bytes([dna[3], dna[4]]) as u64 % 151),
        3 => 600 + (u16::from_le_bytes([dna[4], dna[5]]) as u64 % 301),
        4 => 1200 + (u16::from_le_bytes([dna[5], dna[6]]) as u64 % 401),
        _ => 60,
    }
}

fn pick_visuals(rarity: u8, dna: &[u8; 32]) -> (u8, u8, u8, u8, u8, u8, u8) {
    let mut face = dna[0] % 12;
    let element = dna[1] % 5;
    let mut helmet = dna[2] % 12;
    let mut backpack = dna[3] % 12;
    let mut jacket = dna[4] % 3;
    let mut item = dna[5] % 12;
    let background = pick_background(rarity, dna[6]);

    match rarity {
        0 => {
            helmet = 0;
            backpack = 0;
            jacket = 0;
            item = 0;
        }
        1 => {
            backpack = 0;
            jacket = 0;
            item = 0;
        }
        2 => {
            jacket = 0;
            item = 0;
        }
        3 => {
            item = 0;
        }
        4 => {
            if face == 0 {
                face = 1 + (dna[0] % 11);
            }
            if helmet == 0 {
                helmet = 1 + (dna[2] % 11);
            }
            if backpack == 0 {
                backpack = 1 + (dna[3] % 11);
            }
            if item == 0 {
                item = 1 + (dna[5] % 11);
            }
        }
        _ => {}
    }

    (face, element, helmet, backpack, jacket, item, background)
}

#[derive(Accounts)]
#[instruction(lootbox_id: u64)]
pub struct LootboxMinerInit<'info> {
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
        space = LootboxMinerState::LEN,
        seeds = [SEED_LB_MINER, owner.key().as_ref(), &u64_le_bytes(lootbox_id)],
        bump
    )]
    pub lootbox: Account<'info, LootboxMinerState>,

    pub system_program: Program<'info, System>,
}

pub fn handler_init(ctx: Context<LootboxMinerInit>, lootbox_id: u64) -> Result<()> {
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
    lb.hash_base = 0;
    lb.face = 0;
    lb.helmet = 0;
    lb.backpack = 0;
    lb.jacket = 0;
    lb.item = 0;
    lb.background = 0;
    lb.bump = ctx.bumps.lootbox;
    Ok(())
}

#[derive(Accounts)]
#[instruction(lootbox_id: u64)]
pub struct LootboxMinerCommit<'info> {
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_LB_MINER, owner.key().as_ref(), &u64_le_bytes(lootbox_id)],
        bump = lootbox.bump,
        constraint = lootbox.owner == owner.key() @ MoeError::Unauthorized
    )]
    pub lootbox: Account<'info, LootboxMinerState>,
}

pub fn handler_commit(
    ctx: Context<LootboxMinerCommit>,
    lootbox_id: u64,
    salt32: [u8; 32],
) -> Result<()> {
    let lb = &mut ctx.accounts.lootbox;
    require!(lb.lootbox_id == lootbox_id, MoeError::LootboxIdMismatch);
    require!(!lb.committed, MoeError::AlreadyCommitted);
    require!(!lb.revealed, MoeError::AlreadyRevealed);

    lb.commitment = commitment(DOMAIN_MINER, lootbox_id, &salt32);
    lb.committed = true;
    lb.commit_slot = Clock::get()?.slot;
    Ok(())
}

#[derive(Accounts)]
#[instruction(lootbox_id: u64)]
pub struct LootboxMinerReveal<'info> {
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
        seeds = [SEED_LB_MINER, owner.key().as_ref(), &u64_le_bytes(lootbox_id)],
        bump = lootbox.bump,
        constraint = lootbox.owner == owner.key() @ MoeError::Unauthorized
    )]
    pub lootbox: Account<'info, LootboxMinerState>,

    #[account(
        init,
        payer = owner,
        space = MinerState::LEN,
        seeds = [SEED_MINER, owner.key().as_ref(), &u64_le_bytes(config.next_miner_id)],
        bump
    )]
    pub miner_state: Account<'info, MinerState>,

    #[account(
        init,
        payer = owner,
        space = MinerProgress::LEN,
        seeds = [SEED_MINER_PROGRESS, miner_state.key().as_ref()],
        bump
    )]
    pub miner_progress: Account<'info, MinerProgress>,

    pub system_program: Program<'info, System>,
}

pub fn handler_reveal(
    ctx: Context<LootboxMinerReveal>,
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

    let expected = commitment(DOMAIN_MINER, lootbox_id, &salt32);
    require!(expected == lb.commitment, MoeError::CommitmentMismatch);

    let entropy = build_entropy(&ctx.accounts.owner.key(), now_slot);
    let rarity_bytes = rng32(&entropy, &lb.commitment, lootbox_id);
    let rarity_roll = u16::from_le_bytes([rarity_bytes[0], rarity_bytes[1]]) % 10_000;
    let rarity = pick_miner_rarity(rarity_roll);

    let dna = rng32(&entropy, &lb.commitment, lootbox_id ^ 0x4d4f455f4d494e45);
    let hash_base = pick_hash_base(rarity, &dna);
    let (face, element, helmet, backpack, jacket, item, background) = pick_visuals(rarity, &dna);

    lb.rarity = rarity;
    lb.element = element;
    lb.hash_base = hash_base;
    lb.face = face;
    lb.helmet = helmet;
    lb.backpack = backpack;
    lb.jacket = jacket;
    lb.item = item;
    lb.background = background;
    lb.revealed = true;

    let now_ts = Clock::get()?.unix_timestamp;

    let miner = &mut ctx.accounts.miner_state;
    miner.id = cfg.next_miner_id;
    miner.owner = ctx.accounts.owner.key();
    miner.rarity = rarity;
    miner.element = element;
    miner.hash_base = hash_base;
    miner.face = face;
    miner.helmet = helmet;
    miner.backpack = backpack;
    miner.jacket = jacket;
    miner.item = item;
    miner.background = background;
    miner.allocated_land = Pubkey::default();
    miner.created_at = now_ts;
    miner.bump = ctx.bumps.miner_state;

    let progress = &mut ctx.accounts.miner_progress;
    progress.miner = miner.key();
    progress.owner = ctx.accounts.owner.key();
    progress.level = 0;
    progress.exp = 0;
    progress.last_exp_claim_ts = now_ts;
    progress.bump = ctx.bumps.miner_progress;
msg!("DEBUG lootbox reveal progress.level={}", progress.level);
msg!("DEBUG lootbox reveal progress.exp={}", progress.exp);
msg!("DEBUG lootbox reveal miner={}", progress.miner);




    cfg.next_miner_id = cfg.next_miner_id.saturating_add(1);
    Ok(())
}
