import type { DueOccurrence } from "@/bindings/DueOccurrence";
import type { RecurringRule } from "@/bindings/RecurringRule";
import type { RecurringRuleFields } from "@/bindings/RecurringRuleFields";
import type { Transaction } from "@/bindings/Transaction";
import type { TransactionFields } from "@/bindings/TransactionFields";
import { call } from "./client";

/** Every rule, oldest first, each with its next date (none once it has ended). */
export const listRecurringRules = () =>
  call<RecurringRule[]>("list_recurring_rules");

/** The template's `date` is the start date (REC-1). */
export const createRecurringRule = (rule: RecurringRuleFields) =>
  call<RecurringRule>("create_recurring_rule", { rule });

/** A new frequency, interval or start date starts again from that start date (REC-5). */
export const updateRecurringRule = ({
  id,
  rule,
}: {
  id: number;
  rule: RecurringRuleFields;
}) => call<RecurringRule>("update_recurring_rule", { id, rule });

/** Resuming skips the dates it was paused for (REC-5). */
export const pauseRecurringRule = ({
  id,
  paused,
}: {
  id: number;
  paused: boolean;
}) => call<RecurringRule>("pause_recurring_rule", { id, paused });

/** Keeps the transactions it created. */
export const deleteRecurringRule = (id: number) =>
  call<null>("delete_recurring_rule", { id });

/** Posts or queues every rule's dates due up to today; how many (REC-4). */
export const processRecurring = () => call<number>("process_recurring");

/** The dates Confirm rules queued, oldest first (UI_FLOWS F10). */
export const listDueOccurrences = () =>
  call<DueOccurrence[]>("list_due_occurrences");

/** Post (`transaction` null: the template on its due date) or Edit & post (REC-3). */
export const postOccurrence = ({
  id,
  transaction,
}: {
  id: number;
  transaction: TransactionFields | null;
}) => call<Transaction>("post_occurrence", { id, transaction });

/** Settles the date with no transaction; it never comes back. */
export const skipOccurrence = (id: number) =>
  call<null>("skip_occurrence", { id });
