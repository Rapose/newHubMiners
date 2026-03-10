import * as anchor from "@coral-xyz/anchor";
import { Keypair, PublicKey } from "@solana/web3.js";
import { logStep, SuiteCtx } from "./_shared";
import { runMarketplaceMinerTests } from "./marketplace_miner";
import { runMarketplaceLandTests } from "./marketplace_land";
import { runMarketplaceEquipmentTests } from "./marketplace_equipment";
import { runMarketplaceLockTests } from "./marketplace_locks";
import { runEquipmentFlow } from "./equipment_flow";
import { runGlobalMiningFlow } from "./global_mining_flow";
import { runMinerLevelUpFlow } from "./miner_level_up";

function loadProgram(): anchor.Program {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const idl = require("../target/idl/moe_anchor_v1.json");
  return new anchor.Program(idl, provider);
}

function dummyCtx(program: anchor.Program): SuiteCtx {
  const provider = anchor.getProvider() as anchor.AnchorProvider;
  return {
    provider,
    program,
    admin: Keypair.generate(),
    seller: Keypair.generate(),
    buyer: Keypair.generate(),
    essMint: new PublicKey("11111111111111111111111111111111"),
    recipientWallet: Keypair.generate().publicKey,
  };
}

async function main() {
  logStep("Marketplace suite bootstrap");
  const program = loadProgram();
  const ctx = dummyCtx(program);

  // Substitua esses placeholders pelas contas reais do seu fixture local.
  const a: Record<string, PublicKey> = {
    config: Keypair.generate().publicKey,
    economy: Keypair.generate().publicKey,
    progression: Keypair.generate().publicKey,
    miner: Keypair.generate().publicKey,
    minerUnlocked: Keypair.generate().publicKey,
    minerProgress: Keypair.generate().publicKey,
    minerMining: Keypair.generate().publicKey,
    equipment: Keypair.generate().publicKey,
    land: Keypair.generate().publicKey,
    landListed: Keypair.generate().publicKey,
    sellerInventory: Keypair.generate().publicKey,
    buyerInventory: Keypair.generate().publicKey,
    listing: Keypair.generate().publicKey,
    listing2: Keypair.generate().publicKey,
    sellerAta: Keypair.generate().publicKey,
    buyerAta: Keypair.generate().publicKey,
    recipientAta: Keypair.generate().publicKey,
  };

  await runMarketplaceMinerTests(ctx, a);
  await runMarketplaceLandTests(ctx, a);
  await runMarketplaceEquipmentTests(ctx, a);
  await runMarketplaceLockTests(ctx, a);

  // compat: fluxos já existentes no projeto
  await runEquipmentFlow(ctx);
  await runGlobalMiningFlow(ctx);
  await runMinerLevelUpFlow(ctx);

  logStep("Marketplace suite finished");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
