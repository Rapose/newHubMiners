import { BN } from "@coral-xyz/anchor";
import { expectFailure, logStep, SuiteCtx } from "./_shared";

export async function runMarketplaceLandTests(ctx: SuiteCtx, a: Record<string, any>) {
  const p = ctx.program;

  logStep("land/create success");
  await p.methods.marketplaceCreateLandListing(new BN(500_000)).accounts({
    seller: ctx.seller.publicKey,
    config: a.config,
    land: a.land,
    listing: a.listing,
  }).rpc();

  logStep("land/double listing fail");
  await expectFailure("land double listing", async () => {
    await p.methods.marketplaceCreateLandListing(new BN(500_000)).accounts({
      seller: ctx.seller.publicKey,
      config: a.config,
      land: a.land,
      listing: a.listing2,
    }).rpc();
  });

  logStep("land/cancel success");
  await p.methods.marketplaceCancelLandListing().accounts({
    seller: ctx.seller.publicKey,
    listing: a.listing,
    land: a.land,
  }).rpc();

  logStep("land/relist + buy");
  await p.methods.marketplaceCreateLandListing(new BN(500_000)).accounts({
    seller: ctx.seller.publicKey,
    config: a.config,
    land: a.land,
    listing: a.listing2,
  }).rpc();

  await p.methods.marketplaceBuyLandListing().accounts({
    buyer: ctx.buyer.publicKey,
    listing: a.listing2,
    land: a.land,
    seller: ctx.seller.publicKey,
    economy: a.economy,
    essMint: ctx.essMint,
    buyerAta: a.buyerAta,
    sellerAta: a.sellerAta,
    recipientAta: a.recipientAta,
  }).rpc();
}
