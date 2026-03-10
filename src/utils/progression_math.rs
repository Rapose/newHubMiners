use crate::state::ProgressionConfig;

fn pow_bps(base_bps: u16, mut exp: u16) -> u64 {
    let mut result: u64 = 10_000; // 1.0
    let mut base: u64 = base_bps as u64;

    while exp > 0 {
        if exp & 1 == 1 {
            result = result.saturating_mul(base).saturating_div(10_000);
        }
        base = base.saturating_mul(base).saturating_div(10_000);
        exp >>= 1;
    }
    result
}

pub fn exp_required(cfg: &ProgressionConfig, rarity_idx: usize, level: u16) -> u64 {
    let base = cfg.exp_base_by_rarity[rarity_idx];
    let mult = pow_bps(cfg.exp_growth_bps, level.saturating_sub(1));
    base.saturating_mul(mult).saturating_div(10_000)
}

pub fn ess_cost(cfg: &ProgressionConfig, rarity_idx: usize, level: u16) -> u64 {
    let base = cfg.ess_base_cost_by_rarity[rarity_idx];
    let mult = pow_bps(cfg.ess_growth_bps, level.saturating_sub(1));
    base.saturating_mul(mult).saturating_div(10_000)
}
