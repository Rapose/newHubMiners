import * as anchor from "@coral-xyz/anchor";
import { PublicKey, Keypair } from "@solana/web3.js";

export type SuiteCtx = {
  provider: anchor.AnchorProvider;
  program: anchor.Program;
  admin: Keypair;
  seller: Keypair;
  buyer: Keypair;
  essMint: PublicKey;
  recipientWallet: PublicKey;
};

export const seeds = {
  CONFIG: Buffer.from("config"),
  ECONOMY: Buffer.from("economy_v4"),
  GLOBAL_MINING: Buffer.from("global_mining_v2"),
  EQUIPMENT_INVENTORY: Buffer.from("equipment_inventory_v1"),
  MINER_MINING: Buffer.from("miner_mining_v1"),
  LISTING: Buffer.from("listing_v1"),
};

export const findPda = (programId: PublicKey, seedParts: (Buffer | Uint8Array)[]) =>
  PublicKey.findProgramAddressSync(seedParts, programId)[0];

export async function expectFailure(label: string, fn: () => Promise<unknown>) {
  try {
    await fn();
    throw new Error(`Expected failure but succeeded: ${label}`);
  } catch (e) {
    const msg = String((e as Error)?.message ?? e);
    console.log(`✔ expected fail: ${label} -> ${msg.slice(0, 120)}`);
  }
}

export function logStep(step: string) {
  console.log(`\n=== ${step} ===`);
}
