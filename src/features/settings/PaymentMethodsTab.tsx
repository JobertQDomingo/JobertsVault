import { useRef, useState } from "react";
import type { PaymentMethod } from "@/bindings/PaymentMethod";
import { ErrorState } from "@/components/ErrorState";
import { Loading, Skeleton } from "@/components/Loading";
import { Button } from "@/components/ui/button";
import {
  ConfirmDialog,
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
} from "@/components/ui/dialog";
import { Field, Input } from "@/components/ui/input";
import { MenuItem } from "@/components/ui/menu";
import { notify } from "@/components/ui/toast";
import {
  useArchivePaymentMethod,
  useCreatePaymentMethod,
  useDeletePaymentMethod,
  usePaymentMethods,
  useReorderPaymentMethods,
  useUnarchivePaymentMethod,
  useUpdatePaymentMethod,
} from "@/hooks/usePaymentMethods";
import { showSaveError } from "@/lib/saveError";
import { ManagedList } from "./ManagedList";

/** Settings → Payment methods (UI_FLOWS §6, TXN-7): add, rename, reorder, archive, delete unused. */
export function PaymentMethodsTab() {
  const list = usePaymentMethods();
  const reorder = useReorderPaymentMethods();
  const archive = useArchivePaymentMethod();
  const unarchive = useUnarchivePaymentMethod();
  const remove = useDeletePaymentMethod();
  const [editing, setEditing] = useState<PaymentMethod | "new" | null>(null);
  const [deleting, setDeleting] = useState<PaymentMethod | null>(null);
  const failed = (error: Error) => notify.error(error.message);

  if (list.isError)
    return (
      <ErrorState onRetry={() => list.refetch()} pending={list.isFetching} />
    );
  if (!list.data)
    return (
      <Loading className="grid gap-2 py-3">
        {Array.from({ length: 5 }, (_, i) => (
          <Skeleton key={i} className="h-10" />
        ))}
      </Loading>
    );

  return (
    <>
      <ManagedList
        title="Payment methods"
        items={list.data}
        newLabel="New method"
        onNew={() => setEditing("new")}
        onReorder={(ids) => reorder.mutateAsync(ids)}
        onUnarchive={(m) => unarchive.mutateAsync(m.id)}
        row={(m) => <span className="truncate">{m.name}</span>}
        actions={(m) => (
          <>
            <MenuItem onSelect={() => setEditing(m)}>Rename</MenuItem>
            <MenuItem
              onSelect={() =>
                archive.mutate(m.id, {
                  onSuccess: () => notify.success("Archived"),
                  onError: failed,
                })
              }
            >
              Archive
            </MenuItem>
            {!m.inUse && (
              <MenuItem destructive onSelect={() => setDeleting(m)}>
                Delete
              </MenuItem>
            )}
          </>
        )}
      />
      {editing && (
        <MethodDialog
          method={editing === "new" ? undefined : editing}
          onClose={() => setEditing(null)}
        />
      )}
      <ConfirmDialog
        open={deleting !== null}
        onOpenChange={(o) => !o && setDeleting(null)}
        title={`Delete ${deleting?.name}?`}
        message="This can't be undone."
        confirmLabel="Delete"
        destructive
        onConfirm={() =>
          deleting &&
          remove.mutate(deleting.id, {
            onSuccess: () => notify.success("Deleted"),
            onError: failed,
          })
        }
      />
    </>
  );
}

function MethodDialog({
  method,
  onClose,
}: {
  method?: PaymentMethod;
  onClose: () => void;
}) {
  const [name, setName] = useState(method?.name ?? "");
  const [error, setError] = useState<string>();
  const create = useCreatePaymentMethod();
  const update = useUpdatePaymentMethod();
  const saving = useRef(false);

  const save = async () => {
    if (saving.current) return;
    saving.current = true;
    try {
      if (method) await update.mutateAsync({ id: method.id, name });
      else await create.mutateAsync(name);
      notify.success("Saved");
      onClose();
    } catch (e) {
      showSaveError(e, ["name"] as const, (_f, { message }) =>
        setError(message),
      );
    } finally {
      saving.current = false;
    }
  };

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent
        title={method ? "Rename payment method" : "New payment method"}
        size="confirm"
      >
        <form
          noValidate
          onSubmit={(e) => {
            e.preventDefault();
            void save();
          }}
        >
          <DialogBody>
            <Field label="Name" htmlFor="method-name" error={error}>
              <Input
                id="method-name"
                autoComplete="off"
                autoFocus
                value={name}
                onChange={(e) => {
                  setName(e.target.value);
                  setError(undefined);
                }}
              />
            </Field>
          </DialogBody>
          <DialogFooter>
            <Button onClick={onClose}>Cancel</Button>
            <Button variant="primary" type="submit">
              {method ? "Save" : "Create method"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
