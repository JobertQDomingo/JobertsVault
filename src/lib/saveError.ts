import { ApiError } from "@/api/client";
import { notify } from "@/components/ui/toast";

/**
 * Shows why a save failed: the backend's validation message under the field it names when that
 * field is on screen (ARCHITECTURE §4.4), anything else as a toast.
 */
export function showSaveError<F extends string>(
  error: unknown,
  shown: readonly F[],
  setError: (field: F, error: { message: string }) => void,
) {
  const detail = error instanceof ApiError ? error.detail : null;
  const field =
    detail?.code === "validation"
      ? shown.find((f) => f === detail.field)
      : undefined;
  if (field && detail?.code === "validation")
    setError(field, { message: detail.message });
  else notify.error(error instanceof Error ? error.message : String(error));
}
