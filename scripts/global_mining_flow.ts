import { logStep, SuiteCtx } from "./_shared";

export async function runGlobalMiningFlow(_ctx: SuiteCtx) {
  logStep("global_mining_flow: listed lock checks");
  // Cenários esperados para validar:
  // - global_mining_assign_land falha se miner.listed || land.listed
  // - global_mining_unassign_land falha se miner.listed
  // - global_mining_register_miner falha se miner.listed
  // - global_mining_update falha se miner.listed || land.listed
  console.log("global_mining_flow ready: add concrete accounts from your fixture");
}
