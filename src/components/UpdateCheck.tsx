import { useMutation } from "@tanstack/react-query";
import { Download, RefreshCw } from "lucide-react";
import { checkUpdate, installUpdate } from "@/api/system";
import { Button } from "./ui/button";
import { notify } from "./ui/toast";

/**
 * **Check for updates**, then "You're up to date." or "X is out." with **Update now** (UPD-3,
 * UPD-6). Only a click connects. Update now restarts the app, so it lives where no form is open.
 * A refused or failed update says why in a toast, ending "Nothing was changed." (UPD-4).
 */
export function UpdateCheck() {
  const check = useMutation({
    mutationFn: checkUpdate,
    onError: (error) => notify.error(error.message),
  });
  const install = useMutation({
    // Settles only if the installer didn't start: on success the app closes and reopens.
    mutationFn: installUpdate,
    onError: (error) => notify.error(error.message),
  });
  const version = check.data;
  return (
    <div className="flex flex-wrap items-center gap-3">
      <Button
        onClick={() => check.mutate()}
        disabled={check.isPending || install.isPending}
      >
        <RefreshCw />
        {check.isPending ? "Checking…" : "Check for updates"}
      </Button>
      {check.isSuccess && version === null && (
        <p role="status">You're up to date.</p>
      )}
      {check.isSuccess && version != null && (
        <>
          <p role="status">{version} is out.</p>
          <Button
            variant="primary"
            onClick={() => install.mutate()}
            disabled={install.isPending}
          >
            <Download />
            {install.isPending ? `Updating to ${version}…` : "Update now"}
          </Button>
        </>
      )}
    </div>
  );
}
