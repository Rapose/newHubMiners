import * as anchor from "@coral-xyz/anchor";
import { PublicKey, SystemProgram, LAMPORTS_PER_SOL } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAccount,
  getMint,
  getOrCreateAssociatedTokenAccount,
} from "@solana/spl-token";
import fs from "node:fs";

const SEED_ECONOMY = Buffer.from("economy_v4");
const SEED_REWARDS_AUTH = Buffer.from("rewards_auth");

const SPEND_AMOUNT = new anchor.BN(100_000_000); // 1 ESS

const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);


function bnFieldsToString(obj: any) {
  if (!obj) return obj;
  const out: any = {};
  for (const [k, v] of Object.entries(obj)) {
    out[k] = typeof (v as any)?.toString === "function" ? (v as any).toString() : v;
  }
  return out;
}

function resolveIdlPath(): string {
  const candidates = [
    "target/idl/miners.json",
    "target/idl/moe_anchor_v1.json",
  ];

  for (const p of candidates) {
    if (fs.existsSync(p)) return p;
  }

  throw new Error(
    "IDL not found. Run `anchor build` and check target/idl/*.json"
  );
}

const idlPath = resolveIdlPath();
console.log("Using IDL:", idlPath);

const idl = JSON.parse(fs.readFileSync(idlPath, "utf8"));
const program = new anchor.Program(idl, provider) as anchor.Program;

const payer = (provider.wallet as any).payer;
const owner = provider.wallet.publicKey;

const RECIPIENT_WALLET = new PublicKey(
  "Ea9pUYYtCF6usjYAd2RdqeZ2WJETSwPaPwrGmfQRktXf"
);

async function tryStep(name: string, fn: () => Promise<void>) {
  try {
    console.log(`\n[STEP] ${name}`);
    await fn();
    console.log(`[OK] ${name}`);
  } catch (e: any) {
    console.log(`[WARN] ${name}:`, e?.message ?? e);
  }
}

function getAccountClient(program: any, names: string[]) {
  for (const n of names) {
    if (program.account?.[n]) return program.account[n];
  }
  throw new Error(`Conta não encontrada. Tentadas: ${names.join(", ")}`);
}

async function ensureOwnerFunded() {
  const bal = await provider.connection.getBalance(owner, "confirmed");
  if (bal < 1_000_000_000) {
    console.log("Airdropping SOL to owner:", owner.toBase58());
    const sig = await provider.connection.requestAirdrop(owner, 2 * LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(sig, "confirmed");
  }
}

async function resolveMint(): Promise<PublicKey> {
  if (fs.existsSync(".ess_mint.tmp")) {
    const mintStr = fs.readFileSync(".ess_mint.tmp", "utf8").trim();
    const mintPk = new PublicKey(mintStr);
    const info = await provider.connection.getAccountInfo(mintPk);
    if (info) {
      console.log("Using mint from .ess_mint.tmp:", mintPk.toBase58());
      return mintPk;
    }
  }

  throw new Error("ESS mint not found. Run create_fixed_mint.ts first.");
}

async function economyExists(economyPda: PublicKey): Promise<boolean> {
  const info = await provider.connection.getAccountInfo(economyPda, "confirmed");
  return !!info;
}

async function main() {
  await ensureOwnerFunded();

  const essMint = await resolveMint();

  console.log("=== economy_flow.ts ===");
  console.log("program:", program.programId.toBase58());
  console.log("owner:", owner.toBase58());

  const [economyPda] = PublicKey.findProgramAddressSync(
    [SEED_ECONOMY],
    program.programId
  );
  const [rewardsAuthority] = PublicKey.findProgramAddressSync(
    [SEED_REWARDS_AUTH],
    program.programId
  );

  const userAta = await getOrCreateAssociatedTokenAccount(
    provider.connection,
    payer,
    essMint,
    owner
  );

  const userBefore = await getAccount(provider.connection, userAta.address);
  console.log("user balance before:", userBefore.amount.toString());

  if (userBefore.amount < BigInt(SPEND_AMOUNT.toString())) {
    console.log("INFO: user has insufficient ESS for spendBuy; step will be skipped");
  }

  const recipientAta = await getOrCreateAssociatedTokenAccount(
    provider.connection,
    payer,
    essMint,
    RECIPIENT_WALLET,
    true
  );

  const rewardsVault = await getOrCreateAssociatedTokenAccount(
    provider.connection,
    payer,
    essMint,
    rewardsAuthority,
    true
  );

  const ecoExists = await economyExists(economyPda);

  if (!ecoExists) {
    await tryStep("economyInit", async () => {
      const sig = await program.methods
        .economyInit()
        .accounts({
          admin: owner,
          essMint,
          recipientWallet: RECIPIENT_WALLET,
          economy: economyPda,
          rewardsAuthority,
          rewardsVault: rewardsVault.address,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      console.log("economyInit sig:", sig);
    });
  } else {
    console.log("\n[STEP] economyInit");
    console.log("[SKIP] economy already exists:", economyPda.toBase58());
  }

  await tryStep("economySetMint", async () => {
    const sig = await program.methods
      .economySetMint()
      .accounts({
        admin: owner,
        essMint,
        economy: economyPda,
      })
      .rpc();
    console.log("economySetMint sig:", sig);
  });

  await tryStep("economySetRecipient", async () => {
    const sig = await program.methods
      .economySetRecipient()
      .accounts({
        admin: owner,
        recipientWallet: RECIPIENT_WALLET,
        economy: economyPda,
      })
      .rpc();
    console.log("economySetRecipient sig:", sig);
  });

  await tryStep("economySetRewardsVault", async () => {
    const sig = await program.methods
      .economySetRewardsVault()
      .accounts({
        admin: owner,
        economy: economyPda,
        rewardsAuthority,
        rewardsVault: rewardsVault.address,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();
    console.log("economySetRewardsVault sig:", sig);
  });

  const userNow = await getAccount(provider.connection, userAta.address);

  if (userNow.amount >= BigInt(SPEND_AMOUNT.toString())) {
    await tryStep("spendBuy", async () => {
      const sig = await program.methods
        .spendBuy(SPEND_AMOUNT)
        .accounts({
          user: owner,
          essMint,
          userAta: userAta.address,
          recipientWallet: RECIPIENT_WALLET,
          recipientAta: recipientAta.address,
          economy: economyPda,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      console.log("spendBuy sig:", sig);
    });
  } else {
    console.log("\n[STEP] spendBuy");
    console.log(
      `[SKIP] insufficient ESS balance: have=${userNow.amount.toString()} need=${SPEND_AMOUNT.toString()}`
    );
  }

const economyAccount = getAccountClient(program as any, ["economyConfig", "economy"]);

let eco: any = null;
try {
  eco = await economyAccount.fetch(economyPda);
} catch {
  console.log("\n[INFO] economy account not found yet:", economyPda.toBase58());
}

  const mintInfo = await getMint(provider.connection, essMint);
  const userAcc = await getAccount(provider.connection, userAta.address);
  const recipientAcc = await getAccount(provider.connection, recipientAta.address);
  const rewardsAcc = await getAccount(provider.connection, rewardsVault.address);

  console.log("\n=== DEBUG SUMMARY ===");
  console.log({
    economyPda: economyPda.toBase58(),
    essMint: essMint.toBase58(),
    mintDecimals: mintInfo.decimals,
    recipientWallet: RECIPIENT_WALLET.toBase58(),
    rewardsAuthority: rewardsAuthority.toBase58(),
    rewardsVault: rewardsVault.address.toBase58(),
    userAta: userAta.address.toBase58(),
    recipientAta: recipientAta.address.toBase58(),
    userBalance: userAcc.amount.toString(),
    recipientBalance: recipientAcc.amount.toString(),
    rewardsVaultBalance: rewardsAcc.amount.toString(),
totalsBuy: bnFieldsToString(eco.totalsBuy),
totalsSend: bnFieldsToString(eco.totalsSend),
totalsRecharge: bnFieldsToString(eco.totalsRecharge),
totalsTradeFee: bnFieldsToString(eco.totalsTradeFee),
  });

  console.log("✅ economy_flow completed");
}

main().catch((e) => {
  console.error("FATAL economy_flow:", e);
  process.exit(1);
});