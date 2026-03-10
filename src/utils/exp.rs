use crate::state::MinerProgress;

pub fn add_exp(progress: &mut MinerProgress, amount: u64) {
    progress.exp = progress.exp.saturating_add(amount);
}