import { BN } from "@coral-xyz/anchor";
import { expectFailure, logStep, SuiteCtx } from "./_shared";

export async function runMarketplaceMinerTests(ctx: SuiteCtx, a: Record<string, any>) {
  const p = ctx.program;

  logStep("miner/create success");
  await p.methods.marketplaceCreateMinerListing(new BN(1_000_000)).accounts({
    seller: ctx.seller.publicKey,
    config: a.config,
    miner: a.miner,
    listing: a.listing,
  }).rpc();

  logStep("miner/double listing fail");
  await expectFailure("miner double listing", async () => {
    await p.methods.marketplaceCreateMinerListing(new BN(1_000_000)).accounts({
      seller: ctx.seller.publicKey,
      config: a.config,
      miner: a.miner,
      listing: a.listing2,
    }).rpc();
  });

  logStep("miner/cancel and relist");
  await p.methods.marketplaceCancelMinerListing().accounts({
    seller: ctx.seller.publicKey,
    listing: a.listing,
    miner: a.miner,
  }).rpc();

  await p.methods.marketplaceCreateMinerListing(new BN(1_000_000)).accounts({
    seller: ctx.seller.publicKey,
    config: a.config,
    miner: a.miner,
    listing: a.listing2,
  }).rpc();

  logStep("miner/self purchase fail");
  await expectFailure("miner self buy", async () => {
    await p.methods.marketplaceBuyMinerListing().accounts({
      buyer: ctx.seller.publicKey,
      listing: a.listing2,
      miner: a.miner,
      minerProgress: a.minerProgress,
      minerMining: a.minerMining,
      equipment: a.equipment,
      seller: ctx.seller.publicKey,
      economy: a.economy,
      essMint: ctx.essMint,
      buyerAta: a.sellerAta,
      sellerAta: a.sellerAta,
      recipientAta: a.recipientAta,
    }).rpc();
  });

  logStep("miner/buy success");
  await p.methods.marketplaceBuyMinerListing().accounts({
    buyer: ctx.buyer.publicKey,
    listing: a.listing2,
    miner: a.miner,
    minerProgress: a.minerProgress,
    minerMining: a.minerMining,
    equipment: a.equipment,
    seller: ctx.seller.publicKey,
    economy: a.economy,
    essMint: ctx.essMint,
    buyerAta: a.buyerAta,
    sellerAta: a.sellerAta,
    recipientAta: a.recipientAta,
  }).rpc();
}
