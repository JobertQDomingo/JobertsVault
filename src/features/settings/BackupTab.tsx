import { useMutation } from "@tanstack/react-query";
import { ArchiveRestore, DatabaseBackup } from "lucide-react";
import { useState } from "react";
import { useNavigate } from "react-router";
import {
  backupNow,
  inspectBackup,
  openBackupFolder,
  pickBackupFile,
  restoreBackup,
} from "@/api/backup";
import { Button } from "@/components/ui/button";
import { ConfirmDialog } from "@/components/ui/dialog";
import { notify } from "@/components/ui/toast";
import { useLedgerMutation } from "@/hooks/useLedgerMutation";
import { formatDateTime } from "@/lib/dates";
import { Row } from "./GeneralTab";

const showError = (error: Error) => notify.error(error.message);

/** Settings → Backup (UI_FLOWS §6, F9): Back up now (BAK-1) and Restore (BAK-4). */
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
