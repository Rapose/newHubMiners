import { runMarketplaceMinerFlow } from "./marketplace_miner";
import { runMarketplaceLandFlow } from "./marketplace_land";
import { runMarketplaceEquipmentFlow } from "./marketplace_equipment";
import { runMarketplaceLocksFlow } from "./marketplace_locks";

async function main() {
  console.log("\n=== run_marketplace_suite ===");

  await runMarketplaceMinerFlow();
  await runMarketplaceLandFlow();
  await runMarketplaceEquipmentFlow();
  await runMarketplaceLocksFlow();

  console.log("\n✅ marketplace suite complete");
}

main().catch((e) => {
  console.error("❌ marketplace suite failed", e);
  process.exit(1);
});
