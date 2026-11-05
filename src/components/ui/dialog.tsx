import { X } from "lucide-react";
import { Dialog as DialogPrimitive } from "radix-ui";
import type { ComponentProps, ReactNode } from "react";
import { cn } from "@/lib/utils";
import { Button } from "./button";

// DESIGN_SYSTEM §9.1: Level 2 glass, 24 px corners, 480 px for forms and 400 px for confirmations.
// The app behind dims and blurs 6 px. Esc closes it and focus stays inside (Radix).
export const Dialog = DialogPrimitive.Root;

export function DialogContent({
  title,
  description,
  size = "form",
  className,
  children,
  ...props
}: Omit<ComponentProps<typeof DialogPrimitive.Content>, "title"> & {
  title: ReactNode;
  description?: ReactNode;
  size?: "form" | "confirm";
}) {
  return (
    <DialogPrimitive.Portal>
      <DialogPrimitive.Overlay className="fixed inset-0 z-50 animate-fade-in bg-overlay backdrop-blur-[6px]" />
      <DialogPrimitive.Content
        // Without a description, say so, or Radix warns about a missing one.
        {...(description ? {} : { "aria-describedby": undefined })}
        className={cn(
          "glass-raised fixed top-1/2 left-1/2 z-50 w-[calc(100%-32px)] -translate-1/2 animate-dialog-in rounded-dialog",
          size === "form" ? "max-w-[480px]" : "max-w-[400px]",
          className,
        )}
        {...props}
      >
        <div className="flex items-center justify-between gap-2 px-[22px] pt-[18px] pb-2">
          <DialogPrimitive.Title className="text-h2">
            {title}
          </DialogPrimitive.Title>
          <DialogPrimitive.Close asChild>
            <Button variant="ghost" size="icon" aria-label="Close">
              <X />
            </Button>
          </DialogPrimitive.Close>
        </div>
        {description && (
          <DialogPrimitive.Description className="px-[22px] py-2 text-muted-foreground">
            {description}
          </DialogPrimitive.Description>
        )}
        {children}
      </DialogPrimitive.Content>
    </DialogPrimitive.Portal>
  );
}

export function DialogBody({ className, ...props }: ComponentProps<"div">) {
  return (
    <div className={cn("grid gap-3 px-[22px] py-2", className)} {...props} />
  );
}

export function DialogFooter({ className, ...props }: ComponentProps<"div">) {
  return (
    <div
      className={cn("flex justify-end gap-2 px-[22px] pt-4 pb-5", className)}
      {...props}
    />
  );
}

/** A title, one sentence, Cancel, and a button that names the action (DESIGN_SYSTEM §9.1). */
export function ConfirmDialog({
  open,
  onOpenChange,
  title,
  message,
  confirmLabel,
  destructive = false,
  onConfirm,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: ReactNode;
  message: ReactNode;
  confirmLabel: string;
  destructive?: boolean;
  onConfirm: () => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent title={title} description={message} size="confirm">
        <DialogFooter>
          <Button onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button
            variant={destructive ? "destructive" : "primary"}
            onClick={onConfirm}
          >
            {confirmLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
