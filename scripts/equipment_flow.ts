import { logStep, SuiteCtx } from "./_shared";

export async function runEquipmentFlow(ctx: SuiteCtx) {
  logStep("equipment_flow: listed miner lock checks for equip/remelt paths");

  // Este script fica como hook para seu fluxo real de contas.
  // Objetivo: ser chamado no runner principal após listagem de miner.
  // Cenários esperados (on-chain):
  // - equipment_init falha se miner.listed == true
  // - equipment_replace_hand/head falham se miner.listed == true
  // - equipment_remelt_hand/head falham se miner.listed == true
  console.log("equipment_flow ready: plug real PDAs from your environment");
}
