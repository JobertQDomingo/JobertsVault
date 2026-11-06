import { useMutation } from "@tanstack/react-query";
import { DatabaseBackup } from "lucide-react";
import { backupNow, openBackupFolder } from "@/api/backup";
import { Button } from "@/components/ui/button";
import { notify } from "@/components/ui/toast";
import { Row } from "./GeneralTab";

/** Settings → Backup (UI_FLOWS §6, F9): Back up now (BAK-1). */
export function BackupTab() {
  const backup = useMutation({
    mutationFn: backupNow,
    onSuccess: ({ fileName }) =>
      notify.success(`Backup saved as ${fileName}`, {
        label: "Open folder",
        onClick: () =>
          openBackupFolder().catch((error: Error) =>
            notify.error(error.message),
          ),
      }),
    onError: (error) => notify.error(error.message),
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
    </div>
  );
}
