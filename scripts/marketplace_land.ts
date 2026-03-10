import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { SystemProgram } from "@solana/web3.js";
import { assertEq, bn, expectTxFail } from "./helpers/test_env";
import { buildMarketplaceFixture, fetchListing, listingPda } from "./helpers/marketplace_fixture";

export async function runMarketplaceLandFlow() {
  const fx = await buildMarketplaceFixture();
  const { program } = fx.env;

  await program.methods.globalMiningAssignLand().accounts({
    owner: fx.seller.publicKey,
    minerState: fx.miner,
    landState: fx.land,
  }).signers([fx.seller]).rpc();

  await expectTxFail("cannot list occupied land", async () => {
    const cfg: any = await program.account.config.fetch(fx.config);
    const listing = listingPda(program.programId, Number(cfg.nextListingId));
    await program.methods.marketplaceCreateLandListing(bn(700_000)).accounts({
      seller: fx.seller.publicKey,
      config: fx.config,
      land: fx.land,
      listing,
      systemProgram: SystemProgram.programId,
    }).signers([fx.seller]).rpc();
  });

  await program.methods.globalMiningUnassignLand().accounts({
    owner: fx.seller.publicKey,
    minerState: fx.miner,
    landState: fx.land,
  }).signers([fx.seller]).rpc();

  const cfg: any = await program.account.config.fetch(fx.config);
  const listing = listingPda(program.programId, Number(cfg.nextListingId));

  await program.methods.marketplaceCreateLandListing(bn(700_000)).accounts({
    seller: fx.seller.publicKey,
    config: fx.config,
    land: fx.land,
    listing,
    systemProgram: SystemProgram.programId,
  }).signers([fx.seller]).rpc();

  let land: any = await program.account.landState.fetch(fx.land);
  assertEq("land listed", land.listed, true);

  await expectTxFail("listed land cannot receive miner", async () => {
    await program.methods.globalMiningAssignLand().accounts({
      owner: fx.seller.publicKey,
      minerState: fx.miner,
      landState: fx.land,
    }).signers([fx.seller]).rpc();
  });

  await program.methods.marketplaceCancelLandListing().accounts({
    seller: fx.seller.publicKey,
    listing,
    land: fx.land,
  }).signers([fx.seller]).rpc();

  land = await program.account.landState.fetch(fx.land);
  assertEq("land unlisted after cancel", land.listed, false);

  const cfg2: any = await program.account.config.fetch(fx.config);
  const listing2 = listingPda(program.programId, Number(cfg2.nextListingId));

  await program.methods.marketplaceCreateLandListing(bn(700_000)).accounts({
    seller: fx.seller.publicKey,
    config: fx.config,
    land: fx.land,
    listing: listing2,
    systemProgram: SystemProgram.programId,
  }).signers([fx.seller]).rpc();

  await program.methods.marketplaceBuyLandListing().accounts({
    buyer: fx.buyer.publicKey,
    listing: listing2,
    land: fx.land,
    seller: fx.seller.publicKey,
    economy: fx.economy,
    essMint: fx.essMint,
    buyerAta: fx.buyerAta,
    sellerAta: fx.sellerAta,
    recipientAta: fx.recipientAta,
    tokenProgram: TOKEN_PROGRAM_ID,
  }).signers([fx.buyer]).rpc();

  land = await program.account.landState.fetch(fx.land);
  const listingState = await fetchListing(program, listing2);
  assertEq("land owner transferred", land.owner.toBase58(), fx.buyer.publicKey.toBase58());
  assertEq("listing closed", listingState.active, false);

  console.log("✅ marketplace land flow passed");
}
