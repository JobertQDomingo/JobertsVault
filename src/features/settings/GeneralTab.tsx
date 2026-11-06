import { useRef, useState, type ReactNode } from "react";
import type { Settings } from "@/bindings/Settings";
import type { SettingsView } from "@/bindings/SettingsView";
import { UpdateCheck } from "@/components/UpdateCheck";
import { Combobox } from "@/components/ui/combobox";
import { Input } from "@/components/ui/input";
import { Segmented } from "@/components/ui/segmented";
import { Switch } from "@/components/ui/switch";
import { notify } from "@/components/ui/toast";
import { useAppVersion } from "@/hooks/useAppVersion";
import { useUpdateSettings } from "@/hooks/useSettings";
import { COMPARE } from "@/lib/comparison";
import { formatMoney } from "@/lib/money";
import { showSaveError } from "@/lib/saveError";

type Key = keyof Settings;
const SHOWN: Key[] = [
  "currencySymbol",
  "currencyPosition",
  "currencyDecimals",
  "weekStart",
  "monthStartDay",
  "showPaymentMethod",
  "autoUpdateEnabled",
  "comparisonMode",
];
const DAYS = Array.from({ length: 28 }, (_, i) => ({
  value: String(i + 1),
  label: String(i + 1),
}));

/** A row of the mockup: a 220 px label, the control, and help or the backend's message beside it. */
function Row({
  id,
  label,
  labelable = false,
  error,
  help,
  children,
}: {
  id: string;
  label: string;
  /** The control is an input or a button a <label> can name; a Segmented names itself. */
  labelable?: boolean;
  error?: string;
  help?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="grid grid-cols-[220px_minmax(0,1fr)] items-center gap-4 border-t border-hairline py-3.5 first:border-t-0">
      {labelable ? (
        <label htmlFor={id} className="font-semibold">
          {label}
        </label>
      ) : (
        <span className="font-semibold">{label}</span>
      )}
      <div className="flex flex-wrap items-center gap-3">
        {children}
        {error ? (
          <p
            id={`${id}-message`}
            role="alert"
            className="text-small text-danger"
          >
            {error}
          </p>
        ) : (
          help && <p className="text-small text-muted-foreground">{help}</p>
        )}
      </div>
    </div>
  );
}

/** The installed version. Only asked for where it's shown, not on the first-launch form. */
function AppVersion() {
  const version = useAppVersion();
  return (
    <p className="tabular-nums">
      {version.data ?? (version.isError ? "Unknown" : "…")}
    </p>
  );
}

/**
 * Settings → General (UI_FLOWS §6, SET-1, SET-2, SET-6, UPD-3). Each change saves at once with a "Saved"
 * toast; the backend checks it, and its message shows beside the setting.
 */
export function GeneralTab({
  settings,
  currencyOnly = false,
}: {
  settings: SettingsView;
  /** Only the three currency rows (the first-launch form, ONB-1). */
  currencyOnly?: boolean;
}) {
  const update = useUpdateSettings();
  const [errors, setErrors] = useState<Partial<Record<Key, string>>>({});
  const [symbol, setSymbol] = useState(settings.currencySymbol);
  // The symbol last sent, so Enter and the blur after it (or a second blur) don't send it twice.
  const sent = useRef(settings.currencySymbol);
  const save = (change: Partial<Settings>) => {
    const keys = Object.keys(change) as Key[];
    void (async () => {
      try {
        const saved = await update.mutateAsync(change);
        notify.success("Saved");
        // The backend trims it; show what it kept.
        if ("currencySymbol" in change) {
          sent.current = saved.currencySymbol;
          setSymbol(saved.currencySymbol);
        }
        setErrors((e) => ({
          ...e,
          ...Object.fromEntries(keys.map((k) => [k, undefined])),
        }));
      } catch (error) {
        showSaveError(error, SHOWN, (field, { message }) =>
          setErrors((e) => ({ ...e, [field]: message })),
        );
      }
    })();
  };
  const saveSymbol = () => {
    if (symbol === sent.current) return;
    sent.current = symbol;
    save({ currencySymbol: symbol });
  };
  const example = (position: Settings["currencyPosition"]) =>
    formatMoney(10 ** settings.currencyDecimals, {
      ...settings,
      currencyPosition: position,
    });

  return (
    <div className="pb-2">
      <Row
        id="set-symbol"
        label="Currency symbol"
        labelable
        error={errors.currencySymbol}
      >
        <Input
          id="set-symbol"
          className="w-[120px]"
          autoComplete="off"
          value={symbol}
          aria-invalid={errors.currencySymbol ? true : undefined}
          aria-describedby={
            errors.currencySymbol ? "set-symbol-message" : undefined
          }
          onChange={(e) => setSymbol(e.target.value)}
          onBlur={saveSymbol}
          // Enter leaves the field, and leaving saves it.
          onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()}
        />
      </Row>
      <Row
        id="set-position"
        label="Symbol position"
        error={errors.currencyPosition}
      >
        <Segmented
          aria-label="Symbol position"
          value={settings.currencyPosition}
          onChange={(currencyPosition) => save({ currencyPosition })}
          options={[
            { value: "before", label: `Before · ${example("before")}` },
            { value: "after", label: `After · ${example("after")}` },
          ]}
        />
      </Row>
      <Row
        id="set-decimals"
        label="Decimal places"
        error={errors.currencyDecimals}
        help={
          settings.decimalsLocked &&
          "Locked because amounts are already saved. Changing it would change every stored amount."
        }
      >
        <Segmented
          aria-label="Decimal places"
          value={String(settings.currencyDecimals) as "0" | "2"}
          onChange={(v) => save({ currencyDecimals: v === "0" ? 0 : 2 })}
          options={[
            { value: "0", label: "0" },
            { value: "2", label: "2" },
          ]}
          disabled={settings.decimalsLocked}
        />
      </Row>
      {!currencyOnly && (
        <>
          <Row id="set-week" label="First day of week" error={errors.weekStart}>
            <Segmented
              aria-label="First day of week"
              value={settings.weekStart}
              onChange={(weekStart) => save({ weekStart })}
              options={[
                { value: "monday", label: "Monday" },
                { value: "sunday", label: "Sunday" },
              ]}
            />
          </Row>
          <Row
            id="set-month"
            label="Month starts on day"
            labelable
            error={errors.monthStartDay}
            help="1 = calendar month. Pick 25 for a payday cycle (25th to 24th)."
          >
            <Combobox
              id="set-month"
              className="w-[120px]"
              options={DAYS}
              value={String(settings.monthStartDay)}
              aria-describedby={
                errors.monthStartDay ? "set-month-message" : undefined
              }
              onChange={(v) => save({ monthStartDay: Number(v) })}
            />
          </Row>
          <Row
            id="set-compare"
            label="Compare periods"
            error={errors.comparisonMode}
            help="How a period still in progress is compared with the one before, in Reports"
          >
            <Segmented
              aria-label="Compare periods"
              value={settings.comparisonMode}
              onChange={(comparisonMode) => save({ comparisonMode })}
              options={COMPARE}
            />
          </Row>
          <Row
            id="set-method"
            label="Payment method field"
            labelable
            error={errors.showPaymentMethod}
            help="Shown in the transaction form"
          >
            <Switch
              id="set-method"
              checked={settings.showPaymentMethod}
              onCheckedChange={(showPaymentMethod) =>
                save({ showPaymentMethod })
              }
            />
          </Row>
          <Row id="set-version" label="Version">
            <AppVersion />
            <UpdateCheck />
          </Row>
          <Row
            id="set-auto-update"
            label="Automatic updates"
            labelable
            error={errors.autoUpdateEnabled}
            help="Install updates automatically when the app starts. When off, the app only connects when you click Check for updates."
          >
            <Switch
              id="set-auto-update"
              checked={settings.autoUpdateEnabled}
              onCheckedChange={(autoUpdateEnabled) =>
                save({ autoUpdateEnabled })
              }
            />
          </Row>
        </>
      )}
    </div>
  );
}
