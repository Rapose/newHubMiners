import * as anchor from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import fs from "node:fs";

const SEED_CONFIG = Buffer.from("config");
const SEED_MINER = Buffer.from("miner");
const SEED_EQUIPMENT = Buffer.from("equipment_v1");
const SEED_EQUIPMENT_INVENTORY = Buffer.from("equipment_inventory_v1");

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

async function accountExists(connection: anchor.web3.Connection, pk: PublicKey) {
  return (await connection.getAccountInfo(pk)) !== null;
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

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const idlPath = resolveIdlPath();
  console.log("Using IDL:", idlPath);

  const idl = JSON.parse(fs.readFileSync(idlPath, "utf8"));
  const program = new anchor.Program(idl, provider) as anchor.Program;

  const owner = provider.wallet.publicKey;
  const connection = provider.connection;

  console.log("=== equipment_flow.ts ===");
  console.log("program:", program.programId.toBase58());
  console.log("owner:", owner.toBase58());

  const [configPda] = PublicKey.findProgramAddressSync(
    [SEED_CONFIG],
    program.programId
  );

  const configAccount = getAccountClient(program as any, ["config"]);
  const equipmentAccount = getAccountClient(program as any, ["equipmentState", "equipment"]);
  const inventoryAccount = getAccountClient(program as any, ["equipmentInventoryState", "inventory"]);

  const cfg: any = await configAccount.fetch(configPda);
  const nextMinerId = BigInt(cfg.nextMinerId.toString());
  if (nextMinerId === 0n) {
    throw new Error("No miner exists yet. Run lootbox_miner_flow.ts first.");
  }

  const latestMinerId = nextMinerId - 1n;
  console.log("latestMinerId:", latestMinerId.toString());

  const [minerPda] = PublicKey.findProgramAddressSync(
    [SEED_MINER, owner.toBuffer(), u64Le(latestMinerId)],
    program.programId
  );

  const [equipmentPda] = PublicKey.findProgramAddressSync(
    [SEED_EQUIPMENT, minerPda.toBuffer()],
    program.programId
  );

  const [inventoryPda] = PublicKey.findProgramAddressSync(
    [SEED_EQUIPMENT_INVENTORY, owner.toBuffer()],
    program.programId
  );

  console.log("minerPda:", minerPda.toBase58());
  console.log("equipmentPda:", equipmentPda.toBase58());
  console.log("inventoryPda:", inventoryPda.toBase58());

  await safeStep("equipmentInventoryInit (idempotent)", async () => {
    if (await accountExists(connection, inventoryPda)) return;

    await (program.methods as any)
      .equipmentInventoryInit()
      .accounts({
        owner,
        inventory: inventoryPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
  });

  await safeStep("equipmentInit (idempotent)", async () => {
    if (await accountExists(connection, equipmentPda)) return;

    await (program.methods as any)
      .equipmentInit()
      .accounts({
        owner,
        minerState: minerPda,
        equipment: equipmentPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
  });

  await safeStep("grant hand tier 3 normal item", async () => {
    await (program.methods as any)
      .equipmentInventoryGrantItem(
        { hand: {} },
        3,
        1,
        false,
        false
      )
      .accounts({
        admin: owner,
        config: configPda,
        inventory: inventoryPda,
        owner,
      })
      .rpc();
  });

  await safeStep("grant head tier 2 remelted item", async () => {
    await (program.methods as any)
      .equipmentInventoryGrantItem(
        { head: {} },
        2,
        1,
        true,
        false
      )
      .accounts({
        admin: owner,
        config: configPda,
        inventory: inventoryPda,
        owner,
      })
      .rpc();
  });

  const inventoryBefore: any = await inventoryAccount.fetch(inventoryPda);
  const equipmentBefore: any = await equipmentAccount.fetch(equipmentPda);

  console.log("\n=== INVENTORY BEFORE EQUIP ===");
  console.log({
    newHand: inventoryBefore.newHand ?? inventoryBefore.new_hand ?? null,
    newHead: inventoryBefore.newHead ?? inventoryBefore.new_head ?? null,
    newHandRemelted: inventoryBefore.newHandRemelted ?? inventoryBefore.new_hand_remelted ?? null,
    newHeadRemelted: inventoryBefore.newHeadRemelted ?? inventoryBefore.new_head_remelted ?? null,
  });

  console.log("\n=== EQUIPMENT BEFORE EQUIP ===");
  console.log({
    handLevel: equipmentBefore.handLevel ?? equipmentBefore.hand_level ?? null,
    handPowerBps:
      bnLikeToString(equipmentBefore.handPowerBps) ??
      bnLikeToString(equipmentBefore.hand_power_bps) ??
      null,
    handIsRemelted:
      equipmentBefore.handIsRemelted ?? equipmentBefore.hand_is_remelted ?? null,
    headLevel: equipmentBefore.headLevel ?? equipmentBefore.head_level ?? null,
    headRechargeDiscountBps:
      bnLikeToString(equipmentBefore.headRechargeDiscountBps) ??
      bnLikeToString(equipmentBefore.head_recharge_discount_bps) ??
      null,
    headIsRemelted:
      equipmentBefore.headIsRemelted ?? equipmentBefore.head_is_remelted ?? null,
  });

  await safeStep("equipmentReplaceHand tier 3", async () => {
    await (program.methods as any)
      .equipmentReplaceHand(3)
      .accounts({
        owner,
        minerState: minerPda,
        equipment: equipmentPda,
        inventory: inventoryPda,
      })
      .rpc();
  });

  await safeStep("equipmentReplaceHead tier 2", async () => {
    await (program.methods as any)
      .equipmentReplaceHead(2)
      .accounts({
        owner,
        minerState: minerPda,
        equipment: equipmentPda,
        inventory: inventoryPda,
      })
      .rpc();
  });

  const inventoryAfter: any = await inventoryAccount.fetch(inventoryPda);
  const equipmentAfter: any = await equipmentAccount.fetch(equipmentPda);

  console.log("\n=== INVENTORY AFTER EQUIP ===");
  console.log({
    newHand: inventoryAfter.newHand ?? inventoryAfter.new_hand ?? null,
    newHead: inventoryAfter.newHead ?? inventoryAfter.new_head ?? null,
    newHandRemelted: inventoryAfter.newHandRemelted ?? inventoryAfter.new_hand_remelted ?? null,
    newHeadRemelted: inventoryAfter.newHeadRemelted ?? inventoryAfter.new_head_remelted ?? null,
  });

  console.log("\n=== EQUIPMENT AFTER EQUIP ===");
  console.log({
    handLevel: equipmentAfter.handLevel ?? equipmentAfter.hand_level ?? null,
    handPowerBps:
      bnLikeToString(equipmentAfter.handPowerBps) ??
      bnLikeToString(equipmentAfter.hand_power_bps) ??
      null,
    handIsRemelted:
      equipmentAfter.handIsRemelted ?? equipmentAfter.hand_is_remelted ?? null,
    headLevel: equipmentAfter.headLevel ?? equipmentAfter.head_level ?? null,
    headRechargeDiscountBps:
      bnLikeToString(equipmentAfter.headRechargeDiscountBps) ??
      bnLikeToString(equipmentAfter.head_recharge_discount_bps) ??
      null,
    headIsRemelted:
      equipmentAfter.headIsRemelted ?? equipmentAfter.head_is_remelted ?? null,
  });

  console.log("✅ equipment_flow completed");
}

main().catch((e) => {
  console.error("FATAL equipment_flow:", e);
  process.exit(1);
});