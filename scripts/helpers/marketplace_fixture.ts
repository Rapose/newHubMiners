import * as anchor from "@coral-xyz/anchor";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountIdempotentInstruction,
  createInitializeMintInstruction,
  createMintToInstruction,
  createTransferInstruction,
  getAccount,
  getAssociatedTokenAddressSync,
  getOrCreateAssociatedTokenAccount,
  getMint,
} from "@solana/spl-token";
import { Keypair, PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import { airdropIfNeeded, bn, loadEnv, waitSlots } from "./test_env";

const SEED = {
  CONFIG: Buffer.from("config"),
  ECONOMY: Buffer.from("economy_v4"),
  REWARDS_AUTH: Buffer.from("rewards_auth"),
  PROGRESSION: Buffer.from("progression_v1"),
  GLOBAL_MINING: Buffer.from("global_mining_v2"),
  EQUIP_INV: Buffer.from("equipment_inventory_v1"),
  EQUIPMENT: Buffer.from("equipment_v1"),
  MINER: Buffer.from("miner"),
  LAND: Buffer.from("land"),
  MINER_PROGRESS: Buffer.from("miner_progress_v1"),
  MINER_MINING: Buffer.from("miner_mining_v1"),
  LB_MINER: Buffer.from("lb_miner"),
  LISTING: Buffer.from("listing_v1"),
};

type Fixture = {
  env: ReturnType<typeof loadEnv>;
  seller: Keypair;
  buyer: Keypair;
  recipient: Keypair;
  essMint: PublicKey;
  config: PublicKey;
  progression: PublicKey;
  economy: PublicKey;
  globalMining: PublicKey;
  rewardsAuthority: PublicKey;
  rewardsVault: PublicKey;
  recipientAta: PublicKey;
  sellerAta: PublicKey;
  buyerAta: PublicKey;

  miner: PublicKey;
  minerProgress: PublicKey;
  minerMining: PublicKey;
  equipment: PublicKey;
  land: PublicKey;
  sellerInventory: PublicKey;
  buyerInventory: PublicKey;
};

function pda(programId: PublicKey, ...parts: (Buffer | Uint8Array)[]) {
  return PublicKey.findProgramAddressSync(parts, programId)[0];
}

async function ensureMint(env: ReturnType<typeof loadEnv>, mintAuthority: PublicKey): Promise<PublicKey> {
  const fs = require("fs");
  const path = ".ess_mint.tmp";
  if (fs.existsSync(path)) {
    const v = fs.readFileSync(path, "utf8").trim();
    return new PublicKey(v);
  }

  const mint = Keypair.generate();
  const rent = await env.provider.connection.getMinimumBalanceForRentExemption(82);
  const tx = new Transaction().add(
    SystemProgram.createAccount({
      fromPubkey: env.admin.publicKey,
      newAccountPubkey: mint.publicKey,
      lamports: rent,
      space: 82,
      programId: TOKEN_PROGRAM_ID,
    }),
    createInitializeMintInstruction(mint.publicKey, 6, mintAuthority, null),
  );
  await env.provider.sendAndConfirm(tx, [mint]);
  fs.writeFileSync(path, mint.publicKey.toBase58());
  return mint.publicKey;
}

async function ensureConfig(env: ReturnType<typeof loadEnv>) {
  const config = pda(env.program.programId, SEED.CONFIG);
  try {
    await env.program.account.config.fetch(config);
  } catch {
    await env.program.methods.initializeConfig().accounts({
      admin: env.admin.publicKey,
      config,
      systemProgram: SystemProgram.programId,
    }).rpc();
  }
  return config;
}

async function ensureProgression(env: ReturnType<typeof loadEnv>) {
  const progression = pda(env.program.programId, SEED.PROGRESSION);
  try {
    await env.program.account.progressionConfig.fetch(progression);
  } catch {
    await env.program.methods.progressionInit().accounts({
      admin: env.admin.publicKey,
      progression,
      systemProgram: SystemProgram.programId,
    }).rpc();
  }
  return progression;
}

async function ensureGlobalMining(env: ReturnType<typeof loadEnv>) {
  const global = pda(env.program.programId, SEED.GLOBAL_MINING);
  try {
    await env.program.account.globalMiningState.fetch(global);
  } catch {
    await env.program.methods.globalMiningInit(10, bn(1_000_000_000)).accounts({
      admin: env.admin.publicKey,
      global,
      systemProgram: SystemProgram.programId,
    }).rpc();
  }
  return global;
}

async function ensureEconomy(
  env: ReturnType<typeof loadEnv>,
  essMint: PublicKey,
  recipientWallet: PublicKey,
): Promise<{ economy: PublicKey; rewardsAuthority: PublicKey; rewardsVault: PublicKey; recipientAta: PublicKey }> {
  const economy = pda(env.program.programId, SEED.ECONOMY);
  const rewardsAuthority = pda(env.program.programId, SEED.REWARDS_AUTH);

  const rewardsVault = getAssociatedTokenAddressSync(essMint, rewardsAuthority, true, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID);
  const recipientAta = getAssociatedTokenAddressSync(essMint, recipientWallet);

  const ix: any[] = [
    createAssociatedTokenAccountIdempotentInstruction(
      env.admin.publicKey,
      rewardsVault,
      rewardsAuthority,
      essMint,
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID,
    ),
    createAssociatedTokenAccountIdempotentInstruction(
      env.admin.publicKey,
      recipientAta,
      recipientWallet,
      essMint,
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID,
    ),
  ];
  await env.provider.sendAndConfirm(new Transaction().add(...ix), []);

  try {
    await env.program.account.economyConfig.fetch(economy);
  } catch {
    await env.program.methods.economyInit().accounts({
      admin: env.admin.publicKey,
      essMint,
      recipientWallet,
      economy,
      rewardsAuthority,
      rewardsVault,
      systemProgram: SystemProgram.programId,
    }).rpc();
  }

  return { economy, rewardsAuthority, rewardsVault, recipientAta };
}

async function createMinerWithProgress(env: ReturnType<typeof loadEnv>, config: PublicKey, owner: Keypair) {
  const lootboxId = Date.now();
  const salt = Uint8Array.from(Array(32).fill(7));

  const lootbox = pda(env.program.programId, SEED.LB_MINER, owner.publicKey.toBuffer(), Buffer.from(new anchor.BN(lootboxId).toArray("le", 8)));

  const cfg: any = await env.program.account.config.fetch(config);
  const nextMinerId = Number(cfg.nextMinerId);
  const miner = pda(env.program.programId, SEED.MINER, owner.publicKey.toBuffer(), Buffer.from(new anchor.BN(nextMinerId).toArray("le", 8)));
  const minerProgress = pda(env.program.programId, SEED.MINER_PROGRESS, miner.toBuffer());

  await env.program.methods.lootboxMinerInit(bn(lootboxId)).accounts({
    owner: owner.publicKey,
    config,
    lootbox,
    systemProgram: SystemProgram.programId,
  }).signers([owner]).rpc();

  await env.program.methods.lootboxMinerCommit(bn(lootboxId), [...salt]).accounts({
    owner: owner.publicKey,
    lootbox,
  }).signers([owner]).rpc();

  await waitSlots(env.provider, 2);

  await env.program.methods.lootboxMinerReveal(bn(lootboxId), [...salt]).accounts({
    owner: owner.publicKey,
    config,
    lootbox,
    minerState: miner,
    minerProgress,
    systemProgram: SystemProgram.programId,
  }).signers([owner]).rpc();

  return { miner, minerProgress };
}

async function createLand(env: ReturnType<typeof loadEnv>, config: PublicKey, owner: Keypair) {
  const cfg: any = await env.program.account.config.fetch(config);
  const id = Number(cfg.nextLandId);
  const land = pda(env.program.programId, SEED.LAND, owner.publicKey.toBuffer(), Buffer.from(new anchor.BN(id).toArray("le", 8)));
  await env.program.methods.createLandDebug(1, 1, 2).accounts({
    owner: owner.publicKey,
    config,
    landState: land,
    systemProgram: SystemProgram.programId,
  }).signers([owner]).rpc();
  return land;
}

export async function buildMarketplaceFixture(): Promise<Fixture> {
  const env = loadEnv();
  const seller = Keypair.generate();
  const buyer = Keypair.generate();
  const recipient = Keypair.generate();

  await airdropIfNeeded(env.provider, seller.publicKey);
  await airdropIfNeeded(env.provider, buyer.publicKey);
  await airdropIfNeeded(env.provider, recipient.publicKey);

  const config = await ensureConfig(env);
  const progression = await ensureProgression(env);
  const globalMining = await ensureGlobalMining(env);

  const essMint = await ensureMint(env, env.admin.publicKey);
  const { economy, rewardsAuthority, rewardsVault, recipientAta } = await ensureEconomy(
    env,
    essMint,
    recipient.publicKey,
  );

  const sellerAta = (await getOrCreateAssociatedTokenAccount(env.provider.connection, env.admin.payer as any, essMint, seller.publicKey)).address;
  const buyerAta = (await getOrCreateAssociatedTokenAccount(env.provider.connection, env.admin.payer as any, essMint, buyer.publicKey)).address;

  await env.provider.sendAndConfirm(new Transaction().add(
    createMintToInstruction(essMint, buyerAta, env.admin.publicKey, 5_000_000_000),
    createMintToInstruction(essMint, sellerAta, env.admin.publicKey, 500_000_000),
  ));

  const { miner, minerProgress } = await createMinerWithProgress(env, config, seller);
  const land = await createLand(env, config, seller);

  const equipment = pda(env.program.programId, SEED.EQUIPMENT, miner.toBuffer());
  await env.program.methods.equipmentInit().accounts({
    owner: seller.publicKey,
    minerState: miner,
    equipment,
    systemProgram: SystemProgram.programId,
  }).signers([seller]).rpc();

  await env.program.methods.globalMiningRegisterMiner().accounts({
    owner: seller.publicKey,
    global: globalMining,
    minerState: miner,
    minerMining: pda(env.program.programId, SEED.MINER_MINING, miner.toBuffer()),
    systemProgram: SystemProgram.programId,
  }).signers([seller]).rpc();

  const minerMining = pda(env.program.programId, SEED.MINER_MINING, miner.toBuffer());

  const sellerInventory = pda(env.program.programId, SEED.EQUIP_INV, seller.publicKey.toBuffer());
  const buyerInventory = pda(env.program.programId, SEED.EQUIP_INV, buyer.publicKey.toBuffer());

  await env.program.methods.equipmentInventoryInit().accounts({
    owner: seller.publicKey,
    inventory: sellerInventory,
    systemProgram: SystemProgram.programId,
  }).signers([seller]).rpc();

  await env.program.methods.equipmentInventoryInit().accounts({
    owner: buyer.publicKey,
    inventory: buyerInventory,
    systemProgram: SystemProgram.programId,
  }).signers([buyer]).rpc();

  await env.program.methods.equipmentInventoryGrantItem({ hand: {} }, 1, 3, false, false).accounts({
    admin: env.admin.publicKey,
    config,
    inventory: sellerInventory,
    owner: seller.publicKey,
  }).rpc();

  return {
    env,
    seller,
    buyer,
    recipient,
    essMint,
    config,
    progression,
    economy,
    globalMining,
    rewardsAuthority,
    rewardsVault,
    recipientAta,
    sellerAta,
    buyerAta,
    miner,
    minerProgress,
    minerMining,
    equipment,
    land,
    sellerInventory,
    buyerInventory,
  };
}

export async function tokenBalance(connection: anchor.web3.Connection, ata: PublicKey) {
  const acc = await getAccount(connection, ata);
  return Number(acc.amount);
}

export async function fetchListing(program: anchor.Program, listing: PublicKey): Promise<any> {
  return program.account.listingState.fetch(listing);
}

export function listingPda(programId: PublicKey, id: number) {
  return pda(programId, SEED.LISTING, Buffer.from(new anchor.BN(id).toArray("le", 8)));
}
