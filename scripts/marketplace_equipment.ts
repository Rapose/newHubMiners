import { BN } from "@coral-xyz/anchor";
import { expectFailure, logStep, SuiteCtx } from "./_shared";

export async function runMarketplaceEquipmentTests(ctx: SuiteCtx, a: Record<string, any>) {
  const p = ctx.program;

  logStep("equipment/create listing success");
  await p.methods.marketplaceCreateEquipmentListing(1, 1, 1, new BN(200_000)).accounts({
    seller: ctx.seller.publicKey,
    config: a.config,
    inventory: a.sellerInventory,
    listing: a.listing,
  }).rpc();

  logStep("equipment/invalid amount fail");
  await expectFailure("equipment invalid amount", async () => {
    await p.methods.marketplaceCreateEquipmentListing(1, 1, 0, new BN(200_000)).accounts({
      seller: ctx.seller.publicKey,
      config: a.config,
      inventory: a.sellerInventory,
      listing: a.listing2,
    }).rpc();
  });

  logStep("equipment/cancel returns bucket");
  await p.methods.marketplaceCancelEquipmentListing().accounts({
    seller: ctx.seller.publicKey,
    listing: a.listing,
    inventory: a.sellerInventory,
  }).rpc();

  logStep("equipment/relist + buy success");
  await p.methods.marketplaceCreateEquipmentListing(1, 1, 1, new BN(200_000)).accounts({
    seller: ctx.seller.publicKey,
    config: a.config,
    inventory: a.sellerInventory,
    listing: a.listing2,
  }).rpc();

  await p.methods.marketplaceBuyEquipmentListing().accounts({
    buyer: ctx.buyer.publicKey,
    listing: a.listing2,
    seller: ctx.seller.publicKey,
    buyerInventory: a.buyerInventory,
    economy: a.economy,
    essMint: ctx.essMint,
    buyerAta: a.buyerAta,
    sellerAta: a.sellerAta,
    recipientAta: a.recipientAta,
  }).rpc();
}
