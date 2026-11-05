import {
  Children,
  cloneElement,
  isValidElement,
  type ComponentProps,
  type ReactElement,
  type ReactNode,
} from "react";
import { cn } from "@/lib/utils";

/** The field look shared by inputs, the combobox and the date picker (DESIGN_SYSTEM §9.1). */
export const fieldClasses =
  "h-9 w-full min-w-0 rounded-control border border-field-edge bg-field px-3 text-body text-foreground outline-none placeholder:text-muted-foreground focus:border-ring focus:ring-3 focus:ring-ring/25 aria-invalid:border-danger aria-invalid:ring-3 aria-invalid:ring-danger/18 disabled:opacity-50";

export function Input({ className, ...props }: ComponentProps<"input">) {
  return <input className={cn(fieldClasses, className)} {...props} />;
}

/**
 * A label, one control, and the error (or help) under it in small text. The control gets
 * `aria-describedby` so a screen reader reads the message with it.
 */
export function Field({
  label,
  htmlFor,
  error,
  help,
  className,
  children,
}: {
  label: ReactNode;
  htmlFor: string;
  error?: string;
  help?: ReactNode;
  className?: string;
  children: ReactElement<{ "aria-describedby"?: string }>;
}) {
  const messageId = `${htmlFor}-message`;
  const message = error ?? help;
  const control = Children.only(children);
  return (
    <div className={cn("grid gap-1", className)}>
      <label
        htmlFor={htmlFor}
        className="text-small font-semibold text-muted-foreground"
      >
        {label}
      </label>
      {isValidElement(control) && message
        ? cloneElement(control, { "aria-describedby": messageId })
        : control}
      {error ? (
        <p id={messageId} role="alert" className="text-small text-danger">
          {error}
        </p>
      ) : help ? (
        <p id={messageId} className="text-small text-muted-foreground">
          {help}
        </p>
      ) : null}
    </div>
  );
}
