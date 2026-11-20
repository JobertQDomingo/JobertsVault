import { useQueryClient } from "@tanstack/react-query";
import { Fragment, useEffect, useEffectEvent, useRef, useState } from "react";
import { useOutletContext, useSearchParams } from "react-router";
import logo from "../../../assets/brand/logo.svg";
import type { Checkin } from "@/bindings/Checkin";
import type { Offer } from "@/bindings/Offer";
import type { SettingsView } from "@/bindings/SettingsView";
import type { GuardContext } from "@/components/AppShell";
import { ErrorState } from "@/components/ErrorState";
import { GlassPanel } from "@/components/GlassPanel";
import { Loading, Skeleton } from "@/components/Loading";
import { TopBar } from "@/components/TopBar";
import { Button } from "@/components/ui/button";
import { notify } from "@/components/ui/toast";
import { useAccounts } from "@/hooks/useAccounts";
import { useCategories } from "@/hooks/useCategories";
import { useCheckin, useCompleteCheckin } from "@/hooks/useCheckin";
import { useLoans } from "@/hooks/useLoans";
import { useSettings } from "@/hooks/useSettings";
import { useShortcuts } from "@/hooks/useShortcuts";
import { formatPeriodLabel, formatSpan } from "@/lib/dates";
import { queryKeys } from "@/lib/queryKeys";
import { cn } from "@/lib/utils";
import {
  balanceStep,
  cardStep,
  focusStep,
  goalsStep,
  loansStep,
  moneyStep,
  start,
  statsStep,
  unitOf,
  yearStep,
  type Answer,
  type Ctx,
  type Step,
} from "./steps";

// Every step, in order (CHK-3). Goals, Balance check and Your year show when the backend sends them.
const STEPS = {
  start: { label: "Start", build: start },
  money: { label: "Money in and out", build: moneyStep },
  card: { label: "Card", build: cardStep },
  loans: { label: "Loans and bills", build: loansStep },
  goals: { label: "Goals", build: goalsStep },
  balance: { label: "Balance check", build: balanceStep },
  year: { label: "Your year", build: yearStep },
  stats: { label: "Stats and level", build: statsStep },
  focus: { label: "Pick a focus", build: focusStep },
} satisfies Record<string, { label: string; build: (ctx: Ctx) => Step }>;
type Name = keyof typeof STEPS;
const ORDER = Object.keys(STEPS) as Name[];

/** An offer's identity: the 7 fields Finish sends back, which must match exactly (§4.5). */
const keyOf = (o: Offer) =>
  JSON.stringify([
    o.kind,
    o.categoryId,
    o.accountId,
    o.loanPaymentId,
    o.target,
    o.start,
    o.end,
  ]);

/**
 * `/check-in` (UI_FLOWS §8.7, F17): the required review, full-window with no sidebar and no way
 * to close it. It shows what `get_checkin` returns; the names come from the usual lists.
 */
export function CheckinScreen() {
  const checkin = useCheckin();
  const categories = useCategories();
  const accounts = useAccounts();
  const loans = useLoans();
  const settings = useSettings();
  const data = checkin.data;
  const failed = [categories, accounts, loans, settings].filter(
    (q) => q.isError,
  );
  const title = data
    ? unitOf(data) === "month"
      ? `Monthly check-in · ${formatPeriodLabel("month", data.period.start, data.period.end)}`
      : `Weekly check-in · ${formatSpan(data.period.start, data.period.end, data.today)}`
    : "Check-in";

  return (
    <div className="flex h-full flex-col gap-3 p-3">
      <TopBar
        title={
          <>
            <img src={logo} alt="" className="size-[30px]" />
            {title}
          </>
        }
      />
      <GlassPanel className="mx-auto flex min-h-0 w-[880px] flex-1 flex-col gap-5 p-6">
        {failed.length ? (
          <ErrorState
            className="flex-1 content-center"
            onRetry={() => failed.forEach((q) => q.refetch())}
            pending={failed.some((q) => q.isFetching)}
          />
        ) : data &&
          categories.data &&
          accounts.data &&
          loans.data &&
          settings.data ? (
          <Flow
            data={data}
            refetch={() => void checkin.refetch()}
            base={{
              settings: settings.data,
              categories: categories.data.categories,
              accounts: accounts.data.accounts,
              loans: loans.data.loans,
            }}
          />
        ) : (
          <Loading className="grid gap-5">
            <Skeleton className="h-4 w-64" />
            <Skeleton className="h-8 w-[480px]" />
            <Skeleton className="h-40" />
          </Loading>
        )}
      </GlassPanel>
    </div>
  );
}

function Flow({
  data,
  base,
  refetch,
}: {
  data: Checkin;
  base: Pick<Ctx, "categories" | "accounts" | "loans"> & {
    settings: SettingsView;
  };
  refetch: () => void;
}) {
  const [params, setParams] = useSearchParams();
  const client = useQueryClient();
  const guard = useOutletContext<Partial<GuardContext>>();
  const [answers, setAnswers] = useState<Record<number, Answer>>({});
  const [pickKey, setPickKey] = useState<string | null>(null);

  const steps = ORDER.filter((s) =>
    s === "goals"
      ? data.goals
      : s === "balance"
        ? data.balances
        : s === "year"
          ? data.year
          : true,
  );
  const answered = (data.balances ?? []).every((b) => answers[b.accountId]);
  // The step is kept by name, so one that appears or goes after a save doesn't move the user,
  // except past a Balance check not yet answered, which can appear when a month comes due (CHK-4).
  const asked = ORDER.indexOf(params.get("step") as Name);
  const found = steps.find((s) => ORDER.indexOf(s) >= asked) ?? steps[0];
  const step =
    !answered && ORDER.indexOf(found) > ORDER.indexOf("balance")
      ? "balance"
      : found;
  const at = steps.indexOf(step);
  // A pick that's no longer offered (after a refused Finish reloads them) is no pick (ADR 0014).
  const picked = data.offers.find((o) => keyOf(o) === pickKey);
  // The level-up moment is decided as Stats and level opens, so a refresh on it can't start or cut
  // it, and leaving the step once ends it, so Back and Next don't replay it (LVL-6).
  const rose = data.status.levelChange === "up";
  const [shown, setShown] = useState(step);
  const [levelUp, setLevelUp] = useState(step === "stats" && rose);
  const [played, setPlayed] = useState(false);
  if (shown !== step) {
    setShown(step);
    setPlayed(played || shown === "stats");
    setLevelUp(step === "stats" && rose && !played);
  }

  const ctx: Ctx = {
    data,
    ...base,
    answers,
    answer: (id, value) => setAnswers((was) => ({ ...was, [id]: value })),
    picked,
    pick: (o) => setPickKey(keyOf(o)),
    levelUp,
  };
  const { say, body } = STEPS[step].build(ctx);

  // A failed Finish's toast, closed when Finish is tried again, so its way out can't outlive it.
  const failure = useRef<string | number>(undefined);
  const complete = useCompleteCheckin(() => {
    const off = data.balances?.find((b) => answers[b.accountId] === "mismatch");
    // None at once, before the refresh, and the guard moves on to where Finish goes (F17).
    guard.leaveTo?.(off ? `/accounts/${off.accountId}` : "/");
    client.setQueryData(queryKeys.checkin, null);
    notify.success(
      off
        ? "Check-in done. Check this account against your real balance."
        : "Check-in done",
    );
  });
  const last = step === "focus";
  const ready =
    step === "balance"
      ? answered
      : last
        ? !!picked && !complete.isPending
        : true;

  const go = (s: Name) => setParams({ step: s }, { replace: true });
  const back = () => at > 0 && go(steps[at - 1]);
  const next = () => {
    if (!ready) return;
    if (!last) return go(steps[at + 1]);
    notify.dismiss(failure.current);
    complete.mutate(picked!, {
      onError: (error) => {
        refetch();
        failure.current = notify.error(
          error.message,
          guard.openAnyway && {
            label: "Open the app anyway",
            onClick: guard.openAnyway,
          },
        );
      },
    });
  };

  // Nothing closes it: Esc and Ctrl+1 … Ctrl+4 do nothing here (CHK-2). Alt+← is Back.
  useShortcuts({
    "Alt+ArrowLeft": back,
    "Alt+ArrowRight": () => {},
    "Alt+Home": () => {},
    // There's no search here: not WebView2's find bar either.
    "Ctrl+F": () => {},
  });
  // Enter presses Next, unless it's on a button or field of its own, or a dialog is open.
  const onEnter = useEffectEvent(next);
  useEffect(() => {
    const listener = (e: KeyboardEvent) => {
      // A held Enter moves one step, so no step goes by unread.
      if (
        e.key !== "Enter" ||
        e.repeat ||
        e.ctrlKey ||
        e.altKey ||
        e.shiftKey ||
        e.metaKey
      )
        return;
      const busy =
        "button,a,input,textarea,select,[role=dialog],[role=alertdialog],[role=menu]";
      if ((e.target as Element | null)?.closest?.(busy)) return;
      if (
        document.querySelector("[role=dialog],[role=alertdialog],[role=menu]")
      )
        return;
      e.preventDefault();
      onEnter();
    };
    window.addEventListener("keydown", listener);
    return () => window.removeEventListener("keydown", listener);
  }, []);

  return (
    <>
      <StepIndicator at={at} of={steps.length} name={STEPS[step].label} />
      <h2 className="text-h1">{say}</h2>
      {/* Keyed by step, so each one opens scrolled to its top. */}
      <div
        key={step}
        className="-m-1 grid min-h-0 flex-1 content-start gap-3 overflow-y-auto p-1"
      >
        {body}
      </div>
      <div className="flex justify-between">
        {at > 0 ? <Button onClick={back}>Back</Button> : <span />}
        <Button variant="primary" onClick={next} disabled={!ready}>
          {last ? (
            "Finish"
          ) : (
            <>
              Next
              <span
                aria-hidden
                className="ml-1 text-[11px] font-normal opacity-80"
              >
                Enter
              </span>
            </>
          )}
        </Button>
      </div>
    </>
  );
}

/** Dots for the steps done, the current one and those ahead, with the words beside them (§9.2). */
function StepIndicator({
  at,
  of,
  name,
}: {
  at: number;
  of: number;
  name: string;
}) {
  const edge = "shadow-[inset_0_0_0_1px_var(--hairline)]";
  return (
    <div className="flex items-center gap-3">
      <span aria-hidden className="flex items-center">
        {Array.from({ length: of }, (_, i) => (
          <Fragment key={i}>
            {i > 0 && <i className={cn("h-0.5 w-6 bg-wash", edge)} />}
            <i
              className={cn(
                "rounded-full",
                i < at && "size-2.5 bg-primary",
                i === at &&
                  "size-3.5 border-2 border-primary bg-[radial-gradient(circle,var(--primary)_3px,transparent_3.5px)]",
                i > at && cn("size-2.5 bg-wash", edge),
              )}
            />
          </Fragment>
        ))}
      </span>
      <span className="text-small">{`Step ${at + 1} of ${of} · ${name}`}</span>
    </div>
  );
}
