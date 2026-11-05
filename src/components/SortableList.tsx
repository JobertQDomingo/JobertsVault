import {
  closestCenter,
  DndContext,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  arrayMove,
  SortableContext,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { GripVertical } from "lucide-react";
import { useState, type ReactNode } from "react";
import { cn } from "@/lib/utils";

/**
 * A list reordered by dragging its ⋮⋮ handles, or by focusing one and pressing ↑/↓ (UI_FLOWS §5.1).
 * `onReorder` gets every id in the new order. The new order shows straight away and stays until
 * the ids in `items` change (the save's refresh), so it doesn't flash back while saving.
 */
export function SortableList<T extends { id: number }>({
  items,
  label,
  onReorder,
  children,
}: {
  items: T[];
  label: (item: T) => string;
  onReorder: (ids: number[]) => Promise<unknown>;
  children: (item: T, handle: ReactNode) => ReactNode;
}) {
  // Keyed by the ids, since callers filter the list afresh on every render.
  const key = items.map((item) => item.id).join();
  const [moved, setMoved] = useState<{ from: string; ids: number[] } | null>(
    null,
  );
  const ids = moved?.from === key ? moved.ids : items.map((item) => item.id);
  const byId = new Map(items.map((item) => [item.id, item]));
  const sensors = useSensors(
    // A few pixels first, so a click on the handle isn't a drag.
    useSensor(PointerSensor, { activationConstraint: { distance: 4 } }),
  );

  const move = (from: number, to: number) => {
    if (to < 0 || to >= ids.length || from === to) return;
    const next = arrayMove(ids, from, to);
    setMoved({ from: key, ids: next });
    // A failed save puts the old order back (the caller shows the error).
    onReorder(next).catch(() => setMoved(null));
  };

  const onDragEnd = ({ active, over }: DragEndEvent) => {
    if (over)
      move(ids.indexOf(Number(active.id)), ids.indexOf(Number(over.id)));
  };

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      onDragEnd={onDragEnd}
    >
      <SortableContext items={ids} strategy={verticalListSortingStrategy}>
        {ids.map((id, index) => {
          const item = byId.get(id);
          return item ? (
            <SortableItem
              key={id}
              id={id}
              label={label(item)}
              onMove={(by) => move(index, index + by)}
            >
              {(handle) => children(item, handle)}
            </SortableItem>
          ) : null;
        })}
      </SortableContext>
    </DndContext>
  );
}

function SortableItem({
  id,
  label,
  onMove,
  children,
}: {
  id: number;
  label: string;
  onMove: (by: number) => void;
  children: (handle: ReactNode) => ReactNode;
}) {
  const {
    attributes,
    listeners,
    setNodeRef,
    setActivatorNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id });
  const handle = (
    <button
      type="button"
      ref={setActivatorNodeRef}
      {...attributes}
      {...listeners}
      aria-label={`Move ${label}`}
      aria-keyshortcuts="ArrowUp ArrowDown"
      title="Drag, or press ↑ / ↓"
      // ↑/↓ move it at once; dnd-kit's own keyboard mode (Space first) isn't used.
      onKeyDown={(e) => {
        const by = e.key === "ArrowUp" ? -1 : e.key === "ArrowDown" ? 1 : 0;
        if (!by) return;
        e.preventDefault();
        const button = e.currentTarget;
        onMove(by);
        // The row moves in the DOM; keep the cursor on its handle.
        requestAnimationFrame(() => button.focus());
      }}
      className="grid size-6 cursor-grab touch-none place-items-center rounded-[6px] text-muted-foreground outline-none hover:bg-wash focus-visible:ring-3 focus-visible:ring-ring/40 active:cursor-grabbing"
    >
      <GripVertical aria-hidden className="size-3.5" />
    </button>
  );
  return (
    <div
      ref={setNodeRef}
      style={{ transform: CSS.Translate.toString(transform), transition }}
      className={cn("relative", isDragging && "z-10 opacity-80")}
    >
      {children(handle)}
    </div>
  );
}
