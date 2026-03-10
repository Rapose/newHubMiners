import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import fs from "node:fs";

export const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);

export function resolveProgram() {
  const idl = JSON.parse(
    fs.readFileSync("target/idl/miners.json", "utf8")
  );

  const programId = new PublicKey(idl.address);
  return new anchor.Program(idl, programId, provider);
}

export async function tryStep(name: string, fn: () => Promise<void>) {
  try {
    console.log(`\n[STEP] ${name}`);
    await fn();
    console.log(`[OK] ${name}`);
  } catch (e: any) {
    console.log(`[WARN] ${name}:`, e?.message ?? e);
  }
}

export function loadMint(): PublicKey {
  if (!fs.existsSync(".ess_mint.tmp")) {
    throw new Error("ESS mint not found. Run create_fixed_mint.ts first.");
  }

  const mint = fs.readFileSync(".ess_mint.tmp", "utf8").trim();
  return new PublicKey(mint);
}

export function log(title: string, obj: any) {
  console.log("\n===", title, "===");
  console.dir(obj, { depth: null });
}