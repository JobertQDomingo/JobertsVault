import type { Account } from "@/bindings/Account";
import type { Category } from "@/bindings/Category";
import type { DueOccurrence } from "@/bindings/DueOccurrence";
import { CategoryChip, TransferChip } from "@/components/CategoryChip";
import { ErrorState } from "@/components/ErrorState";
import { Loading, Skeleton } from "@/components/Loading";
import { MoneyText } from "@/components/MoneyText";
import { useQuickAdd } from "@/components/QuickAddProvider";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { notify } from "@/components/ui/toast";
import { useAccounts } from "@/hooks/useAccounts";
import { useCategories } from "@/hooks/useCategories";
import {
  useDueOccurrences,
  usePostOccurrence,
  useSkipOccurrence,
} from "@/hooks/useRecurring";
import { formatDayHeader } from "@/lib/dates";

/**
 * Review due items (UI_FLOWS F10): every date a Confirm rule queued, oldest first, each with
 * Post, Edit & post or Skip (REC-3). Edit & post opens the transaction form over it.
 */
export function DueDialog({
  today,
  onClose,
}: {
  today: string;
  onClose: () => void;
}) {
  const due = useDueOccurrences();
  const accounts = useAccounts();
  const categories = useCategories();
  const failed = [due, accounts, categories].filter((q) => q.isError);
  const post = usePostOccurrence();
  const skip = useSkipOccurrence();
  // One at a time, so a second click can't settle a date twice.
  const busy = post.isPending || skip.isPending;
  const { post: editAndPost } = useQuickAdd();
  const done = (message: string) => ({
    onSuccess: () => notify.success(message),
    onError: (error: Error) => notify.error(error.message),
  });

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent
        title="Recurring transactions due"
        className="max-w-[640px]"
      >
        {failed.length ? (
          <ErrorState
            onRetry={() => failed.forEach((q) => q.refetch())}
            pending={failed.some((q) => q.isFetching)}
          />
        ) : due.data && accounts.data && categories.data ? (
          <ul className="pt-1 pb-3">
            {due.data.map((o) => (
              <DueRow
                key={o.id}
                due={o}
                today={today}
                accounts={accounts.data.accounts}
                categories={categories.data.categories}
                busy={busy}
                onPost={() =>
                  post.mutate({ id: o.id, transaction: null }, done("Posted"))
                }
                onEdit={() => editAndPost(o)}
                onSkip={() => skip.mutate(o.id, done("Skipped"))}
              />
            ))}
          </ul>
        ) : (
          <Loading className="grid gap-2 px-[22px] py-3">
            {[0, 1].map((i) => (
              <Skeleton key={i} className="h-11" />
            ))}
          </Loading>
        )}
      </DialogContent>
    </Dialog>
  );
}

function DueRow({
  due,
  today,
  accounts,
  categories,
  busy,
  onPost,
  onEdit,
  onSkip,
}: {
  due: DueOccurrence;
  today: string;
  accounts: Account[];
  categories: Category[];
  busy: boolean;
  onPost: () => void;
  onEdit: () => void;
  onSkip: () => void;
}) {
  const t = due.transaction;
  const name = (id: number | null) => accounts.find((a) => a.id === id)?.name;
  const category = categories.find((c) => c.id === t.categoryId);
  const transfer = t.kind === "transfer";
  const title = transfer
    ? `${name(t.accountId) ?? ""} → ${name(t.toAccountId) ?? ""}`
    : [category?.name, t.payee].filter(Boolean).join(" · ");
  const day = formatDayHeader(t.date, today);
  // "Due today", but "Due Sat, Oct 24".
  const when = day === "Today" || day === "Yesterday" ? day.toLowerCase() : day;
  const caption = transfer
    ? `Due ${when}`
    : `Due ${when} · ${name(t.accountId) ?? ""}`;
  return (
    <li className="grid grid-cols-[32px_minmax(0,1fr)_110px_auto] items-center gap-3 border-t border-hairline px-[22px] py-2.5 first:border-t-0">
      {transfer ? (
        <TransferChip />
      ) : (
        <CategoryChip
          color={category?.color ?? "cat-11"}
          icon={category?.icon ?? "ellipsis"}
        />
      )}
      <div className="min-w-0">
        <p className="truncate font-semibold">{title}</p>
        <p className="truncate text-small text-muted-foreground">{caption}</p>
      </div>
      <MoneyText amount={t.amount} kind={t.kind} className="justify-self-end" />
      <div className="flex gap-1">
        <Button variant="primary" size="sm" disabled={busy} onClick={onPost}>
          Post
        </Button>
        <Button variant="ghost" size="sm" disabled={busy} onClick={onEdit}>
          Edit &amp; post
        </Button>
        <Button variant="ghost" size="sm" disabled={busy} onClick={onSkip}>
          Skip
        </Button>
      </div>
    </li>
  );
}
