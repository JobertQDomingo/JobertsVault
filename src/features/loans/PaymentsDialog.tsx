import { Ellipsis, TriangleAlert } from "lucide-react";
import { useState } from "react";
import type { Category } from "@/bindings/Category";
import type { Loan } from "@/bindings/Loan";
import type { LoanPayment } from "@/bindings/LoanPayment";
import { CategoryChip } from "@/components/CategoryChip";
import { useQuickAdd } from "@/components/QuickAddProvider";
import { Button } from "@/components/ui/button";
import { ConfirmDialog, Dialog, DialogContent } from "@/components/ui/dialog";
import { Menu, MenuContent, MenuItem, MenuTrigger } from "@/components/ui/menu";
import { notify } from "@/components/ui/toast";
import { useMarkLoanPaymentUnpaid } from "@/hooks/useLoans";
import { formatLongDate, formatShortDate } from "@/lib/dates";
import { formatMoney, type MoneySettings } from "@/lib/money";

/**
 * A loan's payments (UI_FLOWS §8.5), oldest first: Mark paid on an unpaid one opens the
 * transaction form (F13), and a paid one's ⋯ has Mark unpaid. `loan` is the live list's, so a
 * payment marked paid updates here.
 */
export function PaymentsDialog({
  loan,
  categories,
  settings,
  onEdit,
  onClose,
}: {
  loan: Loan;
  categories: Category[];
  settings: MoneySettings;
  onEdit: () => void;
  onClose: () => void;
}) {
  const { markPaid } = useQuickAdd();
  const unpay = useMarkLoanPaymentUnpaid();
  const [unpaying, setUnpaying] = useState<LoanPayment | null>(null);
  const category = categories.find((c) => c.id === loan.categoryId);

  return (
    <>
      <Dialog open onOpenChange={(open) => !open && onClose()}>
        <DialogContent title={loan.name} size="list">
          {/* An archived loan's card offers only Unarchive, so this does too. */}
          {!loan.archived && (
            <Button
              size="sm"
              className="absolute top-4 right-14"
              onClick={onEdit}
            >
              Edit
            </Button>
          )}
          {/* A long list scrolls inside the dialog, which stays on screen. */}
          <ul className="max-h-[60vh] overflow-y-auto pt-1 pb-3">
            {loan.payments.map((p) => (
              <li
                key={p.id}
                className="grid grid-cols-[32px_minmax(0,1fr)_120px_96px] items-center gap-3 border-t border-hairline px-[22px] py-2.5 first:border-t-0"
              >
                <CategoryChip
                  color={category?.color ?? "cat-11"}
                  icon={category?.icon ?? "ellipsis"}
                />
                <div id={`payment-${p.id}`} className="min-w-0">
                  <p className="truncate font-semibold">
                    {formatLongDate(p.dueDate)}
                  </p>
                  <Status payment={p} />
                </div>
                <span className="justify-self-end font-semibold tabular-nums">
                  {formatMoney(p.paidAmount ?? p.amount, settings)}
                </span>
                <div className="justify-self-end">
                  {p.status === "unpaid" ? (
                    <Button
                      size="sm"
                      // The next payment is the one to pay (F13's mockup).
                      variant={loan.next?.id === p.id ? "primary" : "ghost"}
                      aria-describedby={`payment-${p.id}`}
                      onClick={() => markPaid(p.id)}
                    >
                      Mark paid
                    </Button>
                  ) : p.status === "paid" ? (
                    <Menu>
                      <MenuTrigger asChild>
                        <Button
                          variant="ghost"
                          size="icon"
                          aria-label={`More actions for ${formatLongDate(p.dueDate)}`}
                        >
                          <Ellipsis />
                        </Button>
                      </MenuTrigger>
                      <MenuContent>
                        <MenuItem onSelect={() => setUnpaying(p)}>
                          Mark unpaid
                        </MenuItem>
                      </MenuContent>
                    </Menu>
                  ) : null}
                </div>
              </li>
            ))}
          </ul>
        </DialogContent>
      </Dialog>
      <ConfirmDialog
        open={unpaying !== null}
        onOpenChange={(open) => !open && setUnpaying(null)}
        title="Mark this payment unpaid?"
        message="The expense recorded for it is deleted."
        confirmLabel="Mark unpaid"
        destructive
        onConfirm={() =>
          unpaying &&
          unpay.mutate(unpaying.id, {
            onSuccess: () => notify.success("Marked unpaid"),
            onError: (error) => notify.error(error.message),
          })
        }
      />
    </>
  );
}

function Status({ payment: p }: { payment: LoanPayment }) {
  const text = "truncate text-small text-muted-foreground";
  if (p.status === "paidBefore") return <p className={text}>Paid before</p>;
  if (p.status === "paid")
    return (
      <p className={text}>
        {p.paidOn ? `Paid ${formatShortDate(p.paidOn)}` : "Paid"}
      </p>
    );
  if (p.overdue)
    return (
      <p className="flex items-center gap-1 text-small text-danger">
        <TriangleAlert aria-hidden className="size-3.5" />
        Overdue
      </p>
    );
  return <p className={text}>Unpaid</p>;
}
