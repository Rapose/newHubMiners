import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { SystemProgram } from "@solana/web3.js";
import { assertEq, bn, expectTxFail } from "./helpers/test_env";
import { buildMarketplaceFixture, listingPda } from "./helpers/marketplace_fixture";

export async function runMarketplaceEquipmentFlow() {
  const fx = await buildMarketplaceFixture();
  const { program } = fx.env;

  let invSeller: any = await program.account.equipmentInventoryState.fetch(fx.sellerInventory);
  const before = Number(invSeller.newHand[1]);

  const cfg: any = await program.account.config.fetch(fx.config);
  const listing = listingPda(program.programId, Number(cfg.nextListingId));

  await program.methods.marketplaceCreateEquipmentListing(1, 1, 1, bn(200_000)).accounts({
    seller: fx.seller.publicKey,
    config: fx.config,
    inventory: fx.sellerInventory,
    listing,
    systemProgram: SystemProgram.programId,
  }).signers([fx.seller]).rpc();

  invSeller = await program.account.equipmentInventoryState.fetch(fx.sellerInventory);
  assertEq("seller bucket decremented", Number(invSeller.newHand[1]), before - 1);

  await expectTxFail("list without balance", async () => {
    const cfgx: any = await program.account.config.fetch(fx.config);
    const l2 = listingPda(program.programId, Number(cfgx.nextListingId));
    await program.methods.marketplaceCreateEquipmentListing(1, 1, 999, bn(10_000)).accounts({
      seller: fx.seller.publicKey,
      config: fx.config,
      inventory: fx.sellerInventory,
      listing: l2,
      systemProgram: SystemProgram.programId,
    }).signers([fx.seller]).rpc();
  });

  await program.methods.marketplaceCancelEquipmentListing().accounts({
    seller: fx.seller.publicKey,
    listing,
    inventory: fx.sellerInventory,
  }).signers([fx.seller]).rpc();

  invSeller = await program.account.equipmentInventoryState.fetch(fx.sellerInventory);
  assertEq("seller bucket restored", Number(invSeller.newHand[1]), before);

  const cfg2: any = await program.account.config.fetch(fx.config);
  const listing2 = listingPda(program.programId, Number(cfg2.nextListingId));
  await program.methods.marketplaceCreateEquipmentListing(1, 1, 1, bn(200_000)).accounts({
    seller: fx.seller.publicKey,
    config: fx.config,
    inventory: fx.sellerInventory,
    listing: listing2,
    systemProgram: SystemProgram.programId,
  }).signers([fx.seller]).rpc();

  let invBuyer: any = await program.account.equipmentInventoryState.fetch(fx.buyerInventory);
  const buyerBefore = Number(invBuyer.newHand[1]);

  await program.methods.marketplaceBuyEquipmentListing().accounts({
    buyer: fx.buyer.publicKey,
    listing: listing2,
    seller: fx.seller.publicKey,
    buyerInventory: fx.buyerInventory,
    economy: fx.economy,
    essMint: fx.essMint,
    buyerAta: fx.buyerAta,
    sellerAta: fx.sellerAta,
    recipientAta: fx.recipientAta,
    tokenProgram: TOKEN_PROGRAM_ID,
  }).signers([fx.buyer]).rpc();

  invBuyer = await program.account.equipmentInventoryState.fetch(fx.buyerInventory);
  assertEq("buyer bucket incremented", Number(invBuyer.newHand[1]), buyerBefore + 1);

  console.log("✅ marketplace equipment flow passed");
}
