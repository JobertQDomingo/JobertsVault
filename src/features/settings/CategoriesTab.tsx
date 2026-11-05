import { useState } from "react";
import type { Category } from "@/bindings/Category";
import type { CategoryKind } from "@/bindings/CategoryKind";
import { CategoryChip } from "@/components/CategoryChip";
import { ErrorState } from "@/components/ErrorState";
import { Loading, Skeleton } from "@/components/Loading";
import { Button } from "@/components/ui/button";
import { Combobox } from "@/components/ui/combobox";
import {
  ConfirmDialog,
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
} from "@/components/ui/dialog";
import { Field } from "@/components/ui/input";
import { MenuItem } from "@/components/ui/menu";
import { notify } from "@/components/ui/toast";
import {
  useArchiveCategory,
  useCategories,
  useDeleteCategory,
  useMergeCategories,
  useReorderCategories,
  useUnarchiveCategory,
} from "@/hooks/useCategories";
import { CategoryDialog } from "./CategoryDialog";
import { ManagedList } from "./ManagedList";

const SECTIONS: { kind: CategoryKind; title: string }[] = [
  { kind: "expense", title: "Expense" },
  { kind: "income", title: "Income" },
];

const count = (n: number) => `${n} ${n === 1 ? "transaction" : "transactions"}`;

type Open =
  | { edit: Category }
  | { create: CategoryKind }
  | { merge: Category }
  | { delete: Category };

/**
 * Settings → Categories (UI_FLOWS §6, F7, CAT-2, CAT-4): the expense and income lists, each
 * reordered on its own. Archive, Unarchive, Merge into… and Delete (unused only) are the
 * backend's rules; its refusals are toasts.
 */
export function CategoriesTab() {
  const list = useCategories();
  const reorder = useReorderCategories();
  const archive = useArchiveCategory();
  const unarchive = useUnarchiveCategory();
  const remove = useDeleteCategory();
  const [open, setOpen] = useState<Open | null>(null);
  const failed = (error: Error) => notify.error(error.message);

  if (list.isError)
    return (
      <ErrorState onRetry={() => list.refetch()} pending={list.isFetching} />
    );
  if (!list.data)
    return (
      <Loading className="grid gap-2 py-3">
        {Array.from({ length: 6 }, (_, i) => (
          <Skeleton key={i} className="h-10" />
        ))}
      </Loading>
    );
  const { categories, defaultColors } = list.data;

  return (
    <>
      {SECTIONS.map(({ kind, title }) => (
        <ManagedList
          key={kind}
          title={title}
          items={categories.filter((c) => c.kind === kind)}
          newLabel="New category"
          onNew={() => setOpen({ create: kind })}
          onReorder={(ids) => reorder.mutateAsync(ids)}
          onUnarchive={(c) => unarchive.mutateAsync(c.id)}
          row={(c) => (
            <>
              <CategoryChip color={c.color} icon={c.icon} size="sm" />
              <span className="truncate">{c.name}</span>
              <span className="text-small whitespace-nowrap text-muted-foreground">
                {count(c.transactionCount)}
              </span>
            </>
          )}
          actions={(c) => (
            <>
              <MenuItem onSelect={() => setOpen({ edit: c })}>Edit</MenuItem>
              <MenuItem onSelect={() => setOpen({ merge: c })}>
                Merge into…
              </MenuItem>
              <MenuItem
                onSelect={() =>
                  archive.mutate(c.id, {
                    onSuccess: () => notify.success("Archived"),
                    onError: failed,
                  })
                }
              >
                Archive
              </MenuItem>
              {c.transactionCount === 0 && (
                <MenuItem destructive onSelect={() => setOpen({ delete: c })}>
                  Delete
                </MenuItem>
              )}
            </>
          )}
        />
      ))}
      {open && "create" in open && (
        <CategoryDialog
          kind={open.create}
          defaultColor={defaultColors[open.create]}
          onClose={() => setOpen(null)}
        />
      )}
      {open && "edit" in open && (
        <CategoryDialog
          category={open.edit}
          kind={open.edit.kind}
          defaultColor={open.edit.color}
          onClose={() => setOpen(null)}
        />
      )}
      {open && "merge" in open && (
        <MergeDialog
          from={open.merge}
          categories={categories}
          onClose={() => setOpen(null)}
        />
      )}
      <ConfirmDialog
        open={open !== null && "delete" in open}
        onOpenChange={(o) => !o && setOpen(null)}
        title={`Delete ${open && "delete" in open ? open.delete.name : ""}?`}
        message="This can't be undone."
        confirmLabel="Delete"
        destructive
        onConfirm={() =>
          open &&
          "delete" in open &&
          remove.mutate(open.delete.id, {
            onSuccess: () => notify.success("Deleted"),
            onError: failed,
          })
        }
      />
    </>
  );
}

/** Merge into… (F7): pick an active category of the same kind, then confirm the move (CAT-4). */
function MergeDialog({
  from,
  categories,
  onClose,
}: {
  from: Category;
  categories: Category[];
  onClose: () => void;
}) {
  const merge = useMergeCategories();
  const [into, setInto] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const target = categories.find((c) => String(c.id) === into);
  const options = categories
    .filter((c) => c.kind === from.kind && !c.archived && c.id !== from.id)
    .map((c) => ({
      value: String(c.id),
      label: c.name,
      icon: <CategoryChip color={c.color} icon={c.icon} size="sm" />,
    }));

  return (
    <>
      <Dialog open={!confirming} onOpenChange={(o) => !o && onClose()}>
        <DialogContent title={`Merge ${from.name}`}>
          <form
            noValidate
            onSubmit={(e) => {
              e.preventDefault();
              if (target) setConfirming(true);
            }}
          >
            <DialogBody>
              <Field
                label="Into"
                htmlFor="merge-into"
                help={`Its ${count(from.transactionCount)} move there, and '${from.name}' is deleted.`}
              >
                <Combobox
                  id="merge-into"
                  options={options}
                  value={into}
                  onChange={setInto}
                  placeholder="Choose a category"
                />
              </Field>
            </DialogBody>
            <DialogFooter>
              <Button onClick={onClose}>Cancel</Button>
              <Button variant="primary" type="submit" disabled={!target}>
                Merge
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
      <ConfirmDialog
        open={confirming}
        onOpenChange={(o) => !o && onClose()}
        title="Merge categories?"
        message={`Move ${count(from.transactionCount)} from '${from.name}' to '${target?.name}' and delete '${from.name}'?`}
        confirmLabel="Merge"
        destructive
        onConfirm={() =>
          target &&
          merge.mutate(
            { fromId: from.id, intoId: target.id },
            {
              onSuccess: () => notify.success("Merged"),
              onError: (error) => notify.error(error.message),
            },
          )
        }
      />
    </>
  );
}
