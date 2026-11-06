import { useMutation } from "@tanstack/react-query";
import { FolderOpen } from "lucide-react";
import logo from "../../assets/brand/logo.svg";
import { openDataFolder } from "@/api/system";
import { GlassPanel } from "./GlassPanel";
import { UpdateCheck } from "./UpdateCheck";
import { Button } from "./ui/button";
import { notify } from "./ui/toast";

/**
 * The whole window when the database couldn't be opened (ARCHITECTURE §6, UI_FLOWS §9): what went
 * wrong, where the database is, a way to open its folder, and Check for updates, since a newer
 * version may fix it (UPD-6).
 */
export function StartupErrorScreen({
  message,
  databasePath,
}: {
  message: string;
  /** Unknown when the status itself couldn't be read. */
  databasePath?: string;
}) {
  const open = useMutation({
    mutationFn: openDataFolder,
    onError: (error) => notify.error(error.message),
  });
  return (
    <main className="grid h-full place-items-center p-3">
      <GlassPanel className="grid w-[560px] max-w-full gap-4 p-8">
        <div className="flex items-center gap-3">
          <img src={logo} alt="" className="size-10" />
          <h1 className="text-h1">Jobert's Vault couldn't start</h1>
        </div>
        <p>{message}</p>
        {databasePath && (
          <div className="grid gap-1">
            <p className="text-small font-semibold text-muted-foreground">
              Database
            </p>
            <p className="rounded-control bg-wash px-3 py-2 font-mono text-small break-all">
              {databasePath}
            </p>
          </div>
        )}
        <div className="flex items-start justify-between gap-3">
          <UpdateCheck />
          <Button onClick={() => open.mutate()} disabled={open.isPending}>
            <FolderOpen />
            Open data folder
          </Button>
        </div>
      </GlassPanel>
    </main>
  );
}
