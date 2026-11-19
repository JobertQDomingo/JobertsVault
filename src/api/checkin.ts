import type { VaultStatus } from "@/bindings/VaultStatus";
import { call } from "./client";

/** The stats, the level, the active focuses and the streaks for today (LVL-1 to LVL-8). */
export const getVaultStatus = () => call<VaultStatus>("get_vault_status");
