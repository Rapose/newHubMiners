import * as anchor from "@coral-xyz/anchor";
import { Keypair, LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";
import {
  AuthorityType,
  TOKEN_PROGRAM_ID,
  createMint,
  getAccount,
  getMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  setAuthority,
} from "@solana/spl-token";
import fs from "node:fs";

const DECIMALS = 8;

// 1_000 ESS com 8 decimais
const INITIAL_SUPPLY = 1_000n * 10n ** 8n;

const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);

const payer = (provider.wallet as any).payer as Keypair;
const owner = provider.wallet.publicKey;

async function ensureOwnerFunded() {
  const bal = await provider.connection.getBalance(owner, "confirmed");
  if (bal < 1_000_000_000) {
    console.log("Airdropping SOL to owner:", owner.toBase58());
    const sig = await provider.connection.requestAirdrop(owner, 2 * LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(sig, "confirmed");
  }
}

async function main() {
  await ensureOwnerFunded();

  console.log("Create fixed ESS mint");
  console.log("payer:", owner.toBase58());

  const payerBalance = await provider.connection.getBalance(owner, "confirmed");
  console.log("payer balance:", payerBalance);

  // 1) criar mint com authority inicial = owner
  const mintPk = await createMint(
    provider.connection,
    payer,
    owner,
    null,
    DECIMALS,
    undefined,
    undefined,
    TOKEN_PROGRAM_ID
  );

  console.log("ESS mint created:", mintPk.toBase58());

  // 2) criar ATA do owner
  const ownerAta = await getOrCreateAssociatedTokenAccount(
    provider.connection,
    payer,
    mintPk,
    owner
  );

  console.log("ATA:", ownerAta.address.toBase58());

  // 3) mintar supply inicial
  const mintSig = await mintTo(
    provider.connection,
    payer,
    mintPk,
    ownerAta.address,
    payer,
    Number(INITIAL_SUPPLY),
    [],
    undefined,
    TOKEN_PROGRAM_ID
  );

  console.log("mintTo sig:", mintSig);

  // 4) revogar mint authority => token vira fixed
  const revokeSig = await setAuthority(
    provider.connection,
    payer,
    mintPk,
    owner,
    AuthorityType.MintTokens,
    null,
    [],
    undefined,
    TOKEN_PROGRAM_ID
  );

  console.log("revoke mint authority sig:", revokeSig);

  // 5) debug final
  const mintInfo = await getMint(provider.connection, mintPk);
  const ownerAcc = await getAccount(provider.connection, ownerAta.address);

  console.log("\n=== FIXED MINT SUMMARY ===");
  console.log({
    mint: mintPk.toBase58(),
    owner: owner.toBase58(),
    ownerAta: ownerAta.address.toBase58(),
    decimals: mintInfo.decimals,
    supply: mintInfo.supply.toString(),
    mintAuthority: mintInfo.mintAuthority
      ? mintInfo.mintAuthority.toBase58()
      : null,
    ownerBalance: ownerAcc.amount.toString(),
  });

  fs.writeFileSync(".ess_mint.tmp", mintPk.toBase58(), "utf8");
  console.log("\nSaved mint to .ess_mint.tmp");
}

main().catch((e) => {
  console.error("FATAL create_fixed_mint:", e);
  process.exit(1);
});