import type { SettingsView } from "@/bindings/SettingsView";
import type { Theme } from "@/bindings/Theme";
import { Segmented } from "@/components/ui/segmented";
import { notify } from "@/components/ui/toast";
import { useUpdateSettings } from "@/hooks/useSettings";
import { Row } from "./GeneralTab";

const THEMES: { value: Theme; label: string }[] = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

/**
 * Settings → Appearance (UI_FLOWS §6, SET-5). A change saves at once with a "Saved" toast, and
 * Rust gives the window its theme, which `lib/theme.ts` follows (DESIGN_SYSTEM §3.5).
 */
export function AppearanceTab({ settings }: { settings: SettingsView }) {
  const update = useUpdateSettings();
  return (
    <div className="pb-2">
      <Row id="set-theme" label="Theme" help="System follows Windows">
        <Segmented
          aria-label="Theme"
          value={settings.theme}
          // The promise, not mutate's callbacks, so the toast still shows if the tab is left.
          onChange={(theme) =>
            void update.mutateAsync({ theme }).then(
              () => notify.success("Saved"),
              (error: Error) => notify.error(error.message),
            )
          }
          options={THEMES}
        />
      </Row>
    </div>
  );
}
