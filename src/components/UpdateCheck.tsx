import { useIsMutating, useMutation } from "@tanstack/react-query";
import { Download, RefreshCw } from "lucide-react";
import { checkUpdate, installUpdate } from "@/api/system";
import { Button } from "./ui/button";
import { notify } from "./ui/toast";

// Shared, so leaving the screen and coming back mid-download can't start a second install.
const INSTALLING = ["installUpdate"];

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
    mutationKey: INSTALLING,
    // Settles only if the installer didn't start: on success the app closes and reopens.
    mutationFn: installUpdate,
    onError: (error) => {
      notify.error(error.message);
      // Rust has let go of that update, so retrying starts with a new check.
      check.reset();
    },
  });
  const installing = useIsMutating({ mutationKey: INSTALLING }) > 0;
  const version = check.data;
  const found = check.isSuccess && version != null;
  return (
    <div className="flex flex-wrap items-center gap-3">
      <Button
        onClick={() => check.mutate()}
        disabled={check.isPending || installing}
      >
        <RefreshCw />
        {check.isPending ? "Checking…" : "Check for updates"}
      </Button>
      {/* Always in the page, so screen readers announce its text when it changes. */}
      <p role="status">
        {check.isSuccess &&
          (version == null ? "You're up to date." : `${version} is out.`)}
      </p>
      {found && (
        <Button
          variant="primary"
          onClick={() => install.mutate()}
          disabled={installing}
        >
          <Download />
          {installing ? `Updating to ${version}…` : "Update now"}
        </Button>
      )}
    </div>
  );
}
