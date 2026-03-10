use anchor_lang::prelude::*;
use crate::utils::require_element;

pub fn compute_affinity_bps(miner_element: u8, land_element: u8) -> Result<u16> {
    require_element(miner_element)?;
    require_element(land_element)?;

    if miner_element == land_element {
        return Ok(10_000);
    }

    let strong_against = (miner_element + 1) % 5;
    let weak_against = (miner_element + 4) % 5;

    if land_element == strong_against {
        Ok(11_000)
    } else if land_element == weak_against {
        Ok(8_000)
    } else {
        Ok(10_000)
    }
}