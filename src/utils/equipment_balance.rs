use anchor_lang::prelude::*;
use crate::errors::MoeError;

// slot 0 = hand, slot 1 = head
pub const SLOT_HAND: u8 = 0;
pub const SLOT_HEAD: u8 = 1;

pub const MAX_EQUIPMENT_TIER: u8 = 6;

pub fn validate_equipment_tier(tier: u8) -> Result<()> {
    require!(tier >= 1 && tier <= MAX_EQUIPMENT_TIER, MoeError::InvalidBaseLevel);
    Ok(())
}

pub fn hand_power_bps_by_tier(tier: u8) -> Result<u16> {
    validate_equipment_tier(tier)?;

    let v = match tier {
        1 => 200,   // +2.0%
        2 => 450,   // +4.5%
        3 => 800,   // +8.0%
        4 => 1400,  // +14.0%
        5 => 2200,  // +22.0%
        6 => 3500,  // +35.0% (remelt-only)
        _ => return err!(MoeError::InvalidBaseLevel),
    };

    Ok(v)
}

pub fn head_discount_bps_by_tier(tier: u8) -> Result<u16> {
    validate_equipment_tier(tier)?;

    let v = match tier {
        1 => 300,   // -3.0%
        2 => 600,   // -6.0%
        3 => 1000,  // -10.0%
        4 => 1500,  // -15.0%
        5 => 2200,  // -22.0%
        6 => 3000,  // -30.0% (remelt-only)
        _ => return err!(MoeError::InvalidBaseLevel),
    };

    Ok(v)
}

// Retorna custo em "ESS inteiro", sem decimais aplicados.
// Se quiser converter para unidades do mint depois, aplique * 10^ESS_DECIMALS no caller.
pub fn remelt_cost_ess(slot: u8, base_tier: u8) -> Result<u64> {
    validate_equipment_tier(base_tier)?;
    require!(base_tier < MAX_EQUIPMENT_TIER, MoeError::InvalidBaseLevel);

    let cost = match slot {
        SLOT_HAND => match base_tier {
            1 => 10,   // T1 -> T2
            2 => 25,   // T2 -> T3
            3 => 60,   // T3 -> T4
            4 => 140,  // T4 -> T5
            5 => 320,  // T5 -> T6
            _ => return err!(MoeError::InvalidBaseLevel),
        },
        SLOT_HEAD => match base_tier {
            1 => 8,    // T1 -> T2
            2 => 20,   // T2 -> T3
            3 => 48,   // T3 -> T4
            4 => 110,  // T4 -> T5
            5 => 250,  // T5 -> T6
            _ => return err!(MoeError::InvalidBaseLevel),
        },
        _ => return err!(MoeError::InvalidBaseLevel),
    };

    Ok(cost)
}