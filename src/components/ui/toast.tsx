import { CircleAlert, CircleCheck } from "lucide-react";
import { Toaster as Sonner, toast } from "sonner";

/** Rendered once. Toasts are Level 2 glass at the bottom right and close after 4 s (DESIGN_SYSTEM §9.1). */
export function Toaster() {
  return (
    <Sonner
      position="bottom-right"
      duration={4000}
      // Sonner's own stylesheet sets a system font, and it outranks Tailwind's classes.
      style={{ fontFamily: "inherit" }}
      icons={{
        success: <CircleCheck className="size-4 text-success" />,
        error: <CircleAlert className="size-4 text-danger" />,
      }}
      toastOptions={{
        unstyled: true,
        classNames: {
          toast:
            "glass-raised flex w-[356px] items-center gap-2.5 rounded-toolbar py-2.5 pr-3 pl-3.5 text-body text-foreground",
          content: "min-w-0 flex-1",
          closeButton:
            "order-last grid size-6 shrink-0 place-items-center rounded-chip text-muted-foreground hover:bg-wash hover:text-foreground [&_svg]:size-3.5",
        },
      }}
    />
  );
}

export const notify = {
  success: (message: string) => toast.success(message),
  // Errors stay until closed, so they can't be missed.
  error: (message: string) =>
    toast.error(message, { duration: Infinity, closeButton: true }),
};
