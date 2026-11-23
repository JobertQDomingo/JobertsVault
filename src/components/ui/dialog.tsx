import { X } from "lucide-react";
import { Dialog as DialogPrimitive } from "radix-ui";
import type { ComponentProps, ReactNode } from "react";
import { cn } from "@/lib/utils";
import { Button } from "./button";

// DESIGN_SYSTEM §9.1: Level 2 glass, 24 px corners, 480 px for forms, 400 px for confirmations and
// 640 px for a list whose rows have actions.
// The app behind dims and blurs 6 px. Esc closes it and focus stays inside (Radix).
export const Dialog = DialogPrimitive.Root;

const WIDTHS = {
  form: "max-w-[480px]",
  confirm: "max-w-[400px]",
  list: "max-w-[640px]",
};

/**
 * The last 50 elements that had focus, the latest last, each once, with <body> standing for focus
 * that fell to nothing. The app opens its dialogs from state, with no Radix Trigger to return to,
 * so Radix would drop focus to <body> on close (A11Y-01). A field's `autoFocus` moves focus in
 * before Radix's open event, so the opener is read on close instead: the latest of these outside
 * the closing dialog. A menu item counts as its menu's ⋯ button, since the menu is gone by then.
 */
let focused: HTMLElement[] = [];
const remember = (el: HTMLElement) => {
  focused = [...focused.filter((f) => f !== el), el].slice(-50);
};
document.addEventListener("focusin", (e) => {
  const el = e.target as HTMLElement;
  const trigger = el.closest("[role=menu]")?.getAttribute("aria-labelledby");
  remember((trigger && document.getElementById(trigger)) || el);
});
// Focus left for nothing (a click on the page): not a removed element, and not the window losing it.
document.addEventListener("focusout", (e) => {
  const from = e.target as HTMLElement;
  if (e.relatedTarget) return;
  setTimeout(() => {
    if (from.isConnected && document.activeElement === document.body)
      remember(document.body);
  });
});

export function DialogContent({
  title,
  description,
  size = "form",
  className,
  children,
  onCloseAutoFocus,
  ...props
}: Omit<ComponentProps<typeof DialogPrimitive.Content>, "title"> & {
  title: ReactNode;
  description?: ReactNode;
  size?: keyof typeof WIDTHS;
}) {
  return (
    <DialogPrimitive.Portal>
      <DialogPrimitive.Overlay className="fixed inset-0 z-50 animate-fade-in bg-overlay backdrop-blur-[6px]" />
      <DialogPrimitive.Content
        // Without a description, say so, or Radix warns about a missing one.
        {...(description ? {} : { "aria-describedby": undefined })}
        className={cn(
          "glass-raised fixed top-1/2 left-1/2 z-50 w-[calc(100%-32px)] -translate-1/2 animate-dialog-in rounded-dialog",
          WIDTHS[size],
          className,
        )}
        onCloseAutoFocus={(e) => {
          onCloseAutoFocus?.(e);
          const dialog = e.currentTarget as Node;
          const el = [...focused].reverse().find((f) => !dialog.contains(f));
          // Only the opener itself: if it's gone, or behind a dialog that's still open, Radix's
          // default, rather than some older control elsewhere.
          if (
            e.defaultPrevented ||
            !el?.isConnected ||
            el === document.body ||
            el.closest("[aria-hidden=true], [inert]")
          )
            return;
          e.preventDefault();
          el.focus();
        }}
        {...props}
      >
        <DialogPrimitive.Title className="px-[22px] pt-[18px] pr-14 pb-2 text-h2">
          {title}
        </DialogPrimitive.Title>
        {description && (
          <DialogPrimitive.Description className="px-[22px] py-2 text-muted-foreground">
            {description}
          </DialogPrimitive.Description>
        )}
        {children}
        {/* Last in the tab order, so opening focuses the form or Cancel, never ✕. */}
        <DialogPrimitive.Close asChild>
          <Button
            variant="ghost"
            size="icon"
            aria-label="Close"
            className="absolute top-4 right-4"
          >
            <X />
          </Button>
        </DialogPrimitive.Close>
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
  cancelLabel = "Cancel",
  destructive = false,
  onConfirm,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: ReactNode;
  message: ReactNode;
  confirmLabel: string;
  cancelLabel?: string;
  destructive?: boolean;
  onConfirm: () => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent title={title} description={message} size="confirm">
        <DialogFooter>
          <Button onClick={() => onOpenChange(false)}>{cancelLabel}</Button>
          <Button
            variant={destructive ? "destructive" : "primary"}
            // Closes first, so a second click can't run the action twice.
            onClick={() => {
              onOpenChange(false);
              onConfirm();
            }}
          >
            {confirmLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
