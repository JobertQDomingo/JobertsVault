import type { Backup } from "@/bindings/Backup";
import { call } from "./client";

/** Copies the database into the backup folder (BAK-1). */
export const backupNow = () => call<Backup>("backup_now");

/** Opens Explorer at the backup folder. */
export const openBackupFolder = () => call<void>("open_backup_folder");
