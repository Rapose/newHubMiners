import * as anchor from "@coral-xyz/anchor";
import { PublicKey, SystemProgram, LAMPORTS_PER_SOL } from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  createMint,
  getAccount,
  getMint,
  getOrCreateAssociatedTokenAccount,
} from "@solana/spl-token";
import fs from "node:fs";

const SEED_CONFIG = Buffer.from("config");
const SEED_PROGRESSION = Buffer.from("progression_v1");
const SEED_ECONOMY = Buffer.from("economy_v4");
const SEED_REWARDS_AUTH = Buffer.from("rewards_auth");
const SEED_GLOBAL_MINING = Buffer.from("global_mining_v2");

const SEED_MINER = Buffer.from("miner");
const SEED_MINER_PROGRESS = Buffer.from("miner_progress_v1");
const SEED_MINER_MINING = Buffer.from("miner_mining_v1");
const SEED_LAND = Buffer.from("land");

const SEED_EQUIPMENT = Buffer.from("equipment_v1");
const SEED_EQUIPMENT_INVENTORY = Buffer.from("equipment_inventory_v1");

const DEFAULT_PUBKEY = new PublicKey("11111111111111111111111111111111");

const DECIMALS = 8;
const WEEKLY_POOL = new anchor.BN(50_000_000_000);
const TICK_LEN_SEC = new anchor.BN(10);
const DEPOSIT_TO_VAULT = new anchor.BN(2_000_000_000);

function u64LE(n: anchor.BN) {
  return n.toArrayLike(Buffer, "le", 8);
}

function getAllocatedLandPk(minerState: any): PublicKey {
  return (minerState.allocatedLand ?? minerState.allocated_land ?? DEFAULT_PUBKEY) as PublicKey;
}

function getMethod(program: any, names: string[]) {
  for (const n of names) {
    if (typeof program.methods?.[n] === "function") {
      return program.methods[n].bind(program.methods);
    }
  }
  throw new Error(`Method not found. Tried: ${names.join(", ")}`);
}

function getAccountClient(program: any, names: string[]) {
  for (const n of names) {
    if (program.account?.[n]) return program.account[n];
  }
  throw new Error(`Account client not found. Tried: ${names.join(", ")}`);
}

async function accountExists(connection: anchor.web3.Connection, pk: PublicKey) {
  return (await connection.getAccountInfo(pk)) !== null;
}

async function waitSeconds(connection: anchor.web3.Connection, sec: number) {
  const start = Date.now();
  while (Date.now() - start < sec * 1000) {
    await connection.getLatestBlockhash("processed");
  }
}

async function sleep(ms: number) {
  await new Promise((resolve) => setTimeout(resolve, ms));
}

async function safeStep(name: string, fn: () => Promise<void>) {
  try {
    console.log(`\n[STEP] ${name}`);
    await fn();
    console.log(`[OK] ${name}`);
  } catch (e: any) {
    console.log(`[WARN] ${name}:`, e?.message ?? e);
  }
}

async function ensureOwnerFunded(
  connection: anchor.web3.Connection,
  owner: PublicKey
) {
  const bal = await connection.getBalance(owner, "confirmed");
  if (bal < 1_000_000_000) {
    console.log("Airdropping SOL to owner:", owner.toBase58());
    const sig = await connection.requestAirdrop(owner, 2 * LAMPORTS_PER_SOL);
    await connection.confirmTransaction(sig, "confirmed");
  }
}

async function resolveMint(
  connection: anchor.web3.Connection,
  payer: any
): Promise<PublicKey> {
  if (fs.existsSync(".ess_mint.tmp")) {
    const mintStr = fs.readFileSync(".ess_mint.tmp", "utf8").trim();
    const mintPk = new PublicKey(mintStr);
    const info = await connection.getAccountInfo(mintPk);
    if (info) {
      console.log("Using mint from .ess_mint.tmp:", mintPk.toBase58());
      return mintPk;
    }
  }

  console.log("ESS mint not found on disk. Creating fallback test mint.");
  const mint = await createMint(connection, payer, payer.publicKey, payer.publicKey, DECIMALS);
  console.log("Fallback mint:", mint.toBase58());
  return mint;
}

async function fetchTxWithRetry(
  connection: anchor.web3.Connection,
  signature: string,
  tries = 8,
  waitMs = 700
) {
  for (let i = 0; i < tries; i++) {
    const tx = await connection.getTransaction(signature, {
      commitment: "confirmed",
      maxSupportedTransactionVersion: 0,
    });
    if (tx) return tx;
    await sleep(waitMs);
  }
  return null;
}

async function printFilteredProgramLogs(
  connection: anchor.web3.Connection,
  signature: string,
  title: string,
  patterns: string[]
) {
  const tx = await fetchTxWithRetry(connection, signature);

  console.log(`\n=== ${title} ===`);
  console.log("signature:", signature);

  if (!tx?.meta?.logMessages?.length) {
    console.log("No program logs found.");
    return;
  }

  let found = false;
  for (const line of tx.meta.logMessages) {
    if (patterns.some((p) => line.includes(p))) {
      console.log(line);
      found = true;
    }
  }

  if (!found) {
    console.log("No filtered logs matched:", patterns.join(", "));
    console.log("--- All log messages ---");
    for (const line of tx.meta.logMessages) {
      console.log(line);
    }
  }
}

function levelBonusBps(level: number) {
  return level === 0 ? 0 : level * 300;
}

function applyBpsFloor(x: number, bps: number) {
  return Math.floor((x * bps) / 10_000);
}

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.MoeAnchorV1 as anchor.Program;
  const owner = provider.wallet.publicKey as PublicKey;
  const payer = (provider.wallet as any).payer;
  const connection = provider.connection;

  console.log("=== global_mining_flow.ts ===");
  console.log("program:", program.programId.toBase58());
  console.log("owner:", owner.toBase58());
  console.log("payer:", payer.publicKey.toBase58());

  await ensureOwnerFunded(connection, owner);

  const [configPda] = PublicKey.findProgramAddressSync([SEED_CONFIG], program.programId);
  const [progressionPda] = PublicKey.findProgramAddressSync([SEED_PROGRESSION], program.programId);
  const [economyPda] = PublicKey.findProgramAddressSync([SEED_ECONOMY], program.programId);
  const [rewardsAuthority] = PublicKey.findProgramAddressSync([SEED_REWARDS_AUTH], program.programId);
  const [globalPda] = PublicKey.findProgramAddressSync([SEED_GLOBAL_MINING], program.programId);

  console.log("Base PDAs:", {
    configPda: configPda.toBase58(),
    progressionPda: progressionPda.toBase58(),
    economyPda: economyPda.toBase58(),
    rewardsAuthority: rewardsAuthority.toBase58(),
    globalPda: globalPda.toBase58(),
  });

  const configAccount = getAccountClient(program as any, ["config"]);
  const economyAccount = getAccountClient(program as any, ["economyConfig", "economy"]);
  const minerProgressAccount = getAccountClient(program as any, ["minerProgress"]);
  const minerMiningAccount = getAccountClient(program as any, ["minerMiningState", "minerMining"]);
  const minerStateAccount = getAccountClient(program as any, ["minerState"]);
  const equipmentAccount = getAccountClient(program as any, ["equipmentState", "equipment"]);
  const landStateAccount = getAccountClient(program as any, ["landState"]);

  await safeStep("initializeConfig (idempotent)", async () => {
    if (await accountExists(connection, configPda)) return;

    await program.methods
      .initializeConfig()
      .accounts({
        admin: owner,
        config: configPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
  });

  await safeStep("setPaused(false) when owner is admin", async () => {
    if (!(await accountExists(connection, configPda))) return;
    const cfg = await configAccount.fetch(configPda);
    if (cfg.admin.equals(owner) && cfg.paused) {
      await program.methods
        .setPaused(false)
        .accounts({
          admin: owner,
          config: configPda,
        })
        .rpc();
    }
  });

  await safeStep("progressionInit (idempotent)", async () => {
    if (await accountExists(connection, progressionPda)) return;

    await program.methods
      .progressionInit()
      .accounts({
        admin: owner,
        progression: progressionPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
  });

  let essMint: PublicKey;
  if (await accountExists(connection, economyPda)) {
    const eco0 = await economyAccount.fetch(economyPda);
    essMint = eco0.essMint as PublicKey;
    console.log("Economy already exists. Using essMint:", essMint.toBase58());
  } else {
    essMint = await resolveMint(connection, payer);
  }

  const rewardsVaultAta = await getOrCreateAssociatedTokenAccount(
    connection,
    payer,
    essMint,
    rewardsAuthority,
    true
  );

  await safeStep("economyInit (idempotent)", async () => {
    if (await accountExists(connection, economyPda)) return;

    await program.methods
      .economyInit()
      .accounts({
        admin: owner,
        essMint,
        recipientWallet: owner,
        economy: economyPda,
        rewardsAuthority,
        rewardsVault: rewardsVaultAta.address,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
  });

  const eco = await economyAccount.fetch(economyPda);
  essMint = eco.essMint as PublicKey;
  const recipientWallet = eco.recipientWallet as PublicKey;
  const rewardsVault = eco.rewardsVault as PublicKey;

  const userAta = await getOrCreateAssociatedTokenAccount(connection, payer, essMint, owner);
  const recipientAta = await getOrCreateAssociatedTokenAccount(connection, payer, essMint, recipientWallet, true);

  await safeStep("log mintAuthority", async () => {
    const mi = await getMint(connection, essMint);
    console.log("mintAuthority:", mi.mintAuthority?.toBase58() ?? null);
  });

  await safeStep("rewardsDeposit (best effort)", async () => {
    const userTokenBefore = await getAccount(connection, userAta.address);
    console.log("user ESS before deposit:", userTokenBefore.amount.toString());

    if (userTokenBefore.amount < BigInt(DEPOSIT_TO_VAULT.toString())) {
      console.log("Skipping rewardsDeposit: insufficient user ESS for fixed mint scenario");
      return;
    }

    await program.methods
      .rewardsDeposit(DEPOSIT_TO_VAULT)
      .accounts({
        depositor: owner,
        essMint,
        depositorAta: userAta.address,
        economy: economyPda,
        rewardsAuthority,
        rewardsVault,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();
  });

  let globalReady = false;

  await safeStep("globalMiningInit (idempotent)", async () => {
    if (await accountExists(connection, globalPda)) {
      globalReady = true;
      return;
    }

    const initGlobal = getMethod(program as any, ["globalMiningInit"]);
    await initGlobal(TICK_LEN_SEC, WEEKLY_POOL)
      .accounts({
        admin: owner,
        config: configPda,
        global: globalPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    globalReady = true;
  });

  if (!globalReady && !(await accountExists(connection, globalPda))) {
    throw new Error("globalMiningInit failed; aborting remaining global mining steps");
  }

  const globalAccount = getAccountClient(program as any, ["globalMiningState", "globalMining"]);
  let globalState = await globalAccount.fetch(globalPda);

  console.log("\n=== GLOBAL BEFORE RUN ===");
  console.log({
    weekIndex: globalState.weekIndex?.toString?.() ?? globalState.weekIndex,
    weekStartTs: globalState.weekStartTs?.toString?.() ?? globalState.weekStartTs,
    tickLenSec: globalState.tickLenSec?.toString?.() ?? globalState.tickLenSec,
    weeklyPoolAmount: globalState.weeklyPoolAmount?.toString?.() ?? globalState.weeklyPoolAmount,
    totalEpTw: globalState.totalEpTw?.toString?.() ?? globalState.totalEpTw,
    frozen: globalState.frozen,
    frozenWeekIndex: globalState.frozenWeekIndex?.toString?.() ?? globalState.frozenWeekIndex,
    frozenWeeklyPoolAmount:
      globalState.frozenWeeklyPoolAmount?.toString?.() ?? globalState.frozenWeeklyPoolAmount,
    frozenTotalEpTw: globalState.frozenTotalEpTw?.toString?.() ?? globalState.frozenTotalEpTw,
  });

  await safeStep("globalMiningRolloverWeek if frozen", async () => {
    globalState = await globalAccount.fetch(globalPda);

    if (!globalState.frozen) {
      console.log("Global already active; no rollover needed");
      return;
    }

    const rolloverWeek = getMethod(program as any, [
      "globalMiningRolloverWeek",
      "rolloverWeek",
    ]);

    const sigRollover = await rolloverWeek(WEEKLY_POOL)
      .accounts({
        admin: owner,
        global: globalPda,
      })
      .rpc();

    console.log("globalMiningRolloverWeek sig:", sigRollover);

    await printFilteredProgramLogs(
      connection,
      sigRollover,
      "ROLLOVER PROGRAM LOGS",
      ["GM_ROLLOVER"]
    );

    const after = await globalAccount.fetch(globalPda);
    console.log("Global rolled over:", {
      weekIndex: after.weekIndex?.toString?.() ?? after.weekIndex,
      weekStartTs: after.weekStartTs?.toString?.() ?? after.weekStartTs,
      frozen: after.frozen,
      totalEpTw: after.totalEpTw?.toString?.() ?? after.totalEpTw,
    });
  });

  const cfg0 = await configAccount.fetch(configPda);
  const nextMinerId = BigInt(cfg0.nextMinerId.toString());

  if (nextMinerId === 0n) {
    throw new Error("No miner exists yet. Run lootbox_miner_flow.ts first.");
  }

  const latestMinerId = nextMinerId - 1n;
  console.log("latestMinerId:", latestMinerId.toString());

  const [minerPda] = PublicKey.findProgramAddressSync(
    [SEED_MINER, owner.toBuffer(), u64LE(new anchor.BN(latestMinerId.toString()))],
    program.programId
  );

  console.log("minerPda:", minerPda.toBase58());

  const [minerProgressPda] = PublicKey.findProgramAddressSync(
    [SEED_MINER_PROGRESS, minerPda.toBuffer()],
    program.programId
  );
  const [minerMiningPda] = PublicKey.findProgramAddressSync(
    [SEED_MINER_MINING, minerPda.toBuffer()],
    program.programId
  );
  const [equipmentPda] = PublicKey.findProgramAddressSync(
    [SEED_EQUIPMENT, minerPda.toBuffer()],
    program.programId
  );

  await safeStep("adminGrantExp (init minerProgress)", async () => {
    if (await accountExists(connection, minerProgressPda)) return;

    await program.methods
      .adminGrantExp(new anchor.BN(10))
      .accounts({
        admin: owner,
        config: configPda,
        progression: progressionPda,
        minerState: minerPda,
        minerProgress: minerProgressPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
  });

  await safeStep("globalMiningRegisterMiner (idempotent)", async () => {
    if (await accountExists(connection, minerMiningPda)) return;

    const registerMiner = getMethod(program as any, ["globalMiningRegisterMiner"]);
    const sigRegister = await registerMiner()
      .accounts({
        owner,
        global: globalPda,
        minerState: minerPda,
        minerMining: minerMiningPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("globalMiningRegisterMiner sig:", sigRegister);

    await printFilteredProgramLogs(
      connection,
      sigRegister,
      "REGISTER PROGRAM LOGS",
      ["GM_REGISTER"]
    );
  });

  const cfg1 = await configAccount.fetch(configPda);
  const landIdToCreate = new anchor.BN(cfg1.nextLandId.toString());
  const [landPda] = PublicKey.findProgramAddressSync(
    [SEED_LAND, owner.toBuffer(), u64LE(landIdToCreate)],
    program.programId
  );

  console.log("landIdToCreate:", landIdToCreate.toString());
  console.log("landPda:", landPda.toBase58());

  await safeStep("createLandDebug (idempotent-ish)", async () => {
    if (await accountExists(connection, landPda)) return;

    await program.methods
      .createLandDebug(1, 2, 3)
      .accounts({
        owner,
        config: configPda,
        landState: landPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
  });

  await safeStep("globalMiningAssignLand (required)", async () => {
    const minerStateBefore = await minerStateAccount.fetch(minerPda);
    const allocBefore = getAllocatedLandPk(minerStateBefore);
    console.log("allocated before:", allocBefore.toBase58());

    if (!allocBefore.equals(DEFAULT_PUBKEY)) {
      console.log("Miner already allocated:", allocBefore.toBase58());
      return;
    }

    const assignLand = getMethod(program as any, [
      "globalMiningAssignLand",
      "assignLand",
      "allocateMinerToLand",
      "minerAllocateLand",
    ]);

    const sigAssign = await assignLand()
      .accounts({
        owner,
        minerState: minerPda,
        landState: landPda,
      })
      .rpc();

    console.log("globalMiningAssignLand sig:", sigAssign);

    await printFilteredProgramLogs(
      connection,
      sigAssign,
      "ASSIGN PROGRAM LOGS",
      ["GM_ASSIGN"]
    );

    const minerStateAfter = await minerStateAccount.fetch(minerPda);
    const allocAfter = getAllocatedLandPk(minerStateAfter);
    console.log("allocated after:", allocAfter.toBase58());
    console.log("expected landPda:", landPda.toBase58());

    if (!allocAfter.equals(landPda)) {
      throw new Error(
        `Assign did not persist: expected=${landPda.toBase58()} got=${allocAfter.toBase58()}`
      );
    }
  });

  await safeStep("wait 12 seconds to accrue EP", async () => {
    await waitSeconds(connection, 12);
  });

  const minerStateNow = await minerStateAccount.fetch(minerPda);
  const allocNow = getAllocatedLandPk(minerStateNow);
  const withLand = !allocNow.equals(DEFAULT_PUBKEY);

  await safeStep(`globalMiningUpdate (${withLand ? "with" : "without"} landState)`, async () => {
    const updateMethod = getMethod(program as any, ["globalMiningUpdate"]);

    const accounts: any = {
      owner,
      global: globalPda,
      minerState: minerPda,
      minerProgress: minerProgressPda,
      equipment: equipmentPda,
      minerMining: minerMiningPda,
    };
    if (withLand) accounts.landState = landPda;

    const sigUpdate = await updateMethod()
      .accounts(accounts)
      .rpc();

    console.log("globalMiningUpdate sig:", sigUpdate);

    await printFilteredProgramLogs(
      connection,
      sigUpdate,
      "GLOBAL MINING PROGRAM LOGS",
      ["GM_UPDATE", "GM_EQUIPMENT", "GM_PROOF"]
    );
  });

  await safeStep("globalMiningFreezeWeek", async () => {
    const freezeWeek = getMethod(program as any, [
      "globalMiningFreezeWeek",
      "freezeWeek",
    ]);

    const sigFreeze = await freezeWeek()
      .accounts({
        admin: owner,
        global: globalPda,
      })
      .rpc();

    console.log("globalMiningFreezeWeek sig:", sigFreeze);

    await printFilteredProgramLogs(
      connection,
      sigFreeze,
      "FREEZE PROGRAM LOGS",
      ["GM_FREEZE"]
    );
  });

  await safeStep(`globalMiningClaim (${withLand ? "with" : "without"} landState)`, async () => {
    const claimMethod = getMethod(program as any, ["globalMiningClaim"]);

    const sigClaim = await claimMethod()
      .accounts({
        owner,
        global: globalPda,
        minerMining: minerMiningPda,
        minerState: minerPda,
        essMint,
        economy: economyPda,
        rewardsAuthority,
        rewardsVault,
        userAta: userAta.address,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    console.log("globalMiningClaim sig:", sigClaim);

    await printFilteredProgramLogs(
      connection,
      sigClaim,
      "CLAIM PROGRAM LOGS",
      ["GM_CLAIM"]
    );
  });

  const minerMining = await minerMiningAccount.fetch(minerMiningPda);
  const userToken = await getAccount(connection, userAta.address);
  const recipientToken = await getAccount(connection, recipientAta.address);
  const rewardsToken = await getAccount(connection, rewardsVault);

  const minerStateFinal = await minerStateAccount.fetch(minerPda);
  const progressFinal = await minerProgressAccount.fetch(minerProgressPda);
  const allocFinal = getAllocatedLandPk(minerStateFinal);
  const equipmentFinal = await equipmentAccount.fetch(equipmentPda);
  const landFinal: any = await landStateAccount.fetch(landPda);

  console.log("\n=== MINER STATE ===");
  console.log({
    owner: minerStateFinal.owner?.toBase58?.() ?? null,
    rarity: minerStateFinal.rarity,
    element: minerStateFinal.element,
    hashBase:
      minerStateFinal.hashBase?.toString?.() ??
      minerStateFinal.hashBase ??
      minerStateFinal.hash_base?.toString?.() ??
      minerStateFinal.hash_base ??
      null,
    basePower:
      minerStateFinal.basePower?.toString?.() ??
      minerStateFinal.basePower ??
      minerStateFinal.base_power?.toString?.() ??
      minerStateFinal.base_power ??
      null,
    face: minerStateFinal.face ?? null,
    helmet: minerStateFinal.helmet ?? null,
    backpack: minerStateFinal.backpack ?? null,
    jacket: minerStateFinal.jacket ?? null,
    item: minerStateFinal.item ?? null,
    background: minerStateFinal.background ?? null,
    allocatedLand: allocFinal.toBase58(),
  });

  console.log("\n=== MINER PROGRESS ===");
  console.log({
    level: progressFinal.level?.toString?.() ?? progressFinal.level,
    exp: progressFinal.exp?.toString?.() ?? progressFinal.exp ?? null,
  });

  console.log("\n=== EQUIPMENT STATE ===");
  console.log({
    owner: equipmentFinal.owner?.toBase58?.() ?? null,
    miner: equipmentFinal.miner?.toBase58?.() ?? null,
    handLevel: equipmentFinal.handLevel ?? equipmentFinal.hand_level ?? null,
    handPowerBps:
      equipmentFinal.handPowerBps?.toString?.() ??
      equipmentFinal.hand_power_bps?.toString?.() ??
      equipmentFinal.handPowerBps ??
      equipmentFinal.hand_power_bps ??
      null,
    handIsRemelted:
      equipmentFinal.handIsRemelted ?? equipmentFinal.hand_is_remelted ?? null,
    headLevel: equipmentFinal.headLevel ?? equipmentFinal.head_level ?? null,
    headRechargeDiscountBps:
      equipmentFinal.headRechargeDiscountBps?.toString?.() ??
      equipmentFinal.head_recharge_discount_bps?.toString?.() ??
      equipmentFinal.headRechargeDiscountBps ??
      equipmentFinal.head_recharge_discount_bps ??
      null,
    headIsRemelted:
      equipmentFinal.headIsRemelted ?? equipmentFinal.head_is_remelted ?? null,
  });

  const hashBase = Number(
    minerStateFinal.hashBase?.toString?.() ??
    minerStateFinal.hash_base?.toString?.() ??
    0
  );

  const minerElement = Number(minerStateFinal.element ?? 0);
  const landElement = Number(landFinal.element ?? 0);

  const handPowerBps = Number(
    equipmentFinal.handPowerBps?.toString?.() ??
    equipmentFinal.hand_power_bps?.toString?.() ??
    0
  );

  const level = Number(
    progressFinal.level?.toString?.() ??
    progressFinal.level ??
    0
  );

  const affinityBps = "see GM_UPDATE/GM_PROOF logs";
  const expectedWithHandOnly = applyBpsFloor(hashBase, 10_000 + handPowerBps);
  const expectedWithHandAndLevelOnly = applyBpsFloor(
    expectedWithHandOnly,
    10_000 + levelBonusBps(level)
  );

  console.log("\n=== MINING DIAGNOSTIC (TS SIDE) ===");
  console.log({
    hashBase,
    minerElement,
    landElement,
    handPowerBps,
    level,
    levelBonusBps: levelBonusBps(level),
    expectedWithHandOnly,
    expectedWithHandAndLevelOnly,
    affinityBpsNote: affinityBps,
    actualEpTw: Number(
      minerMining.epTw?.toString?.() ??
      minerMining.ep_tw?.toString?.() ??
      0
    ),
  });

  console.log("\n=== GLOBAL MINING SUMMARY ===");
  console.log({
    essMint: essMint.toBase58(),
    minerPda: minerPda.toBase58(),
    landPda: landPda.toBase58(),
    allocatedLand: allocFinal.toBase58(),
    rewardsVault: rewardsVault.toBase58(),
    userAta: userAta.address.toBase58(),
    recipientAta: recipientAta.address.toBase58(),
    minerMining: {
      weekIndex:
        minerMining.weekIndex?.toString?.() ??
        minerMining.weekIndex ??
        minerMining.week_id?.toString?.() ??
        minerMining.week_id ??
        null,
      lastTick:
        minerMining.lastTick?.toString?.() ??
        minerMining.lastTick ??
        minerMining.last_tick?.toString?.() ??
        minerMining.last_tick ??
        null,
      epTw: (minerMining.epTw ?? minerMining.ep_tw)?.toString?.() ?? "0",
      claimed: minerMining.claimed ?? null,
    },
    balances: {
      user: userToken.amount.toString(),
      recipient: recipientToken.amount.toString(),
      rewardsVault: rewardsToken.amount.toString(),
    },
  });

  console.log("✅ global_mining_flow completed");
}

main().catch((e) => {
  console.error("FATAL global_mining_flow:", e);
  process.exit(1);
});