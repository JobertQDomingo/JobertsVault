import type { Settings } from "@/bindings/Settings";
import type { SettingsView } from "@/bindings/SettingsView";
import type { Theme } from "@/bindings/Theme";
import { Segmented } from "@/components/ui/segmented";
import { Switch } from "@/components/ui/switch";
import { notify } from "@/components/ui/toast";
import { useUpdateSettings } from "@/hooks/useSettings";
import { Row } from "./GeneralTab";

const THEMES: { value: Theme; label: string }[] = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

/**
 * Settings → Appearance (UI_FLOWS §6, SET-5, SET-8). A change saves at once with a "Saved" toast.
 * Rust gives the window its theme, which `lib/theme.ts` follows (DESIGN_SYSTEM §3.5), and
 * `FollowReduceTransparency` applies the switch (§2.4).
 */
export function AppearanceTab({ settings }: { settings: SettingsView }) {
  const update = useUpdateSettings();
  // The promise, not mutate's callbacks, so the toast still shows if the tab is left.
  const save = (change: Partial<Settings>) =>
    void update.mutateAsync(change).then(
      () => notify.success("Saved"),
      (error: Error) => notify.error(error.message),
    );
  return (
    <div className="pb-2">
      <Row id="set-theme" label="Theme" help="System follows Windows">
        <Segmented
          aria-label="Theme"
          value={settings.theme}
          onChange={(theme) => save({ theme })}
          options={THEMES}
        />
      </Row>
      <Row
        id="set-transparency"
        label="Reduce transparency"
        labelable
        help="Solid panels and no blur, even when Windows' transparency effects are on"
      >
        <Switch
          id="set-transparency"
          checked={settings.reduceTransparency}
          onCheckedChange={(reduceTransparency) => save({ reduceTransparency })}
        />
      </Row>
    </div>
  );
}
