use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default)]
pub struct Allocation7 {
    pub burn_bps: u16,
    pub liquidity_bps: u16,
    pub mining_pool_bps: u16,
    pub marketing_bps: u16,
    pub dev_infra_bps: u16,
    pub forges_bps: u16,
    pub treasury_bps: u16,
}

impl Allocation7 {
    pub fn sum(&self) -> u32 {
        self.burn_bps as u32
            + self.liquidity_bps as u32
            + self.mining_pool_bps as u32
            + self.marketing_bps as u32
            + self.dev_infra_bps as u32
            + self.forges_bps as u32
            + self.treasury_bps as u32
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default)]
pub struct Allocation3 {
    pub burn_bps: u16,
    pub forges_bps: u16,
    pub treasury_bps: u16,
}

impl Allocation3 {
    pub fn sum(&self) -> u32 {
        self.burn_bps as u32 + self.forges_bps as u32 + self.treasury_bps as u32
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default)]
pub struct Totals7 {
    pub burn: u64,
    pub liquidity: u64,
    pub mining_pool: u64,
    pub marketing: u64,
    pub dev_infra: u64,
    pub forges: u64,
    pub treasury: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default)]
pub struct Totals3 {
    pub burn: u64,
    pub forges: u64,
    pub treasury: u64,
}

#[account]
pub struct EconomyConfig {
    pub admin: Pubkey,

    pub ess_mint: Pubkey,
    pub recipient_wallet: Pubkey,

    pub rewards_authority: Pubkey,
    pub rewards_vault: Pubkey,

    pub buy: Allocation7,
    pub trade_fee: Allocation7,
    pub send: Allocation7,
    pub recharge: Allocation3,

    pub totals_buy: Totals7,
    pub totals_trade_fee: Totals7,
    pub totals_send: Totals7,
    pub totals_recharge: Totals3,

    pub bump: u8,
}

impl EconomyConfig {
    pub const LEN: usize = 8
        + 32
        + 32
        + 32
        + 32
        + 32
        + (2 * 7)
        + (2 * 7)
        + (2 * 7)
        + (2 * 3)
        + (8 * 7)
        + (8 * 7)
        + (8 * 7)
        + (8 * 3)
        + 1;
}
