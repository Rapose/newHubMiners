import { runMarketplaceEquipmentFlow } from "./marketplace_equipment";

// Mantido por compatibilidade com runner antigo do projeto.
export async function runEquipmentFlow() {
  return runMarketplaceEquipmentFlow();
}
