import { useMutation } from "@tanstack/react-query";
import {
  ArchiveRestore,
  DatabaseBackup,
  FileSpreadsheet,
  FolderOpen,
} from "lucide-react";
import { useState } from "react";
import { useNavigate } from "react-router";
import {
  exportCsv,
  inspectBackup,
  pickBackupFile,
  pickCsvPath,
  pickFolder,
  restoreBackup,
  setAutoBackup,
  setBackupFolder,
} from "@/api/backup";
import type { BackupStatus } from "@/bindings/BackupStatus";
import { Banner } from "@/components/Banner";
import { ErrorState } from "@/components/ErrorState";
import { Loading, Skeleton } from "@/components/Loading";
import { Button } from "@/components/ui/button";
import { DatePicker } from "@/components/ui/date-picker";
import { ConfirmDialog } from "@/components/ui/dialog";
import { Switch } from "@/components/ui/switch";
import { notify } from "@/components/ui/toast";
import {
  useBackupNow,
  useBackupStatus,
  useSaveBackupSetting,
} from "@/hooks/useBackup";
import { useLedgerMutation } from "@/hooks/useLedgerMutation";
import { usePeriod } from "@/hooks/usePeriod";
import { useSettings } from "@/hooks/useSettings";
import { formatDateTime, isIsoDate } from "@/lib/dates";
import { Row } from "./GeneralTab";

const showError = (error: Error) => notify.error(error.message);

/**
 * Settings → Backup (UI_FLOWS §6, F9): the warning (BAK-7), the folder and the daily backup
 * (SET-3), the last backup with Back up now (BAK-1), Restore (BAK-4) and Export (BAK-6).
 */
export function BackupTab() {
  const status = useBackupStatus();
  const backup = useBackupNow();
  if (!status.data)
    return status.isError ? (
      <ErrorState
        onRetry={() => status.refetch()}
        pending={status.isFetching}
      />
    ) : (
      <Loading className="grid gap-3 py-3">
        {Array.from({ length: 5 }, (_, i) => (
          <Skeleton key={i} className="h-9" />
        ))}
      </Loading>
    );
  const s = status.data;
  return (
    <div className="pb-4">
      {s.due && (
        <Banner className="mt-1 mb-2">
          {s.lastBackupAt
            ? "Your last backup is more than 7 days old."
            : "You don't have a backup yet."}
        </Banner>
      )}
      <div>
        <FolderRow status={s} />
        <AutoBackupRow status={s} />
        <Row
          id="backup-now"
          label="Last backup"
          help="Manual backups are never deleted automatically."
        >
          <p className="tabular-nums">
            {s.lastBackupAt ? formatDateTime(s.lastBackupAt) : "Never"}
          </p>
          <Button
            variant="primary"
            onClick={() => backup.mutate()}
            disabled={backup.isPending}
          >
            <DatabaseBackup />
            {backup.isPending ? "Backing up…" : "Back up now"}
          </Button>
        </Row>
        <RestoreRow folder={s.folder} />
        <ExportRow />
      </div>
    </div>
  );
}

/** Where backups go (SET-3): Change… opens a folder picker, and Rust checks the folder. */
function FolderRow({ status }: { status: BackupStatus }) {
  const save = useSaveBackupSetting(setBackupFolder);
  const change = async () => {
    try {
      const path = await pickFolder(status.folder);
      if (path) save.mutate(path);
    } catch (error) {
      showError(error as Error);
    }
  };
  return (
    <Row
      id="backup-folder"
      label="Backup folder"
      help="New backups go here. Backups already made stay where they are."
    >
      <p className="min-w-0 break-all">{status.folder}</p>
      <Button onClick={change} disabled={save.isPending}>
        <FolderOpen />
        Change…
      </Button>
      {!status.folderIsDefault && (
        <Button
          variant="ghost"
          onClick={() => save.mutate(null)}
          disabled={save.isPending}
        >
          Use the default
        </Button>
      )}
    </Row>
  );
}

/** The daily automatic backup (SET-3, BAK-2, BAK-3). */
function AutoBackupRow({ status }: { status: BackupStatus }) {
  const save = useSaveBackupSetting(setAutoBackup);
  return (
    <Row
      id="auto-backup"
      label="Automatic backup"
      labelable
      help="Once a day, the first time the app opens. The newest 30 are kept."
    >
      <Switch
        id="auto-backup"
        checked={status.autoBackup}
        onCheckedChange={(on) => save.mutate(on)}
        disabled={save.isPending}
      />
    </Row>
  );
}

/**
 * Restore from backup… (F9): pick a file, which Rust checks, then confirm with its date. Once
 * restored, every query reloads and the app goes to the dashboard.
 */
function RestoreRow({ folder }: { folder: string }) {
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
      const path = await pickBackupFile(folder);
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
