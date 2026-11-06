import { open } from "@tauri-apps/plugin-dialog";
import type { Backup } from "@/bindings/Backup";
import type { BackupFile } from "@/bindings/BackupFile";
import { ApiError, call } from "./client";

/** Copies the database into the backup folder (BAK-1). */
export const backupNow = () => call<Backup>("backup_now");

/** Opens Explorer at the backup folder. */
export const openBackupFolder = () => call<void>("open_backup_folder");

/** The Windows open dialog, for a backup to restore. null when cancelled. Rust checks the path. */
export const pickBackupFile = () =>
  open({
    title: "Restore from backup",
    filters: [{ name: "Jobert's Vault backups", extensions: ["db"] }],
  }).catch((error: unknown) => {
    throw new ApiError({ code: "unknown", message: String(error) });
  });

/** Checks a picked file and says when it was written, changing nothing (BAK-4). */
export const inspectBackup = (path: string) =>
  call<BackupFile>("inspect_backup", { path });

/** Replaces all the data with the backup at `path` (BAK-4). */
export const restoreBackup = (path: string) =>
  call<void>("restore_backup", { path });
