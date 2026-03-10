pub fn u64_le_bytes(x: u64) -> [u8; 8] {
    x.to_le_bytes()
}

pub const SEED_PROGRESSION: &[u8] = b"progression_v1";
pub const SEED_MINER_PROGRESS: &[u8] = b"miner_progress_v1";

pub const SEED_ECONOMY: &[u8] = b"economy_v4";

pub const SEED_REWARDS_AUTH: &[u8] = b"rewards_auth";
pub const SEED_REWARDS_VAULT: &[u8] = b"rewards_vault";
