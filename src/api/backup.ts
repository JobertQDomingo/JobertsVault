import { open, save } from "@tauri-apps/plugin-dialog";
import type { Backup } from "@/bindings/Backup";
import type { BackupFile } from "@/bindings/BackupFile";
import type { BackupStatus } from "@/bindings/BackupStatus";
import type { CsvExport } from "@/bindings/CsvExport";
import type { DateRange } from "@/bindings/DateRange";
import { ApiError, call } from "./client";

// The dialog plugin rejects with plain text; every other call rejects with an ApiError.
const dialogFailed = (error: unknown) => {
  throw new ApiError({ code: "unknown", message: String(error) });
};

/** Copies the database into the backup folder (BAK-1). */
export const backupNow = () => call<Backup>("backup_now");

/** Opens Explorer at the backup folder. */
export const openBackupFolder = () => call<void>("open_backup_folder");

/** The folder, the switch, the last backup and the warning (SET-3, BAK-7). */
export const getBackupStatus = () => call<BackupStatus>("get_backup_status");

/** Chooses the backup folder; null goes back to the default one (SET-3). */
export const setBackupFolder = (path: string | null) =>
  call<BackupStatus>("set_backup_folder", { path });

/** Turns the daily automatic backup on or off (SET-3). */
export const setAutoBackup = (enabled: boolean) =>
  call<BackupStatus>("set_auto_backup", { enabled });

/** The Windows folder dialog, opened at `defaultPath`. null when cancelled. Rust checks it. */
export const pickFolder = (defaultPath: string) =>
  open({
    title: "Choose the backup folder",
    directory: true,
    defaultPath,
  }).catch(dialogFailed);

/** The Windows open dialog, for a backup to restore, opened at `defaultPath` (the backup
 * folder). null when cancelled. Rust checks the path. */
export const pickBackupFile = (defaultPath?: string) =>
  open({
    title: "Restore from backup",
    defaultPath,
    filters: [{ name: "Jobert's Vault backups", extensions: ["db"] }],
  }).catch(dialogFailed);

/** Checks a picked file and says when it was written, changing nothing (BAK-4). */
export const inspectBackup = (path: string) =>
  call<BackupFile>("inspect_backup", { path });

/** Replaces all the data with the backup at `path` (BAK-4). */
export const restoreBackup = (path: string) =>
  call<void>("restore_backup", { path });

/** The Windows save dialog, for a CSV file. null when cancelled. Rust checks the path. */
export const pickCsvPath = (defaultPath: string) =>
  save({
    title: "Export to CSV",
    defaultPath,
    filters: [{ name: "CSV", extensions: ["csv"] }],
  }).catch(dialogFailed);

/** Writes the transactions dated within `range` to the CSV file at `path` (BAK-6). */
export const exportCsv = (range: DateRange, path: string) =>
  call<CsvExport>("export_csv", { range, path });
