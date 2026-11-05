import { Plus } from "lucide-react";
import { useMatches } from "react-router";
import { GlassPanel } from "./GlassPanel";
import { useQuickAdd } from "./QuickAddProvider";
import { Button } from "./ui/button";

/** The page title from the route's `handle.title`, and + Add on every screen (UI_FLOWS §1). */
export function TopBar() {
  const openQuickAdd = useQuickAdd();
  const title = useMatches()
    .map((match) => (match.handle as { title?: string } | undefined)?.title)
    .filter(Boolean)
    .pop();
  return (
    <GlassPanel className="flex items-center justify-between gap-3 rounded-toolbar py-3 pr-3.5 pl-5">
      <h1 className="text-h1">{title}</h1>
      <Button
        variant="primary"
        aria-label="Add transaction"
        title="Add transaction (Ctrl+N)"
        aria-keyshortcuts="Control+N"
        onClick={openQuickAdd}
      >
        <Plus />
        Add
        <span aria-hidden className="ml-1 text-[11px] font-normal opacity-80">
          Ctrl+N
        </span>
      </Button>
    </GlassPanel>
  );
}
