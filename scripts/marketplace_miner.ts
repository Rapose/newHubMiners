import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { SystemProgram } from "@solana/web3.js";
import { assertEq, bn, expectTxFail } from "./helpers/test_env";
import { buildMarketplaceFixture, fetchListing, listingPda, tokenBalance } from "./helpers/marketplace_fixture";

export async function runMarketplaceMinerFlow() {
  const fx = await buildMarketplaceFixture();
  const { program, provider } = fx.env;

  const cfg: any = await program.account.config.fetch(fx.config);
  const listingId = Number(cfg.nextListingId);
  const listing = listingPda(program.programId, listingId);

  await program.methods.marketplaceCreateMinerListing(bn(1_000_000)).accounts({
    seller: fx.seller.publicKey,
    config: fx.config,
    miner: fx.miner,
    listing,
    systemProgram: SystemProgram.programId,
  }).signers([fx.seller]).rpc();

  let miner: any = await program.account.minerState.fetch(fx.miner);
  assertEq("miner listed after create", miner.listed, true);

  await expectTxFail("listed miner cannot claim exp", async () => {
    await program.methods.claimMiningExp().accounts({
      owner: fx.seller.publicKey,
      minerState: fx.miner,
      progression: fx.progression,
      minerProgress: fx.minerProgress,
      systemProgram: SystemProgram.programId,
    }).signers([fx.seller]).rpc();
  });

  await program.methods.marketplaceCancelMinerListing().accounts({
    seller: fx.seller.publicKey,
    listing,
    miner: fx.miner,
  }).signers([fx.seller]).rpc();

  miner = await program.account.minerState.fetch(fx.miner);
  assertEq("miner listed after cancel", miner.listed, false);

  const cfg2: any = await program.account.config.fetch(fx.config);
  const listing2 = listingPda(program.programId, Number(cfg2.nextListingId));
  await program.methods.marketplaceCreateMinerListing(bn(1_000_000)).accounts({
    seller: fx.seller.publicKey,
    config: fx.config,
    miner: fx.miner,
    listing: listing2,
    systemProgram: SystemProgram.programId,
  }).signers([fx.seller]).rpc();

  await expectTxFail("self purchase blocked", async () => {
    await program.methods.marketplaceBuyMinerListing().accounts({
      buyer: fx.seller.publicKey,
      listing: listing2,
      miner: fx.miner,
      minerProgress: fx.minerProgress,
      minerMining: fx.minerMining,
      equipment: fx.equipment,
      seller: fx.seller.publicKey,
      economy: fx.economy,
      essMint: fx.essMint,
      buyerAta: fx.sellerAta,
      sellerAta: fx.sellerAta,
      recipientAta: fx.recipientAta,
      tokenProgram: TOKEN_PROGRAM_ID,
    }).signers([fx.seller]).rpc();
  });

  const bBefore = await tokenBalance(provider.connection, fx.buyerAta);
  const sBefore = await tokenBalance(provider.connection, fx.sellerAta);
  const rBefore = await tokenBalance(provider.connection, fx.recipientAta);

  await program.methods.marketplaceBuyMinerListing().accounts({
    buyer: fx.buyer.publicKey,
    listing: listing2,
    miner: fx.miner,
    minerProgress: fx.minerProgress,
    minerMining: fx.minerMining,
    equipment: fx.equipment,
    seller: fx.seller.publicKey,
    economy: fx.economy,
    essMint: fx.essMint,
    buyerAta: fx.buyerAta,
    sellerAta: fx.sellerAta,
    recipientAta: fx.recipientAta,
    tokenProgram: TOKEN_PROGRAM_ID,
  }).signers([fx.buyer]).rpc();

  const fee = 50_000;
  const price = 1_000_000;
  const payout = price - fee;

  const bAfter = await tokenBalance(provider.connection, fx.buyerAta);
  const sAfter = await tokenBalance(provider.connection, fx.sellerAta);
  const rAfter = await tokenBalance(provider.connection, fx.recipientAta);

  assertEq("buyer paid price", bBefore - bAfter, price);
  assertEq("seller received payout", sAfter - sBefore, payout);
  assertEq("recipient received fee", rAfter - rBefore, fee);

  miner = await program.account.minerState.fetch(fx.miner);
  const progress: any = await program.account.minerProgress.fetch(fx.minerProgress);
  const mining: any = await program.account.minerMiningState.fetch(fx.minerMining);
  const equipment: any = await program.account.equipmentState.fetch(fx.equipment);
  const listingState = await fetchListing(program, listing2);

  assertEq("miner owner transferred", miner.owner.toBase58(), fx.buyer.publicKey.toBase58());
  assertEq("progress owner transferred", progress.owner.toBase58(), fx.buyer.publicKey.toBase58());
  assertEq("mining owner transferred", mining.owner.toBase58(), fx.buyer.publicKey.toBase58());
  assertEq("equipment owner transferred", equipment.owner.toBase58(), fx.buyer.publicKey.toBase58());
  assertEq("listing closed", listingState.active, false);

  console.log("✅ marketplace miner flow passed");
}
