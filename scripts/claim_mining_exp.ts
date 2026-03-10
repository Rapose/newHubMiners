import * as anchor from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import fs from "node:fs";

const SEED_CONFIG = Buffer.from("config");
const SEED_PROGRESSION = Buffer.from("progression_v1");
const SEED_MINER = Buffer.from("miner");
const SEED_MINER_PROGRESS = Buffer.from("miner_progress_v1");

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

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const idlPath = resolveIdlPath();
  console.log("Using IDL:", idlPath);

  const idl = JSON.parse(fs.readFileSync(idlPath, "utf8"));
  const program = new anchor.Program(idl, provider) as anchor.Program;

  const owner = provider.wallet.publicKey;

  console.log("=== claim_mining_exp.ts ===");
  console.log("program:", program.programId.toBase58());
  console.log("owner:", owner.toBase58());

  const [configPda] = PublicKey.findProgramAddressSync(
    [SEED_CONFIG],
    program.programId
  );

  const [progressionPda] = PublicKey.findProgramAddressSync(
    [SEED_PROGRESSION],
    program.programId
  );

  const configAccount = getAccountClient(program as any, ["config"]);
  const minerStateAccount = getAccountClient(program as any, ["minerState"]);
  const minerProgressAccount = getAccountClient(program as any, ["minerProgress"]);

  const cfg: any = await configAccount.fetch(configPda);

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

  const beforeMiner: any = await minerStateAccount.fetch(minerStatePda);
  const beforeProgress: any = await minerProgressAccount.fetch(minerProgressPda);

  console.log("\n=== BEFORE CLAIM MINING EXP ===");
  console.log({
    minerId: latestMinerId.toString(),
    minerState: minerStatePda.toBase58(),
    minerProgress: minerProgressPda.toBase58(),
    rarity: bnLikeToString(beforeMiner.rarity),
    level: bnLikeToString(beforeProgress.level),
    exp: bnLikeToString(beforeProgress.exp),
    lastExpClaimTs:
      bnLikeToString(beforeProgress.lastExpClaimTs) ??
      bnLikeToString(beforeProgress.last_exp_claim_ts) ??
      null,
  });

  const sig = await (program.methods as any)
    .claimMiningExp()
    .accounts({
      owner,
      minerState: minerStatePda,
      progression: progressionPda,
      minerProgress: minerProgressPda,
      systemProgram: SystemProgram.programId,
    })
    .rpc();

  console.log("\nclaimMiningExp sig:", sig);

  const afterProgress: any = await minerProgressAccount.fetch(minerProgressPda);

  console.log("\n=== AFTER CLAIM MINING EXP ===");
  console.log({
    level: bnLikeToString(afterProgress.level),
    exp: bnLikeToString(afterProgress.exp),
    lastExpClaimTs:
      bnLikeToString(afterProgress.lastExpClaimTs) ??
      bnLikeToString(afterProgress.last_exp_claim_ts) ??
      null,
  });

  console.log("✅ claim_mining_exp completed");
}

main().catch((e) => {
  console.error("FATAL claim_mining_exp:", e);
  process.exit(1);
});