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
import { useRef, useState, type ReactNode } from "react";
import { cn } from "@/lib/utils";

/**
 * A list reordered by dragging its ⋮⋮ handles, or by focusing one and pressing ↑/↓ (UI_FLOWS §5.1).
 * `onReorder` gets every id in the new order. The new order shows straight away and stays until
 * every save has finished. Saves run one at a time, and moves made during one go as a single save
 * after it, so a held key or quick presses never race or jump back. Once all are done, `items`
 * (the refreshed order, or the old one after a failure) shows again.
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
  const [moved, setMoved] = useState<number[] | null>(null);
  const queued = useRef<number[] | null>(null);
  const saving = useRef(false);
  const ids = moved ?? items.map((item) => item.id);
  const byId = new Map(items.map((item) => [item.id, item]));
  const sensors = useSensors(
    // A few pixels first, so a click on the handle isn't a drag.
    useSensor(PointerSensor, { activationConstraint: { distance: 4 } }),
  );

  const save = async () => {
    saving.current = true;
    for (let next = queued.current; next; next = queued.current) {
      queued.current = null;
      // The caller shows a failure; the next queued order still goes.
      await onReorder(next).catch(() => {});
    }
    saving.current = false;
    setMoved(null);
  };

  const move = (from: number, to: number) => {
    if (to < 0 || to >= ids.length || from === to) return;
    const next = arrayMove(ids, from, to);
    setMoved(next);
    queued.current = next;
    if (!saving.current) void save();
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
  // dnd-kit's `attributes` aren't spread: they announce its Space-to-pick-up keys, which this
  // handle doesn't use (↑/↓ move it).
  const {
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
