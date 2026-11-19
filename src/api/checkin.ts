import type { Checkin } from "@/bindings/Checkin";
import type { FocusFields } from "@/bindings/FocusFields";
import type { VaultStatus } from "@/bindings/VaultStatus";
import type { Written } from "@/bindings/Written";
import { call } from "./client";

/** The whole check-in, or null when nothing is due (CHK-1 to CHK-7). */
export const getCheckin = () => call<Checkin | null>("get_checkin");

/** Finish: `focus` must be one of the offers `getCheckin` returned (CHK-7). */
export const completeCheckin = (focus: FocusFields) =>
  call<Written[]>("complete_checkin", { focus });

/** The stats, the level, the active focuses and the streaks for today (LVL-1 to LVL-8). */
export const getVaultStatus = () => call<VaultStatus>("get_vault_status");
