import { ChevronRight, Ellipsis, Plus } from "lucide-react";
import { useId, useState, type ReactNode } from "react";
import { SortableList } from "@/components/SortableList";
import { Button } from "@/components/ui/button";
import { Menu, MenuContent, MenuTrigger } from "@/components/ui/menu";
import { notify } from "@/components/ui/toast";
import { cn } from "@/lib/utils";

type Item = { id: number; name: string; archived: boolean };

/**
 * One list in Settings (UI_FLOWS §6): its active items in order (drag ⋮⋮ or ↑/↓), each with a ⋯
 * menu, a "New …" button, and the archived ones folded under "Show archived (N)" with Unarchive.
 */
export function ManagedList<T extends Item>({
  title,
  items,
  newLabel,
  onNew,
  onReorder,
  onUnarchive,
  row,
  actions,
}: {
  title: string;
  items: T[];
  newLabel: string;
  onNew: () => void;
  onReorder: (ids: number[]) => Promise<unknown>;
  onUnarchive: (item: T) => Promise<unknown>;
  /** The row's content between the handle and the ⋯ menu. */
  row: (item: T) => ReactNode;
  /** The ⋯ menu's items. */
  actions: (item: T) => ReactNode;
}) {
  const headingId = useId();
  const [showArchived, setShowArchived] = useState(false);
  const archived = items.filter((i) => i.archived);
  const failed = (error: Error) => {
    notify.error(error.message);
    throw error;
  };

  return (
    <section aria-labelledby={headingId} className="pb-2">
      <div className="flex items-center justify-between gap-2 pt-3 pb-2">
        <h2 id={headingId} className="text-h3">
          {title}
        </h2>
        <Button size="sm" onClick={onNew}>
          <Plus />
          {newLabel}
        </Button>
      </div>
      <SortableList
        items={items.filter((i) => !i.archived)}
        label={(i) => i.name}
        onReorder={(ids) => onReorder(ids).catch(failed)}
      >
        {(item, handle) => (
          <div className="flex min-h-11 items-center gap-2 border-t border-hairline">
            {handle}
            <div className="flex min-w-0 flex-1 items-center gap-2.5">
              {row(item)}
            </div>
            <Menu>
              <MenuTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label={`More actions for ${item.name}`}
                >
                  <Ellipsis />
                </Button>
              </MenuTrigger>
              <MenuContent>{actions(item)}</MenuContent>
            </Menu>
          </div>
        )}
      </SortableList>
      {archived.length > 0 && (
        <>
          <button
            type="button"
            aria-expanded={showArchived}
            onClick={() => setShowArchived(!showArchived)}
            className="flex w-full items-center gap-1.5 border-t border-hairline py-2.5 text-muted-foreground outline-none hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/40"
          >
            <ChevronRight
              aria-hidden
              className={cn(
                "size-3.5 transition-transform duration-150",
                showArchived && "rotate-90",
              )}
            />
            Show archived ({archived.length})
          </button>
          {showArchived &&
            archived.map((item) => (
              <div
                key={item.id}
                className="flex min-h-11 items-center gap-2 border-t border-hairline pl-8 text-muted-foreground"
              >
                <div className="flex min-w-0 flex-1 items-center gap-2.5">
                  {row(item)}
                </div>
                <Button
                  size="sm"
                  aria-label={`Unarchive ${item.name}`}
                  onClick={() =>
                    onUnarchive(item)
                      .then(() => notify.success("Unarchived"))
                      .catch((error: Error) => notify.error(error.message))
                  }
                >
                  Unarchive
                </Button>
              </div>
            ))}
        </>
      )}
    </section>
  );
}
