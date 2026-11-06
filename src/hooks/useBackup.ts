import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { backupNow, getBackupStatus, openBackupFolder } from "@/api/backup";
import type { BackupStatus } from "@/bindings/BackupStatus";
import { notify } from "@/components/ui/toast";
import { queryKeys } from "@/lib/queryKeys";

/** The Backup tab's folder, switch, last backup and warning (SET-3, BAK-7). */
export const useBackupStatus = () =>
  useQuery({ queryKey: queryKeys.backup, queryFn: getBackupStatus });

/**
 * Back up now (BAK-1), from the Backup tab or the dashboard's warning: "Backup saved as …" with
 * Open folder, then the last backup's date and the dashboard's warning reload.
 */
export function useBackupNow() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: backupNow,
    onSuccess: ({ fileName }) => {
      notify.success(`Backup saved as ${fileName}`, {
        label: "Open folder",
        onClick: () =>
          openBackupFolder().catch((error: Error) =>
            notify.error(error.message),
          ),
      });
      return Promise.all([
        client.invalidateQueries({ queryKey: queryKeys.backup }),
        client.invalidateQueries({ queryKey: queryKeys.dashboards }),
      ]);
    },
    onError: (error) => notify.error(error.message),
  });
}

/** Saves one backup setting at once, with "Saved" (UI_FLOWS §6). */
export function useSaveBackupSetting<T>(
  save: (value: T) => Promise<BackupStatus>,
) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: save,
    onSuccess: (status) => {
      client.setQueryData(queryKeys.backup, status);
      notify.success("Saved");
    },
    onError: (error) => notify.error(error.message),
  });
}
