import { expectFailure, logStep, SuiteCtx } from "./_shared";

export async function runMarketplaceLockTests(ctx: SuiteCtx, a: Record<string, any>) {
  const p = ctx.program;

  logStep("locks/miner listed cannot claim exp");
  await expectFailure("claim_mining_exp when miner listed", async () => {
    await p.methods.claimMiningExp().accounts({
      owner: ctx.seller.publicKey,
      minerState: a.miner,
      progression: a.progression,
      minerProgress: a.minerProgress,
    }).rpc();
  });

  logStep("locks/miner listed cannot assign land");
  await expectFailure("assign land when miner listed", async () => {
    await p.methods.globalMiningAssignLand().accounts({
      owner: ctx.seller.publicKey,
      minerState: a.miner,
      landState: a.land,
    }).rpc();
  });

  logStep("locks/land listed cannot assign miner");
  await expectFailure("assign listed land", async () => {
    await p.methods.globalMiningAssignLand().accounts({
      owner: ctx.seller.publicKey,
      minerState: a.minerUnlocked,
      landState: a.landListed,
    }).rpc();
  });

  logStep("locks/miner listed cannot equipment init/replace/remelt");
  await expectFailure("equipment init while listed", async () => {
    await p.methods.equipmentInit().accounts({
      owner: ctx.seller.publicKey,
      minerState: a.miner,
      equipment: a.equipment,
    }).rpc();
  });
}
