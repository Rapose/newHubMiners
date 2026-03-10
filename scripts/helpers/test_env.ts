import * as anchor from "@coral-xyz/anchor";
import { PublicKey, Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";

export const PROGRAM_ID = new PublicKey("5LRTVxfrJmec3PWmNVAnis3ZgAYVYJEMsqJN3Xnhnq79");

export type Env = {
  provider: anchor.AnchorProvider;
  program: anchor.Program;
  admin: anchor.Wallet;
};

export function loadEnv(): Env {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const idl = require("../../target/idl/moe_anchor_v1.json");
  const program = new anchor.Program(idl, PROGRAM_ID, provider);
  return { provider, program, admin: provider.wallet };
}

export async function airdropIfNeeded(
  provider: anchor.AnchorProvider,
  wallet: PublicKey,
  minLamports = 2 * LAMPORTS_PER_SOL,
) {
  const bal = await provider.connection.getBalance(wallet);
  if (bal >= minLamports) return;
  const sig = await provider.connection.requestAirdrop(wallet, minLamports - bal + LAMPORTS_PER_SOL);
  await provider.connection.confirmTransaction(sig, "confirmed");
}

export function bn(n: number | string) {
  return new anchor.BN(n);
}

export function assertEq<T>(label: string, got: T, expected: T) {
  if (got !== expected) {
    throw new Error(`${label}: expected ${String(expected)}, got ${String(got)}`);
  }
}

export async function expectTxFail(label: string, fn: () => Promise<unknown>) {
  try {
    await fn();
    throw new Error(`${label}: tx succeeded unexpectedly`);
  } catch (e) {
    console.log(`✔ expected failure: ${label} :: ${String((e as Error).message).slice(0, 140)}`);
  }
}

export async function waitSlots(provider: anchor.AnchorProvider, n: number) {
  const start = await provider.connection.getSlot("confirmed");
  while ((await provider.connection.getSlot("confirmed")) < start + n) {
    await new Promise((r) => setTimeout(r, 350));
  }
}

export function keypairFromFileMaybe(path: string): Keypair | null {
  const fs = require("fs");
  if (!fs.existsSync(path)) return null;
  const arr = JSON.parse(fs.readFileSync(path, "utf8"));
  return Keypair.fromSecretKey(Uint8Array.from(arr));
}
