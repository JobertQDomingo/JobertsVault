import { useMutation } from "@tanstack/react-query";
import { ArchiveRestore, DatabaseBackup, FileSpreadsheet } from "lucide-react";
import { useState } from "react";
import { useNavigate } from "react-router";
import {
  backupNow,
  exportCsv,
  inspectBackup,
  openBackupFolder,
  pickBackupFile,
  pickCsvPath,
  restoreBackup,
} from "@/api/backup";
import { Button } from "@/components/ui/button";
import { DatePicker } from "@/components/ui/date-picker";
import { ConfirmDialog } from "@/components/ui/dialog";
import { notify } from "@/components/ui/toast";
import { useLedgerMutation } from "@/hooks/useLedgerMutation";
import { usePeriod } from "@/hooks/usePeriod";
import { useSettings } from "@/hooks/useSettings";
import { formatDateTime, isIsoDate } from "@/lib/dates";
import { Row } from "./GeneralTab";

const showError = (error: Error) => notify.error(error.message);

/** Settings → Backup (UI_FLOWS §6, F9): Back up now (BAK-1), Restore (BAK-4), Export (BAK-6). */
export function BackupTab() {
  const backup = useMutation({
    mutationFn: backupNow,
    onSuccess: ({ fileName }) =>
      notify.success(`Backup saved as ${fileName}`, {
        label: "Open folder",
        onClick: () => openBackupFolder().catch(showError),
      }),
    onError: showError,
  });
  return (
    <div className="pb-4">
      <Row
        id="backup-now"
        label="Manual backup"
        help="Saves a copy of all your data in the backup folder, named with the date and time. Manual backups are never deleted automatically."
      >
        <Button
          variant="primary"
          onClick={() => backup.mutate()}
          disabled={backup.isPending}
        >
          <DatabaseBackup />
          {backup.isPending ? "Backing up…" : "Back up now"}
        </Button>
      </Row>
      <RestoreRow />
      <ExportRow />
    </div>
  );
}

/**
 * Restore from backup… (F9): pick a file, which Rust checks, then confirm with its date. Once
 * restored, every query reloads and the app goes to the dashboard.
 */
function RestoreRow() {
  const navigate = useNavigate();
  const [chosen, setChosen] = useState<{
    path: string;
    modified: string;
  } | null>(null);
  const inspect = useMutation({ mutationFn: inspectBackup });
  // On the mutation, not `mutate`, so they still run if this tab has been left meanwhile.
  const restore = useLedgerMutation(restoreBackup, {
    onSuccess: () => {
      navigate("/");
      notify.success("Backup restored");
    },
    onError: showError,
  });
  const pick = async () => {
    try {
      const path = await pickBackupFile();
      if (!path) return;
      const { modified } = await inspect.mutateAsync(path);
      setChosen({ path, modified });
    } catch (error) {
      showError(error as Error);
    }
  };
  const busy = inspect.isPending || restore.isPending;
  return (
    <Row
      id="restore"
      label="Restore"
      help="Replaces all your data with a backup. A safety copy of your current data is saved first."
    >
      <Button onClick={pick} disabled={busy}>
        <ArchiveRestore />
        {restore.isPending ? "Restoring…" : "Restore from backup…"}
      </Button>
      <ConfirmDialog
        open={chosen !== null}
        onOpenChange={(open) => !open && setChosen(null)}
        title="Restore this backup?"
        message={`This replaces all current data with the backup from ${chosen ? formatDateTime(chosen.modified) : ""}. A safety copy of your current data will be saved first.`}
        confirmLabel="Restore"
        destructive
        onConfirm={() => chosen && restore.mutate(chosen.path)}
      />
    </Row>
  );
}

const THIS_MONTH = { kind: "month", anchor: null } as const;

/**
 * Export to CSV (BAK-6): From and To start at this month so far, from `get_period`, so the
 * frontend works out no dates. Export… asks where to save, then Rust writes the file.
 */
function ExportRow() {
  const settings = useSettings();
  const month = usePeriod(THIS_MONTH);
  const [picked, setPicked] = useState<{ start?: string; end?: string }>({});
  const start = picked.start ?? month.data?.start ?? "";
  const end = picked.end ?? month.data?.today ?? "";
  const exporter = useMutation({
    mutationFn: async () => {
      // Rust checks them again; this only spares a trip through the save dialog. ISO dates
      // compare correctly as text.
      if (!isIsoDate(start) || !isIsoDate(end) || end < start)
        throw new Error("Enter a From date and a To date on or after it.");
      const path = await pickCsvPath(`jobertsvault-${start}-to-${end}.csv`);
      return path ? { path, ...(await exportCsv({ start, end }, path)) } : null;
    },
    onSuccess: (done) =>
      done &&
      notify.success(
        `Exported ${done.rows} ${done.rows === 1 ? "transaction" : "transactions"} to ${done.path.split(/[\\/]/).pop()}`,
      ),
    onError: showError,
  });
  const weekStart = settings.data?.weekStart ?? "monday";
  return (
    <Row
      id="export"
      label="Export to CSV"
      help="Every transaction dated from and to these days, for Excel or another app"
    >
      <DatePicker
        aria-label="From"
        value={start}
        onChange={(next) => setPicked((p) => ({ ...p, start: next }))}
        weekStart={weekStart}
        max={month.data?.today}
        className="w-40"
      />
      <span className="text-muted-foreground">to</span>
      <DatePicker
        aria-label="To"
        value={end}
        onChange={(next) => setPicked((p) => ({ ...p, end: next }))}
        weekStart={weekStart}
        max={month.data?.today}
        className="w-40"
      />
      <Button
        onClick={() => exporter.mutate()}
        disabled={exporter.isPending || !month.data}
      >
        <FileSpreadsheet />
        {exporter.isPending ? "Exporting…" : "Export…"}
      </Button>
    </Row>
  );
}
