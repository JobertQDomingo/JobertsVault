import { CircleAlert, RotateCw } from "lucide-react";
import { EmptyState } from "./EmptyState";
import { Button } from "./ui/button";

/** Data that failed to load: an inline message and Retry (UI_FLOWS §9). Rust logged the details. */
export function ErrorState({
  onRetry,
  className,
}: {
  onRetry: () => void;
  className?: string;
}) {
  return (
    <EmptyState
      icon={CircleAlert}
      className={className}
      message="This couldn't be loaded. The details are in the error log."
      action={
        <Button onClick={onRetry}>
          <RotateCw />
          Retry
        </Button>
      }
    />
  );
}
