import { open, save } from "@tauri-apps/plugin-dialog";
import type { Backup } from "@/bindings/Backup";
import type { BackupFile } from "@/bindings/BackupFile";
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

/** The Windows open dialog, for a backup to restore. null when cancelled. Rust checks the path. */
export const pickBackupFile = () =>
  open({
    title: "Restore from backup",
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
