use anchor_lang::prelude::*;

#[error_code]
pub enum MoeError {
    #[msg("Reveal window expired")]
    RevealExpired,

    #[msg("Protocol is paused")]
    Paused,

    #[msg("Invalid element")]
    InvalidElement,

    #[msg("Invalid slots")]
    InvalidSlots,

    #[msg("Lootbox id mismatch")]
    LootboxIdMismatch,

    #[msg("Lootbox not initialized")]
    LootboxNotInitialized,

    #[msg("Already committed")]
    AlreadyCommitted,

    #[msg("Not committed")]
    NotCommitted,

    #[msg("Reveal too early")]
    RevealTooEarly,

    #[msg("Already revealed")]
    AlreadyRevealed,

    #[msg("Commitment mismatch")]
    CommitmentMismatch,

    #[msg("Sysvar slot hashes mismatch")]
    InvalidSlotHashesSysvar,

    #[msg("Invalid BPS (sum must be 10000)")]
    InvalidBpsSum,

    #[msg("Mint mismatch")]
    MintMismatch,

    #[msg("Recipient wallet mismatch")]
    RecipientMismatch,

    #[msg("Unauthorized")]
    Unauthorized,

    #[msg("Invalid miner progress account")]
    InvalidMinerProgress,

    #[msg("Invalid rarity")]
    InvalidRarity,

    #[msg("Max level reached for this rarity")]
    MaxLevelReached,

    #[msg("Not enough EXP")]
    NotEnoughExp,

    #[msg("Invalid salt length. Expected 32 bytes.")]
    InvalidSaltLength,

    #[msg("Invalid tick length")]
    InvalidTickLen,
    #[msg("Invalid miner reference")]
    InvalidMinerRef,
    #[msg("Already claimed")]
    AlreadyClaimed,
    #[msg("Invalid upgrade (new level must be greater than current level)")]
    InvalidUpgrade,

    #[msg("Invalid equipment parameters")]
    InvalidEquipmentParams,

    #[msg("Inventory not enough items (need 4 total) for remelt.")]
    NotEnoughForRemelt,

    #[msg("Invalid base level.")]
    InvalidBaseLevel,

    #[msg("Invalid ESS cost.")]
    InvalidEssCost,

    #[msg("No item available in inventory")]
    NoItemAvailable,
    

    #[msg("Listing is inactive")]
    ListingInactive,
    #[msg("Invalid listing")]
    InvalidListing,
    #[msg("Only seller can cancel listing")]
    NotSeller,
    #[msg("Self purchase not allowed")]
    SelfPurchaseNotAllowed,
    #[msg("Listing price must be greater than zero")]
    ListingPriceInvalid,
    #[msg("Asset already listed")]
    AssetAlreadyListed,
    #[msg("Asset busy")]
    AssetBusy,
    #[msg("Miner must be unassigned for listing")]
    MinerMustBeUnassignedForListing,
    #[msg("Land has allocated miners")]
    LandHasAllocatedMiners,
    #[msg("Insufficient equipment inventory")]
    InsufficientEquipmentInventory,
    #[msg("Equipment bucket mismatch")]
    EquipmentBucketMismatch,
    #[msg("Invalid equipment amount")]
    InvalidEquipmentAmount,
    #[msg("Asset is listed and locked")]
    AssetListedLocked,

    #[msg("Math overflow")]
    MathOverflow,
}
