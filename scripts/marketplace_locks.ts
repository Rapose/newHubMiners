import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { SystemProgram } from "@solana/web3.js";
import { expectTxFail, bn } from "./helpers/test_env";
import { buildMarketplaceFixture, listingPda } from "./helpers/marketplace_fixture";

export async function runMarketplaceLocksFlow() {
  const fx = await buildMarketplaceFixture();
  const { program } = fx.env;

  const cfg: any = await program.account.config.fetch(fx.config);
  const minerListing = listingPda(program.programId, Number(cfg.nextListingId));

  await program.methods.marketplaceCreateMinerListing(bn(500_000)).accounts({
    seller: fx.seller.publicKey,
    config: fx.config,
    miner: fx.miner,
    listing: minerListing,
    systemProgram: SystemProgram.programId,
  }).signers([fx.seller]).rpc();

  await expectTxFail("listed miner cannot assign land", async () => {
    await program.methods.globalMiningAssignLand().accounts({
      owner: fx.seller.publicKey,
      minerState: fx.miner,
      landState: fx.land,
    }).signers([fx.seller]).rpc();
  });

  await expectTxFail("listed miner cannot level up", async () => {
    await program.methods.minerLevelUp().accounts({
      owner: fx.seller.publicKey,
      minerState: fx.miner,
      progression: fx.progression,
      minerProgress: fx.minerProgress,
      economy: fx.economy,
      essMint: fx.essMint,
      userAta: fx.sellerAta,
      recipientWallet: fx.recipient.publicKey,
      recipientAta: fx.recipientAta,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    }).signers([fx.seller]).rpc();
  });

  await program.methods.marketplaceCancelMinerListing().accounts({
    seller: fx.seller.publicKey,
    listing: minerListing,
    miner: fx.miner,
  }).signers([fx.seller]).rpc();

  const cfg2: any = await program.account.config.fetch(fx.config);
  const landListing = listingPda(program.programId, Number(cfg2.nextListingId));

  await program.methods.marketplaceCreateLandListing(bn(400_000)).accounts({
    seller: fx.seller.publicKey,
    config: fx.config,
    land: fx.land,
    listing: landListing,
    systemProgram: SystemProgram.programId,
  }).signers([fx.seller]).rpc();

  await expectTxFail("listed land cannot receive miner", async () => {
    await program.methods.globalMiningAssignLand().accounts({
      owner: fx.seller.publicKey,
      minerState: fx.miner,
      landState: fx.land,
    }).signers([fx.seller]).rpc();
  });

  console.log("✅ marketplace locks flow passed");
}
