import * as anchor from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  getAccount,
  getOrCreateAssociatedTokenAccount,
} from "@solana/spl-token";
import fs from "node:fs";

const SEED_CONFIG = Buffer.from("config");
const SEED_PROGRESSION = Buffer.from("progression_v1");
const SEED_ECONOMY = Buffer.from("economy_v4");
const SEED_MINER = Buffer.from("miner");
const SEED_MINER_PROGRESS = Buffer.from("miner_progress_v1");

const RECIPIENT_WALLET = new PublicKey(
  "Ea9pUYYtCF6usjYAd2RdqeZ2WJETSwPaPwrGmfQRktXf"
);

function u64Le(n: bigint) {
  const b = Buffer.alloc(8);
  b.writeBigUInt64LE(n);
  return b;
}

function resolveIdlPath(): string {
  const candidates = [
    "target/idl/miners.json",
    "target/idl/moe_anchor_v1.json",
  ];
  for (const p of candidates) {
    if (fs.existsSync(p)) return p;
  }
  throw new Error("IDL not found. Run `anchor build` first.");
}

function getAccountClient(program: any, names: string[]) {
  for (const n of names) {
    if (program.account?.[n]) return program.account[n];
  }
  throw new Error(`Account client not found. Tried: ${names.join(", ")}`);
}

function bnLikeToString(v: any) {
  return typeof v?.toString === "function" ? v.toString() : v;
}

function expRequired(base: number, growthBps: number, level: number): number {
  let v = base;
  for (let i = 0; i < level; i++) {
    v = Math.floor((v * growthBps) / 10_000);
  }
  return Math.max(1, v);
}

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const idlPath = resolveIdlPath();
  console.log("Using IDL:", idlPath);

  const idl = JSON.parse(fs.readFileSync(idlPath, "utf8"));
  const program = new anchor.Program(idl, provider) as anchor.Program;

  const payer = (provider.wallet as any).payer;
  const owner = provider.wallet.publicKey;

  if (!fs.existsSync(".ess_mint.tmp")) {
    throw new Error("ESS mint not found. Run create_fixed_mint.ts first.");
  }
  const essMint = new PublicKey(fs.readFileSync(".ess_mint.tmp", "utf8").trim());

  console.log("=== miner_level_up.ts ===");
  console.log("program:", program.programId.toBase58());
  console.log("owner:", owner.toBase58());
  console.log("essMint:", essMint.toBase58());

  const [configPda] = PublicKey.findProgramAddressSync(
    [SEED_CONFIG],
    program.programId
  );
  const [progressionPda] = PublicKey.findProgramAddressSync(
    [SEED_PROGRESSION],
    program.programId
  );
  const [economyPda] = PublicKey.findProgramAddressSync(
    [SEED_ECONOMY],
    program.programId
  );

  const configAccount = getAccountClient(program as any, ["config"]);
  const progressionAccount = getAccountClient(program as any, ["progressionConfig", "progression"]);
  const minerStateAccount = getAccountClient(program as any, ["minerState"]);
  const minerProgressAccount = getAccountClient(program as any, ["minerProgress"]);

  const cfg: any = await configAccount.fetch(configPda);
  const progression: any = await progressionAccount.fetch(progressionPda);

  const nextMinerId = BigInt(cfg.nextMinerId.toString());
  if (nextMinerId === 0n) {
    throw new Error("No miner exists yet. Run lootbox_miner_flow.ts first.");
  }

  const latestMinerId = nextMinerId - 1n;

  const [minerStatePda] = PublicKey.findProgramAddressSync(
    [SEED_MINER, owner.toBuffer(), u64Le(latestMinerId)],
    program.programId
  );

  const [minerProgressPda] = PublicKey.findProgramAddressSync(
    [SEED_MINER_PROGRESS, minerStatePda.toBuffer()],
    program.programId
  );

  const userAta = await getOrCreateAssociatedTokenAccount(
    provider.connection,
    payer,
    essMint,
    owner
  );

  const recipientAta = await getOrCreateAssociatedTokenAccount(
    provider.connection,
    payer,
    essMint,
    RECIPIENT_WALLET,
    true
  );

  const beforeMiner: any = await minerStateAccount.fetch(minerStatePda);
  const beforeProgress0: any = await minerProgressAccount.fetch(minerProgressPda);

  const rarity = Number(beforeMiner.rarity);
  const level = Number(beforeProgress0.level);

  const maxLevel = Number(progression.maxLevelByRarity[rarity]);
  if (level >= maxLevel) {
    console.log("\n[SKIP] miner already at max level");
    console.log({
      minerId: latestMinerId.toString(),
      rarity,
      level,
      maxLevel,
    });
    return;
  }

  const baseExp = Number(progression.expBaseByRarity[rarity]);
  const growthBps = Number(progression.expGrowthBps);
  const needExp = expRequired(baseExp, growthBps, level);

  const currentExp = BigInt(beforeProgress0.exp.toString());
  const missingExp = BigInt(Math.max(0, needExp)) - currentExp;

  const beforeUserAta = await getAccount(provider.connection, userAta.address);
  const beforeRecipientAta = await getAccount(provider.connection, recipientAta.address);

  console.log("\n=== BEFORE MINER LEVEL UP ===");
  console.log({
    minerId: latestMinerId.toString(),
    minerState: minerStatePda.toBase58(),
    minerProgress: minerProgressPda.toBase58(),
    rarity,
    level,
    exp: bnLikeToString(beforeProgress0.exp),
    needExp,
    missingExp: missingExp.toString(),
    userBalance: beforeUserAta.amount.toString(),
    recipientBalance: beforeRecipientAta.amount.toString(),
  });

  if (missingExp > 0n) {
    console.log("\n[STEP] adminGrantExp");
    const sigGrant = await (program.methods as any)
      .adminGrantExp(new anchor.BN(missingExp.toString()))
      .accounts({
        admin: owner,
        config: configPda,
        progression: progressionPda,
        minerState: minerStatePda,
        minerProgress: minerProgressPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("adminGrantExp sig:", sigGrant);
  } else {
    console.log("\n[SKIP] adminGrantExp not needed");
  }

  const beforeLevelUpProgress: any = await minerProgressAccount.fetch(minerProgressPda);

  console.log("\n=== READY FOR LEVEL UP ===");
  console.log({
    level: bnLikeToString(beforeLevelUpProgress.level),
    exp: bnLikeToString(beforeLevelUpProgress.exp),
    needExp,
  });

  console.log("\n[STEP] minerLevelUp");
  const sigLevel = await (program.methods as any)
    .minerLevelUp()
    .accounts({
      owner,
      minerState: minerStatePda,
      progression: progressionPda,
      minerProgress: minerProgressPda,
      economy: economyPda,
      essMint,
      userAta: userAta.address,
      recipientWallet: RECIPIENT_WALLET,
      recipientAta: recipientAta.address,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .rpc();

  console.log("minerLevelUp sig:", sigLevel);

  const afterProgress: any = await minerProgressAccount.fetch(minerProgressPda);
  const afterUserAta = await getAccount(provider.connection, userAta.address);
  const afterRecipientAta = await getAccount(provider.connection, recipientAta.address);

  console.log("\n=== AFTER MINER LEVEL UP ===");
  console.log({
    level: bnLikeToString(afterProgress.level),
    exp: bnLikeToString(afterProgress.exp),
    userBalance: afterUserAta.amount.toString(),
    recipientBalance: afterRecipientAta.amount.toString(),
    userSpent: (beforeUserAta.amount - afterUserAta.amount).toString(),
    recipientGained: (afterRecipientAta.amount - beforeRecipientAta.amount).toString(),
  });

  console.log("✅ miner_level_up completed");
}

main().catch((e) => {
  console.error("FATAL miner_level_up:", e);
  process.exit(1);
});