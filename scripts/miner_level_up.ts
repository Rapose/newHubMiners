import { runMarketplaceLocksFlow } from "./marketplace_locks";

// Mantido por compatibilidade com runner antigo do projeto.
export async function runMinerLevelUpFlow() {
  return runMarketplaceLocksFlow();
}
