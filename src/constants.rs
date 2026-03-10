pub const SEED_CONFIG: &[u8] = b"config";
pub const SEED_MINER: &[u8] = b"miner";
pub const SEED_LAND: &[u8] = b"land";
pub const SEED_AFF: &[u8] = b"affinity_v2";
pub const SEED_LB_MINER: &[u8] = b"lb_miner";
pub const SEED_LB_LAND: &[u8] = b"lb_land";

pub const ELEMENTS: u8 = 5; // 0..4
pub const RARITIES: u8 = 5; // 0..4

pub const MIN_REVEAL_DELAY_SLOTS: u64 = 2;
pub const MAX_COMMIT_AGE_SLOTS: u64 = 21_600; // ~3 dias em slots de ~400ms

pub const DOMAIN_MINER: &[u8] = b"MOE:LB:MINER";
pub const DOMAIN_LAND: &[u8] = b"MOE:LB:LAND";

pub const SEED_ECONOMY: &[u8] = b"economy_v4";

pub const SEED_GLOBAL_MINING: &[u8] = b"global_mining_v2";
pub const SEED_MINER_MINING: &[u8] = b"miner_mining_v1";

pub const SEED_REWARDS_AUTH: &[u8] = b"rewards_auth";

pub const BPS_DENOM: u64 = 10_000;

pub const SEED_EQUIPMENT: &[u8] = b"equipment_v1";
pub const SEED_EQUIPMENT_INVENTORY: &[u8] = b"equipment_inventory_v1";

pub const MAX_ITEM_LEVEL: usize = 10;

pub const SEED_LISTING: &[u8] = b"listing_v1";

pub const MARKETPLACE_FEE_BPS: u64 = 500;

