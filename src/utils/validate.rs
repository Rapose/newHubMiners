use crate::{constants::*, errors::MoeError};
use anchor_lang::prelude::*;

pub fn require_not_paused(paused: bool) -> Result<()> {
    require!(!paused, MoeError::Paused);
    Ok(())
}

pub fn require_element(e: u8) -> Result<()> {
    require!(e < ELEMENTS, MoeError::InvalidElement);
    Ok(())
}

pub fn require_rarity(r: u8) -> Result<()> {
    require!(r < RARITIES, MoeError::InvalidRarity);
    Ok(())
}

pub fn require_slots(s: u8) -> Result<()> {
    require!(s > 0 && s <= 10, MoeError::InvalidSlots);
    Ok(())
}
