import { useRef, useState, type ReactNode } from "react";
import type { Settings } from "@/bindings/Settings";
import type { SettingsView } from "@/bindings/SettingsView";
import { Combobox } from "@/components/ui/combobox";
import { Input } from "@/components/ui/input";
import { Segmented } from "@/components/ui/segmented";
import { Switch } from "@/components/ui/switch";
import { notify } from "@/components/ui/toast";
import { useUpdateSettings } from "@/hooks/useSettings";
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
];
const DAYS = Array.from({ length: 28 }, (_, i) => ({
  value: String(i + 1),
  label: String(i + 1),
}));

/** A row of the mockup: a 220 px label, the control, and help or the backend's message beside it. */
function Row({
  id,
  label,
  error,
  help,
  children,
}: {
  id: string;
  label: string;
  error?: string;
  help?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="grid grid-cols-[220px_minmax(0,1fr)] items-center gap-4 border-t border-hairline py-3.5 first:border-t-0">
      <label htmlFor={id} id={`${id}-label`} className="font-semibold">
        {label}
      </label>
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

/**
 * Settings → General (UI_FLOWS §6, SET-1, SET-2, SET-6). Each change saves at once with a "Saved"
 * toast; the backend checks it, and its message shows beside the setting.
 */
export function GeneralTab({ settings }: { settings: SettingsView }) {
  const update = useUpdateSettings();
  const [errors, setErrors] = useState<Partial<Record<Key, string>>>({});
  const [symbol, setSymbol] = useState(settings.currencySymbol);
  // Changes not yet stored. Each save sends the whole set, so a quick second change must carry
  // the first even before it's back.
  const draft = useRef<Partial<Settings>>({});

  const save = (change: Partial<Settings>) => {
    const keys = Object.keys(change) as Key[];
    draft.current = { ...draft.current, ...change };
    const { decimalsLocked, ...stored } = settings;
    update
      .mutateAsync({ ...stored, ...draft.current })
      .then(() => {
        notify.success("Saved");
        setErrors((e) => ({
          ...e,
          ...Object.fromEntries(keys.map((k) => [k, undefined])),
        }));
      })
      .catch((error: unknown) => {
        // Refused: the stored value stays, so later saves don't send this one again.
        for (const k of keys) delete draft.current[k];
        showSaveError(error, SHOWN, (field, { message }) =>
          setErrors((e) => ({ ...e, [field]: message })),
        );
      });
  };
  const saveSymbol = () => {
    if (symbol !== settings.currencySymbol || errors.currencySymbol)
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
          onKeyDown={(e) => e.key === "Enter" && saveSymbol()}
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
        error={errors.monthStartDay}
        help="1 = calendar month. Pick 25 for a payday cycle (25th to 24th)."
      >
        <Combobox
          id="set-month"
          className="w-[120px]"
          options={DAYS}
          value={String(settings.monthStartDay)}
          onChange={(v) => save({ monthStartDay: Number(v) })}
        />
      </Row>
      <Row
        id="set-method"
        label="Payment method field"
        error={errors.showPaymentMethod}
        help="Shown in the transaction form"
      >
        <Switch
          id="set-method"
          checked={settings.showPaymentMethod}
          onCheckedChange={(showPaymentMethod) => save({ showPaymentMethod })}
        />
      </Row>
    </div>
  );
}
